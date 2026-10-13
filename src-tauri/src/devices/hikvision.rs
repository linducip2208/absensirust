//! Adapter Hikvision (terminal akses/absensi) via `HCNetSDK.dll`.
//!
//! Cakupan jujur (terverifikasi dari `vendor_sdk/Hikvision-Set/bin` +
//! `CHCNetSDK.cs`):
//! - Ekspor terverifikasi (842 ekspor, x64): `NET_DVR_Init`, `NET_DVR_Cleanup`,
//!   `NET_DVR_SetConnectTime`, `NET_DVR_Login_V30`, `NET_DVR_Logout`,
//!   `NET_DVR_Logout_V30`, `NET_DVR_GetLastError`, `NET_DVR_GetSDKVersion`
//!   — dipakai persis sesuai deklarasi `DllImport("HCNetSDK.dll")`
//!   (stdcall, string ANSI, BOOL 4-byte) di `CHCNetSDK.cs`.
//! - `fetch_logs` / `fetch_users` / `provision_user` = **Unsupported**:
//!   build SDK ini (fokus NVR/kamera) tidak mengekspos API event absensi
//!   terminal (AcsEvent/ISAPI). Diperlukan dokumen ISAPI/ACS resmi +
//!   perangkat fisik sebelum diklaim. `device_info` didukung via login
//!   (serial number dari `NET_DVR_DEVICEINFO_V30`, `SERIALNO_LEN=48`).
//!
//! Kredensial dari `config_json` perangkat:
//! `{"port":8000,"username":"admin","password":"..."}` (jangan log password).

use super::adapter::{
    AdapterError, AdapterInfo, Capabilities, Capability, ConnStatus, DeviceAdapter, DeviceLog,
    DeviceUser,
};
use super::zkcom::{find_vendor_dll, last_host, parse_host_port, remember_host};
use std::ffi::CString;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub const HIK_DEFAULT_PORT: u16 = 8000;
/// Versi DLL yang terverifikasi di vendor_sdk (bukan klaim dukungan penuh).
pub const SDK_VERSION: &str =
    "HCNetSDK 6.1.6.3 x64 (DLL terverifikasi; event absensi BUTUH dokumen ISAPI/ACS)";

type FnInit = unsafe extern "system" fn() -> i32;
type FnCleanup = unsafe extern "system" fn() -> i32;
type FnSetConnectTime = unsafe extern "system" fn(u32, u32) -> i32;
type FnLoginV30 = unsafe extern "system" fn(
    *const std::os::raw::c_char,
    i32,
    *const std::os::raw::c_char,
    *const std::os::raw::c_char,
    *mut u8,
) -> i32;
type FnLogout = unsafe extern "system" fn(i32) -> i32;
type FnGetLastError = unsafe extern "system" fn() -> u32;
type FnGetSdkVersion = unsafe extern "system" fn() -> u32;

fn conn_config(config_json: &str) -> (u16, String, String) {
    let v: serde_json::Value = serde_json::from_str(config_json).unwrap_or_default();
    let port = v
        .get("port")
        .and_then(|p| p.as_u64())
        .filter(|p| (1..=65535).contains(p))
        .unwrap_or(u64::from(HIK_DEFAULT_PORT)) as u16;
    let user = v
        .get("username")
        .and_then(|u| u.as_str())
        .unwrap_or("admin")
        .to_string();
    let pass = v
        .get("password")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    (port, user, pass)
}

fn ensure_lib() -> Result<libloading::Library, AdapterError> {
    let path = find_vendor_dll("HCNetSDK.dll").ok_or_else(|| {
        AdapterError::Connection(
            "HCNetSDK.dll tidak ditemukan. Salin DLL x64 resmi Hikvision (Device Network SDK) \
             ke folder exe atau %APPDATA%\\AttendX\\sdk. Unduh paket resmi di hikvision.com \
             (butuh akun) — lihat vendor_sdk/README.md."
                .to_string(),
        )
    })?;
    unsafe { libloading::Library::new(&path) }
        .map_err(|e| AdapterError::Connection(format!("gagal memuat {}: {e}", path.display())))
}

fn sym<'lib, T>(
    lib: &'lib libloading::Library,
    name: &[u8],
) -> Result<libloading::Symbol<'lib, T>, AdapterError> {
    unsafe { lib.get(name) }.map_err(|e| {
        AdapterError::Sdk(format!(
            "ekspor SDK hilang: {} ({e})",
            String::from_utf8_lossy(name)
        ))
    })
}

