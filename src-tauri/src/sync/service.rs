//! Sinkronisasi: checkpoint per perangkat, retry + backoff, idempotent ingest,
//! deteksi duplikat/tidak dikenal, antrian gagal, last successful sync.

use crate::db::Repo;
use crate::devices::adapter::{backoff_delays, DeviceAdapter};
use crate::devices::hikvision::HikvisionAdapter;
use crate::devices::mock::MockAdapter;
use crate::devices::solution::SolutionAdapter;
use crate::devices::timetech::TimetechAc6Adapter;
use crate::devices::zkteco::ZktecoAdapter;
use crate::errors::{AppError, AppResult};
use crate::timex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub device_id: String,
    pub fetched: i64,
    pub inserted: i64,
    pub duplicates: i64,
    pub unmapped: i64,
    pub failed: i64,
    pub checkpoint: String,
    pub attempts: u32,
}

/// Bangun adapter nyata berdasar nama + config JSON perangkat.
/// `config_json`: `{"port":4370,"machine":1}` (ZK) atau
/// `{"port":8000,"username":"admin","password":"..."}` (Hikvision).
pub fn build_adapter(adapter_name: &str, config_json: &str) -> AppResult<Box<dyn DeviceAdapter>> {
    match adapter_name {
        "zkteco-terminal" => Ok(Box::new(ZktecoAdapter::new(config_json))),
        "solution-terminal" => Ok(Box::new(SolutionAdapter::new(config_json))),
        "hikvision-terminal" => Ok(Box::new(HikvisionAdapter::new(config_json))),
        "timetech-ac6" => Ok(Box::new(TimetechAc6Adapter)),
        // Tambalan/nama belum diatur → tiruan jujur (ditandai SIMULATED).
        a if a.starts_with("mock") || a.is_empty() => {
            Ok(Box::new(MockAdapter::new_simulated("generic", vec![])))
        }
        // Nama tidak dikenal = salah konfigurasi: jangan diam-diam memakai
        // tiruan — tolak dengan pesan jelas agar data nyata tidak tertukar.
        other => Err(AppError::Validation(format!(
            "adapter \"{other}\" tidak dikenal — pilih: zkteco-terminal, solution-terminal, hikvision-terminal, timetech-ac6, atau mock*"
        ))),
    }
}

