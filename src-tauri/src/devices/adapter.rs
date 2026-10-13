//! Kontrak trait adapter vendor. Operasi yang tidak didukung SDK
//! harus mengembalikan Capability::Unsupported — jangan diarang.

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub adapter_name: String,
    pub sdk_version: String,
    pub vendor: String,
    pub models: Vec<String>,
    pub dll_arch: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub fetch_logs: Capability,
    pub fetch_users: Capability,
    pub provision_user: Capability,
    pub device_info: Capability,
    pub cancel_sync: Capability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLog {
    pub vendor_record_id: String,
    pub device_user_id: String,
    /// WIB "YYYY-MM-DD HH:MM:SS"
    pub event_time: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceUser {
    pub device_user_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnStatus {
    Online,
    Offline,
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("timeout setelah {0:?}")]
    Timeout(Duration),
    #[error("koneksi: {0}")]
    Connection(String),
    #[error("tidak didukung: {0}")]
    Unsupported(String),
    #[error("sdk: {0}")]
    Sdk(String),
    #[error("dibatalkan")]
    Cancelled,
}

pub trait DeviceAdapter: Send + Sync {
    fn info(&self) -> AdapterInfo;
    fn capabilities(&self) -> Capabilities;
    fn initialize(&mut self) -> Result<(), AdapterError>;
    fn connect(&mut self, host: &str, timeout: Duration) -> Result<(), AdapterError>;
    fn disconnect(&mut self) -> Result<(), AdapterError>;
    fn health_check(&mut self) -> Result<ConnStatus, AdapterError>;
    fn fetch_attendance_logs(
        &mut self,
        checkpoint: &str,
        limit: usize,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(Vec<DeviceLog>, String), AdapterError>;
    fn fetch_device_users(&mut self) -> Result<Vec<DeviceUser>, AdapterError>;
    fn provision_user(&mut self, user: &DeviceUser) -> Result<(), AdapterError>;
    fn fetch_device_info(&mut self) -> Result<serde_json::Value, AdapterError>;
    fn get_sdk_version(&self) -> String;
}

/// Helper retry dengan exponential backoff (tanpa sleep di test via base_ms=0).
pub fn backoff_delays(base_ms: u64, max_retries: u32) -> Vec<u64> {
    (0..max_retries)
        .map(|i| base_ms.saturating_mul(1 << i.min(6)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backoff_grows_exponentially() {
        assert_eq!(backoff_delays(100, 4), vec![100, 200, 400, 800]);
    }
}