/// Login penuh (Init → SetConnectTime → Login_V30). Mengembalikan userID.
/// Wajib diimbangi `logout_cleanup(lib, uid)` oleh pemanggil.
fn login(
    lib: &libloading::Library,
    ip: &str,
    port: u16,
    user: &str,
    pass: &str,
    timeout: Duration,
    info_out: Option<&mut [u8; 256]>,
) -> Result<i32, AdapterError> {
    let init: libloading::Symbol<FnInit> = sym(lib, b"NET_DVR_Init")?;
    let ok = unsafe { init() };
    if ok == 0 {
        let e: libloading::Symbol<FnGetLastError> = sym(lib, b"NET_DVR_GetLastError")?;
        return Err(AdapterError::Connection(format!(
            "NET_DVR_Init gagal (code={})",
            unsafe { e() }
        )));
    }
    let set_time: libloading::Symbol<FnSetConnectTime> = sym(lib, b"NET_DVR_SetConnectTime")?;
    let wait = timeout.as_secs().clamp(2, 30) as u32;
    unsafe {
        set_time(wait * 1000, 1);
    }
    let login_fn: libloading::Symbol<FnLoginV30> = sym(lib, b"NET_DVR_Login_V30")?;
    let c_ip = CString::new(ip).map_err(|_| AdapterError::Connection("IP tidak valid".into()))?;
    let c_user = CString::new(user).unwrap_or_default();
    let c_pass = CString::new(pass).unwrap_or_default();
    let mut local_info = [0u8; 256];
    let info_ptr: *mut u8 = match info_out {
        Some(buf) => buf.as_mut_ptr(),
        None => local_info.as_mut_ptr(),
    };
    // SAFETY: CString hidup selama pemanggilan; buffer 256B ≥ struct 78B.
    let uid = unsafe {
        login_fn(
            c_ip.as_ptr(),
            i32::from(port),
            c_user.as_ptr(),
            c_pass.as_ptr(),
            info_ptr,
        )
    };
    if uid < 0 {
        let e: libloading::Symbol<FnGetLastError> = sym(lib, b"NET_DVR_GetLastError")?;
        let code = unsafe { e() };
        let _ = cleanup(lib);
        return Err(AdapterError::Connection(format!(
            "NET_DVR_Login_V30({ip}:{port}) gagal (code={code}). Periksa IP/port/user/password \
             dan aktivasi perangkat."
        )));
    }
    Ok(uid)
}

fn logout_cleanup(lib: &libloading::Library, uid: i32) {
    if let Ok(f) = unsafe { lib.get::<FnLogout>(b"NET_DVR_Logout") } {
        unsafe {
            f(uid);
        }
    }
    let _ = cleanup(lib);
}

fn cleanup(lib: &libloading::Library) -> Result<(), AdapterError> {
    let f: libloading::Symbol<FnCleanup> = sym(lib, b"NET_DVR_Cleanup")?;
    unsafe {
        f();
    }
    Ok(())
}

