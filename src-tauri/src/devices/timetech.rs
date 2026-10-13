//! Adapter TIME TECH AC6 — SENGAJA stub `Unsupported`.
//!
//! Fakta terverifikasi (bukan tebakan):
//! - Menurut halaman resmi distributor (`taruna-jaya.com/en/product/time-tech-ac6/`),
//!   AC6 adalah *door access controller* standalone (fingerprint + RFID 125kHz +
//!   PIN, 1000 user): output berupa Lock/Exit-Button/**Wiegand Out**/Door-Bell.
//! - Tidak ada port TCP/IP, tidak ada SDK PC yang dipublikasikan vendor.
//! - `docs/SDK_ADAPTER_DEVELOPMENT.md` melarang memakai Wiegand generik sebagai
//!   pengganti SDK nyata tanpa dokumentasi vendor.
//!
//! Maka adapter ini menolak semua operasi dengan pesan jelas sampai vendor
//! menerbitkan SDK/dokumen protokol + tersedia hardware untuk uji.

use super::adapter::{
    AdapterError, AdapterInfo, Capabilities, Capability, ConnStatus, DeviceAdapter, DeviceLog,
    DeviceUser,
};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

fn unsupported(op: &str) -> AdapterError {
    AdapterError::Unsupported(format!(
        "TIME TECH AC6: operasi '{op}' tidak tersedia — perangkat ini access controller \
         standalone (Wiegand-only, tanpa SDK PC menurut vendor). Integrasi butuh dokumen \
         protokol resmi + hardware untuk uji. Lihat docs/SDK_INVENTORY.md."
    ))
}

pub struct TimetechAc6Adapter;

impl DeviceAdapter for TimetechAc6Adapter {
    fn info(&self) -> AdapterInfo {
        AdapterInfo {
            adapter_name: "timetech-ac6".into(),
            sdk_version: "tidak ada (vendor tidak menerbitkan SDK PC)".into(),
            vendor: "TIME TECH".into(),
            models: vec!["AC6".into()],
            dll_arch: "n/a".into(),
            notes: "Access controller Wiegand-only; bukan terminal absensi TCP/IP.".into(),
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            fetch_logs: Capability::Unsupported,
            fetch_users: Capability::Unsupported,
            provision_user: Capability::Unsupported,
            device_info: Capability::Unsupported,
            cancel_sync: Capability::Unsupported,
        }
    }

    fn initialize(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    fn connect(&mut self, _host: &str, _timeout: Duration) -> Result<(), AdapterError> {
        Err(unsupported("connect"))
    }

    fn disconnect(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    fn health_check(&mut self) -> Result<ConnStatus, AdapterError> {
        Err(unsupported("health_check"))
    }

    fn fetch_attendance_logs(
        &mut self,
        _checkpoint: &str,
        _limit: usize,
        _cancel: &AtomicBool,
    ) -> Result<(Vec<DeviceLog>, String), AdapterError> {
        Err(unsupported("fetch_attendance_logs"))
    }

    fn fetch_device_users(&mut self) -> Result<Vec<DeviceUser>, AdapterError> {
        Err(unsupported("fetch_device_users"))
    }

    fn provision_user(&mut self, _user: &DeviceUser) -> Result<(), AdapterError> {
        Err(unsupported("provision_user"))
    }

    fn fetch_device_info(&mut self) -> Result<serde_json::Value, AdapterError> {
        Err(unsupported("fetch_device_info"))
    }

    fn get_sdk_version(&self) -> String {
        "tidak ada (vendor tidak menerbitkan SDK PC)".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timetech_refuses_honestly() {
        let mut a = TimetechAc6Adapter;
        assert!(a.connect("192.168.1.1", Duration::from_secs(1)).is_err());
        let cancel = AtomicBool::new(false);
        assert!(a.fetch_attendance_logs("", 10, &cancel).is_err());
        let caps = a.capabilities();
        assert!(matches!(caps.fetch_logs, Capability::Unsupported));
    }
}
