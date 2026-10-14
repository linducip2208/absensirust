//! Registry adapter: status implementasi per vendor (jujur, terverifikasi).
//! Lihat `docs/SDK_INVENTORY.md` + `vendor_sdk/README.md` untuk bukti SDK.

use super::adapter::{AdapterInfo, Capabilities, Capability};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub adapter_name: String,
    pub vendor: String,
    pub models: Vec<String>,
    pub sdk_version: String,
    pub dll_arch: String,
    pub status: String,
    pub supported_ops: Vec<String>,
    pub missing: String,
}

pub struct DeviceRegistry;

impl DeviceRegistry {
    pub fn entries() -> Vec<RegistryEntry> {
        vec![
            RegistryEntry {
                adapter_name: "zkteco-terminal".into(),
                vendor: "ZKTeco".into(),
                models: vec!["terminal absensi (platform ZKEM)".into()],
                sdk_version: "6.3.1.54 (ZKEM x64+x86) + PullSDK x64 6.3.1.37".into(),
                dll_arch: "x64 + x86 (terverifikasi header PE)".into(),
                status: "TERSEDIA — implementasi COM CZKEM; butuh hardware untuk uji".into(),
                supported_ops: vec![
                    "fetch_logs".into(),
                    "fetch_users".into(),
                    "provision_user".into(),
                    "device_info".into(),
                ],
                missing: "Perangkat fisik untuk uji connect/fetch/provision.".into(),
            },
            RegistryEntry {
                adapter_name: "solution-terminal".into(),
                vendor: "Solution".into(),
                models: vec![
                    "X100-C".into(),
                    "X105".into(),
                    "X107".into(),
                    "X304".into(),
                    "X401".into(),
                    "X601".into(),
                    "X606-S".into(),
                    "X609".into(),
                    "X900".into(),
                ],
                sdk_version: "6.3.1.54 (ZKEM x64+x86, dari support Solution)".into(),
                dll_arch: "x64 + x86 (terverifikasi header PE)".into(),
                status: "TERSEDIA — inti COM sama dengan ZKTeco (satu keluarga SDK); TIAP MODEL butuh uji hardware sendiri"
                    .into(),
                supported_ops: vec![
                    "fetch_logs".into(),
                    "fetch_users".into(),
                    "provision_user".into(),
                    "device_info".into(),
                ],
                missing: "Uji hardware per model (daftar model = klaim vendor solution.co.id, bukan hasil uji).".into(),
            },
            RegistryEntry {
                adapter_name: "hikvision-terminal".into(),
                vendor: "Hikvision".into(),
                models: vec!["terminal akses/absensi".into()],
                sdk_version: "HCNetSDK 6.1.6.3 x64 (DLL terverifikasi)".into(),
                dll_arch: "x64 (terverifikasi header PE)".into(),
                status: "SEBAGIAN — login/info via HCNetSDK; tarik log BUTUH dokumen ISAPI/ACS + hardware"
                    .into(),
                supported_ops: vec!["device_info".into()],
                missing: "Dokumen ISAPI/ACS resmi untuk event absensi + perangkat fisik.".into(),
            },
            RegistryEntry {
                adapter_name: "timetech-ac6".into(),
                vendor: "TIME TECH".into(),
                models: vec!["AC6".into()],
                sdk_version: "tidak ada (vendor tidak menerbitkan SDK PC)".into(),
                dll_arch: "n/a".into(),
                status: "TIDAK TERSEDIA — AC6 access controller Wiegand-only, bukan terminal TCP/IP"
                    .into(),
                supported_ops: vec![],
                missing: "SDK/dokumen protokol resmi vendor (bila ada) + hardware.".into(),
            },
            RegistryEntry {
                adapter_name: "mock-simulated".into(),
                vendor: "SIMULATED".into(),
                models: vec!["SIMULATED".into()],
                sdk_version: "mock-1.0.0 (SIMULATED)".into(),
                dll_arch: "n/a".into(),
                status: "SIMULATED — test/demo saja".into(),
                supported_ops: vec![
                    "fetch_logs".into(),
                    "fetch_users".into(),
                    "provision_user".into(),
                ],
                missing: "Bukan hardware nyata.".into(),
            },
        ]
    }

    pub fn stub_capabilities() -> Capabilities {
        Capabilities {
            fetch_logs: Capability::Unknown,
            fetch_users: Capability::Unknown,
            provision_user: Capability::Unknown,
            device_info: Capability::Unknown,
            cancel_sync: Capability::Unknown,
        }
    }

    pub fn stub_info(adapter_name: &str) -> AdapterInfo {
        AdapterInfo {
            adapter_name: adapter_name.into(),
            sdk_version: "unknown (SDK belum tersedia)".into(),
            vendor: "unknown".into(),
            models: vec![],
            dll_arch: "unverified".into(),
            notes: "Adapter nyata belum diimplementasikan — butuh SDK vendor & hardware.".into(),
        }
    }
}