/// Serial number dari 48 byte pertama `NET_DVR_DEVICEINFO_V30` (`SERIALNO_LEN=48`
/// menurut `CHCNetSDK.cs` baris 27).
pub(crate) fn parse_serial(info: &[u8]) -> String {
    let raw = info.get(..48).unwrap_or(&[]);
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

pub struct HikvisionAdapter {
    config_json: String,
}

impl HikvisionAdapter {
    pub fn new(config_json: &str) -> Self {
        Self {
            config_json: config_json.to_string(),
        }
    }
}

impl DeviceAdapter for HikvisionAdapter {
    fn info(&self) -> AdapterInfo {
        AdapterInfo {
            adapter_name: "hikvision-terminal".into(),
            sdk_version: SDK_VERSION.into(),
            vendor: "Hikvision".into(),
            models: vec!["terminal akses/absensi".into()],
            dll_arch: "x64 (terverifikasi header PE)".into(),
            notes: "Login/info via HCNetSDK; tarik log absensi BUTUH dokumen ISAPI/ACS + hardware."
                .into(),
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            // Jujur: belum ada jalur terdokumentasi untuk event absensi pada build ini.
            fetch_logs: Capability::Unsupported,
            fetch_users: Capability::Unsupported,
            provision_user: Capability::Unsupported,
            device_info: Capability::Supported,
            cancel_sync: Capability::Supported,
        }
    }

    fn initialize(&mut self) -> Result<(), AdapterError> {
        let lib = ensure_lib()?;
        // Verifikasi simbol inti benar-benar ada (bukan sekadar file ada).
        for name in [
            &b"NET_DVR_Init"[..],
            b"NET_DVR_Login_V30",
            b"NET_DVR_Logout",
            b"NET_DVR_Cleanup",
            b"NET_DVR_GetLastError",
        ] {
            let _: libloading::Symbol<FnInit> = unsafe { lib.get(name) }.map_err(|_| {
                AdapterError::Sdk(format!(
                    "HCNetSDK.dll tanpa {}",
                    String::from_utf8_lossy(name)
                ))
            })?;
        }
        Ok(())
    }

    fn connect(&mut self, host: &str, timeout: Duration) -> Result<(), AdapterError> {
        if host.trim().is_empty() || host.trim() == "local" {
            return Err(AdapterError::Connection(
                "host kosong: isi IP perangkat Hikvision".to_string(),
            ));
        }
        let (cfg_port, user, pass) = conn_config(&self.config_json);
        let (ip, port) = parse_host_port(host, cfg_port);
        if pass.is_empty() {
            return Err(AdapterError::Connection(
                "password kosong: isi config_json {\"username\":...,\"password\":...} pada data perangkat."
                    .to_string(),
            ));
        }
        // Uji hidup: login lalu langsung logout.
        let lib = ensure_lib()?;
        let uid = login(&lib, &ip, port, &user, &pass, timeout, None)?;
        logout_cleanup(&lib, uid);
        remember_host(host);
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    fn health_check(&mut self) -> Result<ConnStatus, AdapterError> {
        // Uji hidup nyata (login → logout). Gagal → Offline, bukan error keras.
        let host = last_host();
        if host.trim().is_empty() || host.trim() == "local" {
            return Ok(ConnStatus::Offline);
        }
        let (cfg_port, user, pass) = conn_config(&self.config_json);
        let (ip, port) = parse_host_port(&host, cfg_port);
        if pass.is_empty() {
            return Ok(ConnStatus::Offline);
        }
        let lib = match ensure_lib() {
            Ok(l) => l,
            Err(_) => return Ok(ConnStatus::Offline),
        };
        match login(&lib, &ip, port, &user, &pass, Duration::from_secs(5), None) {
            Ok(uid) => {
                logout_cleanup(&lib, uid);
                Ok(ConnStatus::Online)
            }
            Err(_) => Ok(ConnStatus::Offline),
        }
    }

    fn fetch_attendance_logs(
        &mut self,
        _checkpoint: &str,
        _limit: usize,
        _cancel: &AtomicBool,
    ) -> Result<(Vec<DeviceLog>, String), AdapterError> {
        Err(AdapterError::Unsupported(
            "tarik log absensi Hikvision belum didukung: build HCNetSDK ini tidak mengekspos \
             API event absensi terminal. Diperlukan dokumen ISAPI/ACS resmi + uji perangkat fisik. \
             Lihat docs/SDK_INVENTORY.md."
                .to_string(),
        ))
    }

    fn fetch_device_users(&mut self) -> Result<Vec<DeviceUser>, AdapterError> {
        Err(AdapterError::Unsupported(
            "daftar user Hikvision via HCNetSDK belum didukung (butuh dokumen ISAPI/ACS + hardware)."
                .to_string(),
        ))
    }

    fn provision_user(&mut self, _user: &DeviceUser) -> Result<(), AdapterError> {
        Err(AdapterError::Unsupported(
            "provision user Hikvision belum didukung (butuh dokumen ISAPI/ACS + hardware)."
                .to_string(),
        ))
    }

    fn fetch_device_info(&mut self) -> Result<serde_json::Value, AdapterError> {
        let host = last_host();
        if host.trim().is_empty() || host.trim() == "local" {
            return Err(AdapterError::Connection(
                "host kosong: connect() dulu dengan IP perangkat.".to_string(),
            ));
        }
        hikvision_device_info(&host, &self.config_json, Duration::from_secs(8))
    }

    fn get_sdk_version(&self) -> String {
        SDK_VERSION.to_string()
    }
}

/// Status DLL HCNetSDK untuk UI (`device_sdk_status`), tanpa perangkat/login.
pub(crate) fn hcnet_status() -> serde_json::Value {
    let path = find_vendor_dll("HCNetSDK.dll");
    let (found, path_str, arch) = match &path {
        None => (false, String::new(), String::new()),
        Some(p) => {
            let a = std::fs::read(p)
                .ok()
                .and_then(|b| super::zkcom::pe_arch_label(&b).map(str::to_string))
                .unwrap_or_default();
            (true, p.display().to_string(), a)
        }
    };
    let (loadable, hint) = match ensure_lib() {
        Err(e) => (false, e.to_string()),
        Ok(lib) => {
            let mut missing = Vec::new();
            for name in [
                "NET_DVR_Init",
                "NET_DVR_Login_V30",
                "NET_DVR_Logout",
                "NET_DVR_Cleanup",
                "NET_DVR_GetLastError",
            ] {
                if unsafe { lib.get::<FnInit>(name.as_bytes()) }.is_err() {
                    missing.push(name);
                }
            }
            if missing.is_empty() {
                (true, String::new())
            } else {
                (false, format!("ekspor hilang: {}", missing.join(", ")))
            }
        }
    };
    serde_json::json!({
        "adapter": "hikvision-terminal",
        "dll": "HCNetSDK.dll",
        "found": found,
        "path": path_str,
        "arch": arch,
        "arch_ok": arch == "x64",
        "loadable": loadable,
        "hint": hint,
    })
}
/// Info perangkat via sesi login eksplisit (dipakai `device_test`).
/// Dibuat terpisah agar alur connect→info→disconnect tetap dalam satu sesi SDK.
pub fn hikvision_device_info(
    host: &str,
    config_json: &str,
    timeout: Duration,
) -> Result<serde_json::Value, AdapterError> {
    let (cfg_port, user, pass) = conn_config(config_json);
    let (ip, port) = parse_host_port(host, cfg_port);
    let lib = ensure_lib()?;
    let mut info_buf = [0u8; 256];
    let uid = login(&lib, &ip, port, &user, &pass, timeout, Some(&mut info_buf))?;
    let serial = parse_serial(&info_buf);
    let sdk_code = unsafe {
        lib.get::<FnGetSdkVersion>(b"NET_DVR_GetSDKVersion")
            .map(|f| f())
            .unwrap_or(0)
    };
    logout_cleanup(&lib, uid);
    Ok(serde_json::json!({
        "serial_number": serial,
        "endpoint": format!("{ip}:{port}"),
        "sdk_version_code": format!("0x{sdk_code:08X}"),
        "sdk_version": SDK_VERSION,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conn_config_defaults() {
        let (p, u, pw) = conn_config("{}");
        assert_eq!((p, u.as_str(), pw.as_str()), (8000, "admin", ""));
        let (p2, u2, pw2) = conn_config(r#"{"port":8001,"username":"op","password":"s3cr3t"}"#);
        assert_eq!((p2, u2.as_str(), pw2.as_str()), (8001, "op", "s3cr3t"));
    }

    #[test]
    fn serial_parsing_trims_nul() {
        let mut b = [0u8; 256];
        b[..9].copy_from_slice(b"DS-K1T804");
        assert_eq!(parse_serial(&b), "DS-K1T804");
        assert_eq!(parse_serial(&[0u8; 10]), "");
    }

    #[test]
    fn unsupported_ops_are_honest() {
        let mut a = HikvisionAdapter::new("{}");
        let caps = a.capabilities();
        assert!(matches!(caps.fetch_logs, Capability::Unsupported));
        let cancel = AtomicBool::new(false);
        assert!(a.fetch_attendance_logs("", 10, &cancel).is_err());
    }

    #[test]
    fn sdk_status_shape() {
        let v = hcnet_status();
        assert_eq!(v["dll"], serde_json::json!("HCNetSDK.dll"));
        assert!(v["found"].is_boolean());
        assert!(v["loadable"].is_boolean());
        assert!(v["arch_ok"].is_boolean());
    }

    // Hardware: butuh terminal Hikvision + kredensial.
    //   set ATTENDX_TEST_HIK_HOST=192.168.1.64
    //   set ATTENDX_TEST_HIK_CONFIG={"port":8000,"username":"admin","password":"..."}
    //   cargo test -p attendx_rust_lib hik_hw -- --ignored --nocapture
    #[test]
    #[ignore]
    fn hik_hw_login_smoke() {
        let host = std::env::var("ATTENDX_TEST_HIK_HOST").expect("set host");
        let cfg = std::env::var("ATTENDX_TEST_HIK_CONFIG").expect("set config");
        let mut a = HikvisionAdapter::new(&cfg);
        a.initialize().expect("DLL ada");
        a.connect(&host, Duration::from_secs(8)).expect("login");
        let info = hikvision_device_info(&host, &cfg, Duration::from_secs(8)).expect("info");
        assert!(info.get("serial_number").is_some());
    }
}