/// Sinkronisasi satu perangkat dengan retry. Mengembalikan statistik.
pub fn sync_device(
    repo: &Repo,
    device_id: &str,
    adapter: &mut Box<dyn DeviceAdapter>,
    max_retries: u32,
    page_size: usize,
    cancel: &AtomicBool,
) -> AppResult<SyncStats> {
    let row: Option<(String, String, String)> = repo.query_one(
        "SELECT id, host, adapter_name FROM devices WHERE id=?1 AND is_active=1",
        &[&device_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let Some((did, host, _adapter_name)) = row else {
        return Err(AppError::NotFound(format!("perangkat {device_id}")));
    };
    let checkpoint: String = repo
        .query_one(
            "SELECT checkpoint FROM device_sync_state WHERE device_id=?1",
            &[&did],
            |r| r.get(0),
        )?
        .unwrap_or_default();

    let delays = backoff_delays(500, max_retries.max(1));
    let mut attempts = 0u32;
    let mut last_err = String::new();
    for (i, wait) in delays.iter().enumerate() {
        attempts = i as u32 + 1;
        match try_sync_once(repo, &did, &host, adapter, &checkpoint, page_size, cancel) {
            Ok(s) => {
                let now = timex::now_wib_string();
                repo.exec(
                    "INSERT INTO device_sync_state (device_id, checkpoint, last_success_at, consecutive_failures, updated_at)
                     VALUES (?1,?2,?3,0,?3)
                     ON CONFLICT(device_id) DO UPDATE SET checkpoint=excluded.checkpoint, last_success_at=excluded.last_success_at, consecutive_failures=0, updated_at=excluded.updated_at",
                    &[&did, &s.checkpoint, &now],
                )?;
                repo.exec(
                    "UPDATE devices SET status='online', last_sync_at=?1 WHERE id=?2",
                    &[&now, &did],
                )?;
                repo.exec(
                    "INSERT INTO device_connection_log (id, device_id, event, detail, created_at) VALUES (?1,?2,'sync_ok',?3,?4)",
                    &[&uuid::Uuid::new_v4().to_string(), &did, &format!("fetched={} inserted={}", s.fetched, s.inserted), &now],
                )?;
                // attempts mencerminkan usaha sungguhan (biasanya 1, ×retry).
                return Ok(SyncStats { attempts, ..s });
            }
            Err(e) => {
                last_err = e.to_string();
                // Pembatalan user BUKAN kegagalan perangkat: jangan masukkan
                // backoff, jangan menaikkan consecutive_failures.
                if last_err.contains("dibatalkan") {
                    break;
                }
                if !(matches!(e, AppError::Device(_))
                    || last_err.contains("SIMULATED")
                    || last_err.contains("koneksi")
                    || last_err.contains("belum connect"))
                {
                    // Non-retryable lain: berhenti cepat.
                    break;
                }
                if (i as u32) < max_retries.saturating_sub(1) {
                    std::thread::sleep(Duration::from_millis(*wait));
                }
            }
        }
    }
    let now = timex::now_wib_string();
    // Pembatalan oleh pengguna: keluar bersih, tanpa menandai perangkat
    // error / menaikkan hitungan kegagalan (mis. status error palsu).
    if last_err.contains("dibatalkan") {
        log::info!("sinkron {did} dibatalkan pengguna");
        return Err(AppError::Conflict("sinkronisasi dibatalkan".into()));
    }
    repo.exec(
        "INSERT INTO device_sync_state (device_id, checkpoint, consecutive_failures, updated_at)
         VALUES (?1,?2,1,?3)
         ON CONFLICT(device_id) DO UPDATE SET consecutive_failures=consecutive_failures+1, updated_at=excluded.updated_at",
        &[&did, &checkpoint, &now],
    )?;
    repo.exec("UPDATE devices SET status='error' WHERE id=?1", &[&did])?;
    repo.exec(
        "INSERT INTO device_connection_log (id, device_id, event, detail, created_at) VALUES (?1,?2,'sync_fail',?3,?4)",
        &[&uuid::Uuid::new_v4().to_string(), &did, &last_err, &now],
    )?;
    Err(AppError::Device(format!(
        "sync gagal setelah {attempts}x: {last_err}"
    )))
}

fn try_sync_once(
    repo: &Repo,
    device_id: &str,
    host: &str,
    adapter: &mut Box<dyn DeviceAdapter>,
    checkpoint: &str,
    page_size: usize,
    cancel: &AtomicBool,
) -> AppResult<SyncStats> {
    adapter
        .initialize()
        .map_err(|e| AppError::Device(e.to_string()))?;
    adapter
        .connect(
            if host.is_empty() { "local" } else { host },
            Duration::from_secs(10),
        )
        .map_err(|e| AppError::Device(e.to_string()))?;
    let (logs, new_cp) = adapter
        .fetch_attendance_logs(checkpoint, page_size.max(1), cancel)
        .map_err(|e| AppError::Device(e.to_string()))?;
    let events: Vec<crate::db::repo::RawEventIn> = logs
        .into_iter()
        .map(|l| crate::db::repo::RawEventIn {
            device_id: device_id.to_string(),
            vendor_record_id: l.vendor_record_id,
            device_user_id: l.device_user_id,
            event_time: l.event_time,
            direction: l.direction,
            payload_json: "{}".into(),
        })
        .collect();
    let fetched = events.len() as i64;
    let rep = repo.transaction_ingest_events(&events)?;
    let _ = adapter.disconnect();
    Ok(SyncStats {
        device_id: device_id.to_string(),
        fetched,
        inserted: rep.inserted,
        duplicates: rep.duplicates,
        unmapped: rep.unmapped,
        // Jujur: baris waktu korup yang dilewati dihitung gagal (bukan 0).
        failed: rep.invalid,
        checkpoint: if new_cp.is_empty() {
            checkpoint.to_string()
        } else {
            new_cp
        },
        attempts: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::adapter::DeviceLog;
    use std::sync::atomic::AtomicBool;

    fn repo_with_device() -> Repo {
        let r = Repo::open_in_memory().unwrap();
        r.exec("INSERT INTO devices (id,name,vendor,model,adapter_name,sdk_version,status,created_at,updated_at) VALUES ('d1','Sim','SIMULATED','M','mock-simulated','mock-1.0','offline','2026-01-01 00:00:00','2026-01-01 00:00:00')", &[]).unwrap();
        r
    }

    #[test]
    fn sync_inserts_and_checkpoints() {
        let r = repo_with_device();
        let logs = vec![
            DeviceLog {
                vendor_record_id: "R1".into(),
                device_user_id: "U1".into(),
                event_time: "2026-05-04 07:00:00".into(),
                direction: "in".into(),
            },
            DeviceLog {
                vendor_record_id: "R2".into(),
                device_user_id: "U1".into(),
                event_time: "2026-05-04 15:00:00".into(),
                direction: "out".into(),
            },
        ];
        let mut a: Box<dyn DeviceAdapter> = Box::new(MockAdapter::new_simulated("t", logs));
        let cancel = AtomicBool::new(false);
        let s = sync_device(&r, "d1", &mut a, 2, 50, &cancel).unwrap();
        assert_eq!(s.inserted, 2);
        assert_eq!(s.checkpoint, "R2");
        // Sync ulang idempotent: tidak ada data baru setelah checkpoint.
        let s2 = sync_device(&r, "d1", &mut a, 2, 50, &cancel).unwrap();
        assert_eq!(s2.fetched, 0);
    }

    #[test]
    fn sync_retry_lalu_gagal_tertulis() {
        let r = repo_with_device();
        let mut m = MockAdapter::new_simulated("t", vec![]);
        m.fail_next = 99;
        let mut a: Box<dyn DeviceAdapter> = Box::new(m);
        let cancel = AtomicBool::new(false);
        let res = sync_device(&r, "d1", &mut a, 2, 50, &cancel);
        assert!(res.is_err());
        let st: String = r
            .query_one("SELECT status FROM devices WHERE id='d1'", &[], |x| {
                x.get(0)
            })
            .unwrap()
            .unwrap();
        assert_eq!(st, "error");
    }
}
