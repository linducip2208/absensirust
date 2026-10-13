//! Adapter Solution (terminal absensi) — satu platform dengan ZKTeco (ZKEM).
//!
//! Fakta vendor (terverifikasi): halaman resmi `solution.co.id/sdk.php`
//! menyatakan SDK Solution = "ZKEM SDK" (ActiveX DLL + manual + contoh C#/VB),
//! kompatibel dengan X100-C, X107, X105, X304/X305, X401, X601, X606-S, X609,
//! X900, dsb. Paket `Standalone SDK 6.3.1.54` di `vendor_sdk/` berisi DLL +
//! manual + demo ZKTeco yang sama persis. Maka adapter ini memakai inti COM
//! yang sama (`zkcom`) dengan identitas Solution sendiri.

use super::adapter::{
    AdapterError, AdapterInfo, Capabilities, ConnStatus, DeviceAdapter, DeviceLog, DeviceUser,
};
use super::zkteco::{ZkIdentity, ZktecoAdapter};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub struct SolutionAdapter {
    inner: ZktecoAdapter,
}

impl SolutionAdapter {
    pub fn new(config_json: &str) -> Self {
        Self {
            inner: ZktecoAdapter::with_identity(
                config_json,
                ZkIdentity {
                    adapter_name: "solution-terminal",
                    vendor: "Solution",
                    models: "terminal absensi (platform ZKEM)",
                },
            ),
        }
    }
}

impl DeviceAdapter for SolutionAdapter {
    fn info(&self) -> AdapterInfo {
        self.inner.info()
    }
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    fn initialize(&mut self) -> Result<(), AdapterError> {
        self.inner.initialize()
    }
    fn connect(&mut self, host: &str, timeout: Duration) -> Result<(), AdapterError> {
        self.inner.connect(host, timeout)
    }
    fn disconnect(&mut self) -> Result<(), AdapterError> {
        self.inner.disconnect()
    }
    fn health_check(&mut self) -> Result<ConnStatus, AdapterError> {
        self.inner.health_check()
    }
    fn fetch_attendance_logs(
        &mut self,
        checkpoint: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<(Vec<DeviceLog>, String), AdapterError> {
        self.inner.fetch_attendance_logs(checkpoint, limit, cancel)
    }
    fn fetch_device_users(&mut self) -> Result<Vec<DeviceUser>, AdapterError> {
        self.inner.fetch_device_users()
    }
    fn provision_user(&mut self, user: &DeviceUser) -> Result<(), AdapterError> {
        self.inner.provision_user(user)
    }
    fn fetch_device_info(&mut self) -> Result<serde_json::Value, AdapterError> {
        self.inner.fetch_device_info()
    }
    fn get_sdk_version(&self) -> String {
        self.inner.get_sdk_version()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_solution() {
        let a = SolutionAdapter::new("{}");
        let info = a.info();
        assert_eq!(info.adapter_name, "solution-terminal");
        assert_eq!(info.vendor, "Solution");
        let caps = a.capabilities();
        assert!(matches!(
            caps.fetch_logs,
            super::super::adapter::Capability::Supported
        ));
    }

    // Hardware: butuh perangkat fisik Solution.
    //   set ATTENDX_TEST_SOLUTION_HOST=192.168.1.202[:4370]
    //   cargo test -p attendx_rust_lib solution_hw -- --ignored --nocapture
    #[test]
    #[ignore]
    fn solution_hw_connect_smoke() {
        let host = std::env::var("ATTENDX_TEST_SOLUTION_HOST").expect("set host");
        let mut a = SolutionAdapter::new("{}");
        a.initialize().expect("SDK terinstal");
        a.connect(&host, Duration::from_secs(10)).expect("connect");
        let info = a.fetch_device_info().expect("info");
        assert!(info.get("serial_number").is_some());
    }
}
