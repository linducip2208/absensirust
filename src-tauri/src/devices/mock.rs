//! Mock adapter — HANYA untuk automated test & demo.
//! Semua perangkat mock ditandai SIMULATED dan tidak boleh diklaim hardware nyata.

use super::adapter::*;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub struct MockAdapter {
    pub logs: VecDeque<DeviceLog>,
    pub users: Vec<DeviceUser>,
    pub connected: bool,
    pub fail_next: usize,
    pub info: AdapterInfo,
}

impl MockAdapter {
    pub fn new_simulated(vendor: &str, seed_logs: Vec<DeviceLog>) -> Self {
        Self {
            logs: seed_logs.into(),
            users: vec![],
            connected: false,
            fail_next: 0,
            info: AdapterInfo {
                adapter_name: format!("mock-{vendor}"),
                sdk_version: "mock-1.0.0 (SIMULATED)".into(),
                vendor: vendor.into(),
                models: vec!["SIMULATED".into()],
                dll_arch: "n/a (SIMULATED)".into(),
                notes: "SIMULATED — hanya untuk test/demo, bukan hardware nyata.".into(),
            },
        }
    }
}

impl DeviceAdapter for MockAdapter {
    fn info(&self) -> AdapterInfo {
        self.info.clone()
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
        Ok(())
    }
    fn connect(&mut self, _host: &str, _timeout: Duration) -> Result<(), AdapterError> {
        if self.fail_next > 0 {
            self.fail_next -= 1;
            return Err(AdapterError::Connection("SIMULATED failure".into()));
        }
        self.connected = true;
        Ok(())
    }
    fn disconnect(&mut self) -> Result<(), AdapterError> {
        self.connected = false;
        Ok(())
    }
    fn health_check(&mut self) -> Result<ConnStatus, AdapterError> {
        Ok(if self.connected {
            ConnStatus::Online
        } else {
            ConnStatus::Offline
        })
    }
    fn fetch_attendance_logs(
        &mut self,
        checkpoint: &str,
        limit: usize,
        cancel: &AtomicBool,
    ) -> Result<(Vec<DeviceLog>, String), AdapterError> {
        if !self.connected {
            return Err(AdapterError::Connection("belum connect".into()));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(AdapterError::Cancelled);
        }
        if self.fail_next > 0 {
            self.fail_next -= 1;
            return Err(AdapterError::Connection("SIMULATED fetch failure".into()));
        }
        let mut out = vec![];
        let mut last_cp = checkpoint.to_string();
        let mut skipped = checkpoint.is_empty();
        // Checkpoint = vendor_record_id terakhir; ambil setelahnya (incremental).
        let all: Vec<DeviceLog> = self.logs.iter().cloned().collect();
        for log in all {
            if cancel.load(Ordering::Relaxed) {
                return Err(AdapterError::Cancelled);
            }
            if !skipped {
                if log.vendor_record_id == checkpoint {
                    skipped = true;
                }
                continue;
            }
            out.push(log.clone());
            last_cp = log.vendor_record_id.clone();
            if out.len() >= limit {
                break;
            }
        }
        Ok((out, last_cp))
    }
    fn fetch_device_users(&mut self) -> Result<Vec<DeviceUser>, AdapterError> {
        Ok(self.users.clone())
    }
    fn provision_user(&mut self, user: &DeviceUser) -> Result<(), AdapterError> {
        self.users.push(user.clone());
        Ok(())
    }
    fn fetch_device_info(&mut self) -> Result<serde_json::Value, AdapterError> {
        Ok(serde_json::json!({"simulated": true, "vendor": self.info.vendor}))
    }
    fn get_sdk_version(&self) -> String {
        self.info.sdk_version.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mock_checkpoint_pagination() {
        let logs = (1..=5)
            .map(|i| DeviceLog {
                vendor_record_id: format!("R{i}"),
                device_user_id: "U1".into(),
                event_time: "2026-05-04 07:00:00".into(),
                direction: "in".into(),
            })
            .collect();
        let mut m = MockAdapter::new_simulated("demo", logs);
        m.connect("x", Duration::from_secs(1)).unwrap();
        let cancel = AtomicBool::new(false);
        let (p1, cp1) = m.fetch_attendance_logs("", 2, &cancel).unwrap();
        assert_eq!(p1.len(), 2);
        let (p2, _) = m.fetch_attendance_logs(&cp1, 10, &cancel).unwrap();
        assert_eq!(p2.len(), 3);
    }
    #[test]
    fn mock_cancel() {
        let mut m = MockAdapter::new_simulated("demo", vec![]);
        m.connect("x", Duration::from_secs(1)).unwrap();
        let cancel = AtomicBool::new(true);
        assert!(matches!(
            m.fetch_attendance_logs("", 10, &cancel),
            Err(AdapterError::Cancelled)
        ));
    }
}
