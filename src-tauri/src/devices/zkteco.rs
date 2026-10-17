//! Adapter ZKTeco (terminal absensi, platform ZKEM) via COM `zkcom`.
//!
//! Sumber terverifikasi: SDK 6.3.1.54 x64 (`vendor_sdk/.../SDK/x64/`),
//! manual `ZKTeco_Standalone_SDK_Development_Manual_V2.2_A.3-EN.pdf`,
//! demo `Demo/C#/TFT/{AttLogs,UserInfo,Device Management}`.
//!
//! Desain: stateless — tiap operasi membuka sesi COM sendiri lalu menutupnya
//! (aman dari thread pool; tidak ada handle lintas-thread). Pembacaan log
//! NON-DESTRUKTIF (`ClearGLog` tidak pernah dipanggil); deduplikasi merujuk
//! `vendor_record_id` deterministik di ingest. Checkpoint = `event_time`
//! maksimum yang sudah ditarik (format WIB).

use super::adapter::{
    AdapterError, AdapterInfo, Capabilities, Capability, ConnStatus, DeviceAdapter, DeviceLog,
    DeviceUser,
};
use super::zkcom::{
    self, last_host, parse_host_port, parse_machine_number, parse_port, remember_host, ZkSession,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const ZK_DEFAULT_PORT: u16 = 4370;
/// Batas iterasi baca buffer sebagai pengaman (tiap iterasi cek cancel).
const MAX_SCAN_ROWS: usize = 20_000;

/// Versi SDK yang dipakai referensi implementasi (terverifikasi di vendor_sdk/).
pub const SDK_VERSION: &str = "6.3.1.54 (ZKTeco/Solution ZKEM x64+x86; NuGet PullSDK x64 6.3.1.37)";

pub struct ZktecoAdapter {
    config_json: String,
    identity: ZkIdentity,
}

/// Identitas vendor untuk inti ZKEM bersama (ZKTeco & Solution satu platform).
#[derive(Clone)]
pub(crate) struct ZkIdentity {
    pub adapter_name: &'static str,
    pub vendor: &'static str,
    pub models: &'static str,
}

impl ZktecoAdapter {
    pub fn new(config_json: &str) -> Self {
        Self {
            config_json: config_json.to_string(),
            identity: ZkIdentity {
                adapter_name: "zkteco-terminal",
                vendor: "ZKTeco",
                models: "terminal absensi (platform ZKEM)",
            },
        }
    }

    pub(crate) fn with_identity(config_json: &str, identity: ZkIdentity) -> Self {
        Self {
            config_json: config_json.to_string(),
            identity,
        }
    }

    fn endpoint(&self, host: &str) -> (String, u16, i32) {
        let cfg_port = parse_port(&self.config_json, ZK_DEFAULT_PORT);
        let (ip, port) = parse_host_port(host, cfg_port);
        (ip, port, parse_machine_number(&self.config_json))
    }
}

impl DeviceAdapter for ZktecoAdapter {
    fn info(&self) -> AdapterInfo {
        AdapterInfo {
            adapter_name: self.identity.adapter_name.into(),
            sdk_version: SDK_VERSION.into(),
            vendor: self.identity.vendor.into(),
            models: vec![self.identity.models.into()],
            dll_arch: "x64 + x86 (terverifikasi header PE)".into(),
            notes:
                "COM CZKEM via Connect_Net; butuh zkemkeeper.dll x64 terdaftar + perangkat fisik."
                    .into(),
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            fetch_logs: Capability::Supported,
            fetch_users: Capability::Supported,
            provision_user: Capability::Supported,
            device_info: Capability::Supported,
            cancel_sync: Capability::Supported,
        }
    }

    fn initialize(&mut self) -> Result<(), AdapterError> {
        match zkcom::find_vendor_dll("zkemkeeper.dll") {
            Some(p) => match std::fs::read(&p) {
                Ok(bytes) => match zkcom::pe_arch_label(&bytes) {
                    Some("x64") => Ok(()),
                    Some(other) => Err(AdapterError::Connection(format!(
                        "zkemkeeper.dll ditemukan di {} tetapi arsitektur {other}; \
                         aplikasi x64 butuh DLL x64 (folder SDK/x64).",
                        p.display()
                    ))),
                    None => Err(AdapterError::Connection(format!(
                        "file {} bukan DLL PE valid.",
                        p.display()
                    ))),
                },
                Err(e) => Err(AdapterError::Connection(format!(
                    "tidak bisa membaca {}: {e}",
                    p.display()
                ))),
            },
            None => Err(AdapterError::Connection(
                "zkemkeeper.dll tidak ditemukan. Salin dari SDK/x64 ke folder exe atau \
                 %APPDATA%\\AttendX\\sdk lalu daftarkan (Register_SDK x64.bat, Administrator)."
                    .to_string(),
            )),
        }
    }

    fn connect(&mut self, host: &str, _timeout: Duration) -> Result<(), AdapterError> {
        if host.trim().is_empty() || host.trim() == "local" {
            return Err(AdapterError::Connection(
                "host kosong: isi IP perangkat (mis. 192.168.1.201)".to_string(),
            ));
        }
        let (ip, port, _machine) = self.endpoint(host);
        // Uji hidup: sesi dibuka lalu langsung ditutup (Drop).
        let _session = ZkSession::open(&ip, port)?;
        remember_host(host);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), AdapterError> {
        // Sesi selalu self-contained per operasi; tidak ada handle tersimpan.
        Ok(())
    }

    fn health_check(&mut self) -> Result<ConnStatus, AdapterError> {
        // Uji hidup nyata (sesi COM baru). Gagal → Offline, bukan error keras.
        let host = last_host();
        if host.trim().is_empty() {
            return Ok(ConnStatus::Offline);
        }
        let (ip, port, _machine) = self.endpoint(&host);
        match ZkSession::open(&ip, port) {
            Ok(_) => Ok(ConnStatus::Online),
            Err(_) => Ok(ConnStatus::Offline),
        }
    }

    fn fetch_attendance_logs(
        &mut self,
        checkpoint: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<(Vec<DeviceLog>, String), AdapterError> {
        let host = last_host();
        let (ip, port, machine) = self.endpoint(&host);
        let mut session = ZkSession::open(&ip, port)?;
        // Baca log TIDAK perlu menonaktifkan perangkat (dulu:
        // EnableDevice(false) lalu EnableDevice(true) setelah loop — bila
        // re-enable gagal terminal nyaris-mati/terkunci untuk absensi).
        session.read_general_log_data(machine)?;
        let mut out: Vec<DeviceLog> = Vec::new();
        let mut new_cp = checkpoint.to_string();
        let skip_before = checkpoint_time(checkpoint);
        let mut scanned = 0usize;
        let mut loop_err: Option<AdapterError> = None;
        loop {
            if cancel.load(Ordering::Relaxed) {
                loop_err = Some(AdapterError::Cancelled);
                break;
            }
            if out.len() >= limit.max(1) || scanned >= MAX_SCAN_ROWS {
                break;
            }
            scanned += 1;
            match session.ssr_get_general_log_data(machine) {
                Err(e) => {
                    loop_err = Some(e);
                    break;
                }
                Ok(None) => break,
                Ok(Some(row)) => {
                    let Some(event_time) = zkcom::build_event_time(
                        row.year, row.month, row.day, row.hour, row.minute, row.second,
                    ) else {
                        continue; // baris korup dari perangkat → lewati, jangan karang.
                    };
                    if let Some(skip) = &skip_before {
                        if event_time.as_str() < skip.as_str() {
                            continue;
                        }
                    }
                    let vendor_record_id = zkcom::build_record_id(&row.enroll, &row);
                    out.push(DeviceLog {
                        vendor_record_id,
                        device_user_id: row.enroll,
                        event_time: event_time.clone(),
                        direction: zkcom::inout_direction(row.inout),
                    });
                    if event_time > new_cp {
                        new_cp = event_time;
                    }
                }
            }
        }
        // Baris korup di akhir tidak menghambat hasil; perangkat tetap
        // aktif sepanjang pembacaan (tanpa EnableDevice).
        if let Some(e) = loop_err {
            return Err(e);
        }
        Ok((out, new_cp))
    }

    fn fetch_device_users(&mut self) -> Result<Vec<DeviceUser>, AdapterError> {
        let host = last_host();
        let (ip, port, machine) = self.endpoint(&host);
        let mut session = ZkSession::open(&ip, port)?;
        session.read_all_user_id(machine)?;
        let mut users = Vec::new();
        for _ in 0..MAX_SCAN_ROWS {
            match session.ssr_get_all_user_info(machine)? {
                None => break,
                Some(u) => {
                    if u.enroll.trim().is_empty() {
                        continue;
                    }
                    users.push(DeviceUser {
                        device_user_id: u.enroll,
                        name: u.name,
                    });
                }
            }
        }
        Ok(users)
    }

    fn provision_user(&mut self, user: &DeviceUser) -> Result<(), AdapterError> {
        let enroll = user.device_user_id.trim();
        if enroll.is_empty() {
            return Err(AdapterError::Unsupported(
                "device_user_id kosong".to_string(),
            ));
        }
        let host = last_host();
        let (ip, port, machine) = self.endpoint(&host);
        let mut session = ZkSession::open(&ip, port)?;
        // privilege 0 (user biasa), enabled. Password/template tidak dikelola di sini.
        session.ssr_set_user_info(machine, enroll, user.name.trim(), 0, true)
    }

    fn fetch_device_info(&mut self) -> Result<serde_json::Value, AdapterError> {
        let host = last_host();
        let (ip, port, machine) = self.endpoint(&host);
        let mut session = ZkSession::open(&ip, port)?;
        let info = serde_json::json!({
            "serial_number": session.serial_number(machine).unwrap_or_default(),
            "product_code": session.product_code(machine).unwrap_or_default(),
            "firmware_version": session.firmware_version(machine).unwrap_or_default(),
            "device_mac": session.device_mac(machine).unwrap_or_default(),
            "sdk_version": session.sdk_version_live().unwrap_or_else(|| SDK_VERSION.to_string()),
            "log_count": session.record_count(machine).unwrap_or(-1),
            "endpoint": format!("{ip}:{port}"),
        });
        Ok(info)
    }

    fn get_sdk_version(&self) -> String {
        SDK_VERSION.to_string()
    }
}

// ---------------------------------------------------------------------------
// Checkpoint = event_time WIB maksimum; `None` = dari awal.
// ---------------------------------------------------------------------------

/// Checkpoint berupa event_time WIB maksimum; `None` = dari awal.
/// Format checkpoint ditulis oleh adapter ini: `"YYYY-MM-DD HH:MM:SS"`.
fn checkpoint_time(checkpoint: &str) -> Option<String> {
    let cp = checkpoint.trim();
    if cp.len() == 19
        && cp.as_bytes()[4] == b'-'
        && cp.as_bytes()[13] == b':'
        && zkcom::build_event_time(
            cp[0..4].parse().unwrap_or(0),
            cp[5..7].parse().unwrap_or(0),
            cp[8..10].parse().unwrap_or(0),
            cp[11..13].parse().unwrap_or(0),
            cp[14..16].parse().unwrap_or(0),
            cp[17..19].parse().unwrap_or(0),
        )
        .is_some()
    {
        Some(cp.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_parsing() {
        assert_eq!(checkpoint_time(""), None);
        assert_eq!(checkpoint_time("R2"), None); // checkpoint gaya mock lama → dari awal.
        assert_eq!(
            checkpoint_time("2026-10-13 07:05:09"),
            Some("2026-10-13 07:05:09".to_string())
        );
        assert_eq!(checkpoint_time("2026-13-40 99:99:99"), None);
    }

    // Hardware: butuh perangkat fisik.
    //   set ATTENDX_TEST_ZK_HOST=192.168.1.201[:4370] (+ ATTENDX_TEST_ZK_CONFIG={"machine":1})
    //   cargo test -p attendx_rust_lib zkteco_hw -- --ignored --nocapture
    #[test]
    #[ignore]
    fn zkteco_hw_fetch_smoke() {
        let host = std::env::var("ATTENDX_TEST_ZK_HOST").expect("set ATTENDX_TEST_ZK_HOST");
        let cfg = std::env::var("ATTENDX_TEST_ZK_CONFIG").unwrap_or_else(|_| "{}".into());
        let mut a = ZktecoAdapter::new(&cfg);
        a.initialize().expect("SDK terinstal");
        a.connect(&host, Duration::from_secs(10)).expect("connect");
        remember_host(&host);
        let cancel = AtomicBool::new(false);
        let (logs, cp) = a.fetch_attendance_logs("", 5, &cancel).expect("fetch");
        assert!(logs.len() <= 5);
        let _ = (logs, cp);
        let info = a.fetch_device_info().expect("info");
        assert!(info.get("serial_number").is_some());
    }
}
