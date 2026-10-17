//! Klien COM late-binding (IDispatch) untuk CZKEM (`zkemkeeper.dll`).
//!
//! Cakupan jujur:
//! - `zkemkeeper.dll` adalah ActiveX COM (terverifikasi: hanya mengekspor
//!   `DllGetClassObject` dkk, bukan fungsi C biasa), sehingga dipanggil lewat
//!   `CoCreateInstance` + `IDispatch::Invoke`, bukan `libloading`.
//! - CLSID coclass `CZKEM` = `{00853A19-BD51-419B-9269-2DABE57EB61F}` dan IID
//!   `IZKEM` = `{102F4206-E43D-4FC9-BAB0-331CFFE4D25B}` dibaca langsung dari
//!   typelib yang tertanam di `vendor_sdk/.../SDK/x64/zkemkeeper.dll`
//!   (SDK 6.3.1.54), bukan dari ingatan.
//! - Nama method + urutan parameter + arti `InOutMode` diverifikasi dari demo
//!   resmi (`Demo/C#/TFT/AttLogs`, `UserInfo`, `Device Management`) dan
//!   `ZKTeco_Standalone_SDK_Development_Manual_V2.2_A.3-EN.pdf`
//!   (`vendor_sdk/.../doc/`).
//! - Tiap operasi membuka sesi COM sendiri (STA di thread pemanggil) lalu
//!   menutupnya: aman dipakai dari thread pool Tauri.
//!
//! Tanpa DLL terdaftar (`Register_SDK x64.bat` sebagai Administrator) atau
//! tanpa perangkat fisik, semua operasi mengembalikan error yang jelas —
//! tidak pernah mengarang data.

use super::adapter::AdapterError;
use std::collections::HashMap;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// GUID
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

const fn guid(data1: u32, data2: u16, data3: u16, data4: [u8; 8]) -> Guid {
    Guid {
        data1,
        data2,
        data3,
        data4,
    }
}

/// CLSID coclass CZKEM — terverifikasi dari typelib SDK 6.3.1.54 x64.
const CLSID_CZKEM: Guid = guid(
    0x0085_3A19,
    0xBD51,
    0x419B,
    [0x92, 0x69, 0x2D, 0xAB, 0xE5, 0x7E, 0xB6, 0x1F],
);
const IID_IDISPATCH: Guid = guid(
    0x0002_0400,
    0x0000,
    0x0000,
    [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
);
const IID_NULL: Guid = guid(0, 0, 0, [0; 8]);

// ---------------------------------------------------------------------------
// VARIANT minimal (16 byte, ABI COM stabil)
// ---------------------------------------------------------------------------

const VT_I4: u16 = 3;
const VT_BSTR: u16 = 8;
const VT_BOOL: u16 = 11;
const VT_BYREF: u16 = 0x4000;

#[repr(C)]
struct Variant {
    vt: u16,
    reserved: [u16; 3],
    /// 8 byte data/pointer. Untuk BYREF: pointer ke slot milik pemanggil.
    payload: u64,
}

impl Variant {
    fn empty() -> Self {
        Self {
            vt: 0,
            reserved: [0; 3],
            payload: 0,
        }
    }
    fn i4(v: i32) -> Self {
        Self {
            vt: VT_I4,
            reserved: [0; 3],
            payload: v as u32 as u64,
        }
    }
    fn boolean(v: bool) -> Self {
        Self {
            vt: VT_BOOL,
            reserved: [0; 3],
            payload: if v { 0xFFFF } else { 0 },
        }
    }
    fn bstr_owned(b: *mut u16) -> Self {
        Self {
            vt: VT_BSTR,
            reserved: [0; 3],
            payload: b as u64,
        }
    }
    fn byref_i4(slot: *mut i32) -> Self {
        Self {
            vt: VT_I4 | VT_BYREF,
            reserved: [0; 3],
            payload: slot as u64,
        }
    }
    fn byref_bool(slot: *mut i16) -> Self {
        Self {
            vt: VT_BOOL | VT_BYREF,
            reserved: [0; 3],
            payload: slot as u64,
        }
    }
    fn byref_bstr(slot: *mut *mut u16) -> Self {
        Self {
            vt: VT_BSTR | VT_BYREF,
            reserved: [0; 3],
            payload: slot as u64,
        }
    }
    fn as_bool(&self) -> bool {
        (self.payload as u16 as i16) != 0
    }
}

// ---------------------------------------------------------------------------
// IDispatch + EXCEPINFO (ABI stabil)
// ---------------------------------------------------------------------------

type HResult = i32;
const S_OK: HResult = 0;
const DISPATCH_METHOD: u16 = 1;
const LOCALE_USER_DEFAULT: u32 = 0x0400;
const CLSCTX_INPROC_SERVER: u32 = 1;
const COINIT_APARTMENTTHREADED: u32 = 2;
const REGDB_E_CLASSNOTREG: HResult = 0x8004_0154u32 as i32;

#[repr(C)]
struct DispParams {
    rgvarg: *mut Variant,
    rgdispid_named_args: *mut i32,
    c_args: u32,
    c_named_args: u32,
}

#[repr(C)]
struct ExcepInfo {
    w_code: u16,
    w_reserved: u16,
    bstr_source: *mut u16,
    bstr_description: *mut u16,
    bstr_help_file: *mut u16,
    dw_help_context: u32,
    _pad: u32,
    pv_reserved: *mut c_void,
    pfn_deferred_fill_in: *mut c_void,
    scode: HResult,
    _pad2: u32,
}

impl ExcepInfo {
    fn empty() -> Self {
        Self {
            w_code: 0,
            w_reserved: 0,
            bstr_source: std::ptr::null_mut(),
            bstr_description: std::ptr::null_mut(),
            bstr_help_file: std::ptr::null_mut(),
            dw_help_context: 0,
            _pad: 0,
            pv_reserved: std::ptr::null_mut(),
            pfn_deferred_fill_in: std::ptr::null_mut(),
            scode: 0,
            _pad2: 0,
        }
    }
}

#[repr(C)]
struct IDispatch {
    vtbl: *const IDispatchVtbl,
}

#[repr(C)]
struct IDispatchVtbl {
    query_interface:
        unsafe extern "system" fn(*mut IDispatch, *const Guid, *mut *mut c_void) -> HResult,
    add_ref: unsafe extern "system" fn(*mut IDispatch) -> u32,
    release: unsafe extern "system" fn(*mut IDispatch) -> u32,
    get_type_info_count: unsafe extern "system" fn(*mut IDispatch, *mut u32) -> HResult,
    get_type_info: unsafe extern "system" fn(*mut IDispatch, u32, u32, *mut *mut c_void) -> HResult,
    get_ids_of_names: unsafe extern "system" fn(
        *mut IDispatch,
        *const Guid,
        *mut *mut u16,
        u32,
        u32,
        *mut i32,
    ) -> HResult,
    invoke: unsafe extern "system" fn(
        *mut IDispatch,
        i32,
        *const Guid,
        u32,
        u16,
        *mut DispParams,
        *mut Variant,
        *mut ExcepInfo,
        *mut u32,
    ) -> HResult,
}

#[link(name = "ole32")]
extern "system" {
    fn CoInitializeEx(pv_reserved: *mut c_void, dw_coinit: u32) -> HResult;
    fn CoUninitialize();
    fn CoCreateInstance(
        rclsid: *const Guid,
        p_unk_outer: *mut c_void,
        dw_cls_context: u32,
        riid: *const Guid,
        ppv: *mut *mut c_void,
    ) -> HResult;
}

#[link(name = "oleaut32")]
extern "system" {
    fn SysAllocString(psz: *const u16) -> *mut u16;
    fn SysFreeString(bstr: *mut u16);
    fn SysStringLen(bstr: *mut u16) -> u32;
    fn VariantInit(pvarg: *mut Variant);
    fn VariantClear(pvarg: *mut Variant) -> HResult;
}

// ---------------------------------------------------------------------------
// Util string lebar
// ---------------------------------------------------------------------------

fn to_wide_null(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

unsafe fn bstr_to_string(b: *mut u16) -> String {
    if b.is_null() {
        return String::new();
    }
    let len = SysStringLen(b) as usize;
    let slice = std::slice::from_raw_parts(b, len);
    String::from_utf16_lossy(slice)
}

/// Bebaskan VARIANT hasil Invoke (return value milik pemanggil).
/// SAFETY: hanya untuk VARIANT yang di-`VariantInit` + diisi callee.
unsafe fn clear_result(r: &Variant) {
    VariantClear(r as *const Variant as *mut Variant);
}

// ---------------------------------------------------------------------------
// Apartemen COM (RAII)
// ---------------------------------------------------------------------------

struct ComGuard {
    needs_uninit: bool,
}

impl ComGuard {
    fn enter() -> Self {
        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED) };
        // S_OK/S_FALSE: apartemen milik kita → CoUninitialize saat Drop.
        // RPC_E_CHANGED_MODE (0x80010106): thread sudah MTA milik host — lanjut
        // best-effort tanpa CoUninitialize (bukan milik kita).
        Self {
            needs_uninit: hr == S_OK || hr == 1,
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.needs_uninit {
            unsafe { CoUninitialize() };
        }
    }
}

// ---------------------------------------------------------------------------
// Pencarian DLL vendor (murni, unit-testable)
// ---------------------------------------------------------------------------

/// Direktori kandidat pencarian DLL vendor.
/// Urutan: folder exe, folder exe/sdk, folder kerja, folder data aplikasi,
/// lalu path dev `vendor_sdk/...` (tidak ada di produksi — diabaikan).
pub(crate) fn candidate_dll_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            dirs.push(d.to_path_buf());
            dirs.push(d.join("sdk"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.clone());
        dirs.push(cwd.join("vendor_sdk").join("zkteco-x64").join("lib"));
        dirs.push(
            cwd.join("vendor_sdk")
                .join("solution-6.3.1.54")
                .join("Standalone SDK-6.3.1.54")
                .join("SDK")
                .join("x64"),
        );
        dirs.push(cwd.join("vendor_sdk").join("Hikvision-Set").join("bin"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("AttendX").join("sdk"));
    }
    dirs
}

/// Cari `file_name` di direktori kandidat. Mengembalikan path pertama yang ada.
pub(crate) fn find_vendor_dll(file_name: &str) -> Option<PathBuf> {
    candidate_dll_dirs()
        .into_iter()
        .map(|d| d.join(file_name))
        .find(|p| p.is_file())
}

/// Klasifikasi arsitektur PE dari header (`MZ` + `e_lfanew` + machine).
/// Mengembalikan `Some("x64")`, `Some("x86")`, atau `None` bila tak dikenal.
pub(crate) fn pe_arch_label(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 64 || &bytes[0..2] != b"MZ" {
        return None;
    }
    let e_lfanew = u32::from_le_bytes(bytes[0x3C..0x40].try_into().ok()?) as usize;
    let m_off = e_lfanew.checked_add(4)?;
    if bytes.len() < m_off + 2 || &bytes[e_lfanew..e_lfanew + 4] != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes(bytes[m_off..m_off + 2].try_into().ok()?) {
        0x8664 => Some("x64"),
        0x14c => Some("x86"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helper parsing host/konfig (murni, unit-testable)
// ---------------------------------------------------------------------------

/// Pecah `host` menjadi (ip_atau_namahost, port).
/// Format: `"192.168.1.201"` atau `"192.168.1.201:4370"`.
pub(crate) fn parse_host_port(host: &str, default_port: u16) -> (String, u16) {
    let h = host.trim();
    if let Some(idx) = h.rfind(':') {
        let (left, right) = h.split_at(idx);
        // Abaikan ':' tunggal IPv6 sederhana? Perangkat memakai IPv4/hostname.
        if let Ok(p) = right[1..].parse::<u16>() {
            if !left.is_empty() {
                let ip = left.trim_start_matches('[').trim_end_matches(']');
                return (ip.to_string(), p);
            }
        }
    }
    (h.to_string(), default_port)
}

/// Ambil `machine_number` dari config JSON (`{"machine":1}`), default 1.
/// Nomor mesin diabaikan untuk TCP/IP menurut demo vendor (bebas, dipakai 1).
pub(crate) fn parse_machine_number(config_json: &str) -> i32 {
    serde_json::from_str::<serde_json::Value>(config_json)
        .ok()
        .and_then(|v| v.get("machine").and_then(|m| m.as_i64()))
        .filter(|m| (1..=255).contains(m))
        .unwrap_or(1) as i32
}

/// Ambil port dari config JSON (`{"port":4370}`), default diberikan.
pub(crate) fn parse_port(config_json: &str, default_port: u16) -> u16 {
    serde_json::from_str::<serde_json::Value>(config_json)
        .ok()
        .and_then(|v| v.get("port").and_then(|m| m.as_u64()))
        .filter(|p| *p >= 1 && *p <= 65535)
        .unwrap_or(u64::from(default_port)) as u16
}

/// Petakan `InOutMode` SDK (manual §5.2.1.5: 0=Check-In, 1=Check-Out,
/// 2=Break-Out, 3=Break-In, 4=OT-In, 5=OT-Out) ke direction ingest.
pub(crate) fn inout_direction(mode: i32) -> String {
    match mode {
        0 | 2 | 4 => "in".to_string(),
        1 | 3 | 5 => "out".to_string(),
        other => format!("mode-{other}"),
    }
}

/// Bentuk `vendor_record_id` deterministik untuk deduplikasi ingest.
/// Format lebar-tetap agar stabil diurutkan/dibandingkan.
pub(crate) fn build_record_id(enroll: &str, row: &LogRow) -> String {
    format!(
        "{enroll}|{:04}-{:02}-{:02} {:02}:{:02}:{:02}|{}|{}|{}",
        row.year,
        row.month,
        row.day,
        row.hour,
        row.minute,
        row.second,
        row.inout,
        row.verify,
        row.workcode
    )
}

/// Bentuk `event_time` WIB `YYYY-MM-DD HH:MM:SS`, `None` bila di luar jangkauan.
pub(crate) fn build_event_time(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
) -> Option<String> {
    if !(2000..=2100).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}"
    ))
}

// ---------------------------------------------------------------------------
// Host per-thread (trait memisah connect(host) dari operasi lain). Dulu dari
// static global: dua sync di utas berbeda (manual vs scheduler) bisa saling
// menimpa sehingga event perangkat A tercatat sebagai perangkat B.
// ---------------------------------------------------------------------------

use std::cell::RefCell;

thread_local! {
    static LAST_HOST: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Catat host terakhir saat `connect` agar fetch/info/provision tahu targetnya.
pub(crate) fn remember_host(host: &str) {
    LAST_HOST.with(|c| *c.borrow_mut() = host.to_string());
}

/// Ambil host terakhir; string kosong bila `connect` belum pernah dipanggil.
pub(crate) fn last_host() -> String {
    LAST_HOST.with(|c| c.borrow().clone())
}

// ---------------------------------------------------------------------------
// Sesi CZKEM
// ---------------------------------------------------------------------------

pub(crate) struct LogRow {
    pub enroll: String,
    pub verify: i32,
    pub inout: i32,
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    pub minute: i32,
    pub second: i32,
    pub workcode: i32,
}

pub(crate) struct UserRow {
    pub enroll: String,
    pub name: String,
}

pub(crate) struct ZkSession {
    _guard: ComGuard,
    dispatch: *mut IDispatch,
    dispids: HashMap<String, i32>,
}

impl ZkSession {
    /// Buka sesi: cek DLL → CoCreateInstance CZKEM → `Connect_Net`.
    pub(crate) fn open(ip: &str, port: u16) -> Result<Self, AdapterError> {
        if find_vendor_dll("zkemkeeper.dll").is_none() {
            let searched: Vec<String> = candidate_dll_dirs()
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            return Err(AdapterError::Connection(format!(
                "zkemkeeper.dll tidak ditemukan. Salin DLL SDK x64 (Solution/ZKTeco 6.3.1.54) \
                 ke salah satu folder ini lalu daftarkan via 'Register_SDK x64.bat' (Administrator): {}",
                searched.join("; ")
            )));
        }
        let guard = ComGuard::enter();
        let mut unk: *mut c_void = std::ptr::null_mut();
        // SAFETY: parameter valid; CLSID/IID terverifikasi dari typelib SDK.
        let hr = unsafe {
            CoCreateInstance(
                &CLSID_CZKEM,
                std::ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IDISPATCH,
                &mut unk,
            )
        };
        if hr == REGDB_E_CLASSNOTREG {
            return Err(AdapterError::Connection(
                "CZKEM belum terdaftar (CLASSNOTREG). Jalankan 'Register_SDK x64.bat' \
                 dari SDK sebagai Administrator, lalu ulangi."
                    .to_string(),
            ));
        }
        if hr != S_OK || unk.is_null() {
            return Err(AdapterError::Connection(format!(
                "CoCreateInstance CZKEM gagal (HRESULT=0x{hr:08X}). \
                 Pastikan zkemkeeper.dll x64 terdaftar."
            )));
        }
        let mut session = Self {
            _guard: guard,
            dispatch: unk as *mut IDispatch,
            dispids: HashMap::new(),
        };
        let wip = to_wide_null(ip);
        // SAFETY: BSTR dialokasikan via SysAllocString, dibebaskan via VariantClear.
        let bstr = unsafe { SysAllocString(wip.as_ptr()) };
        if bstr.is_null() {
            return Err(AdapterError::Connection("alokasi BSTR gagal".to_string()));
        }
        let mut args = [Variant::i4(i32::from(port)), Variant::bstr_owned(bstr)];
        let ok = session.call_bool("Connect_Net", &mut args)?;
        if !ok {
            let code = session.last_error_code();
            return Err(AdapterError::Connection(format!(
                "Connect_Net({ip}:{port}) ditolak perangkat (ErrorCode={code}). \
                 Periksa IP/port 4370, kabel LAN, dan PIN komunikasi perangkat."
            )));
        }
        Ok(session)
    }

    fn dispid(&mut self, name: &str) -> Result<i32, AdapterError> {
        if let Some(id) = self.dispids.get(name) {
            return Ok(*id);
        }
        let w = to_wide_null(name);
        let mut name_ptr = w.as_ptr() as *mut u16;
        let mut id: i32 = 0;
        // SAFETY: IDispatch valid selama sesi; nama null-terminated.
        let hr = unsafe {
            ((*(*self.dispatch).vtbl).get_ids_of_names)(
                self.dispatch,
                &IID_NULL,
                &mut name_ptr,
                1,
                LOCALE_USER_DEFAULT,
                &mut id,
            )
        };
        if hr != S_OK {
            return Err(AdapterError::Sdk(format!(
                "method '{name}' tidak ada di IZKEM (HRESULT=0x{hr:08X})"
            )));
        }
        self.dispids.insert(name.to_string(), id);
        Ok(id)
    }

    /// Panggil method boolean. `args` dalam urutan terbalik (terakhir dulu).
    fn invoke_raw(&mut self, name: &str, args: &mut [Variant]) -> Result<Variant, AdapterError> {
        let id = self.dispid(name)?;
        let mut params = DispParams {
            rgvarg: if args.is_empty() {
                std::ptr::null_mut()
            } else {
                args.as_mut_ptr()
            },
            rgdispid_named_args: std::ptr::null_mut(),
            c_args: args.len() as u32,
            c_named_args: 0,
        };
        let mut result = Variant::empty();
        // SAFETY: VariantInit di bawah; buffer milik stack frame ini.
        unsafe { VariantInit(&mut result) };
        let mut excep = ExcepInfo::empty();
        let mut arg_err: u32 = 0;
        // SAFETY: dispatch + buffer valid selama pemanggilan sinkron ini.
        let hr = unsafe {
            ((*(*self.dispatch).vtbl).invoke)(
                self.dispatch,
                id,
                &IID_NULL,
                LOCALE_USER_DEFAULT,
                DISPATCH_METHOD,
                &mut params,
                &mut result,
                &mut excep,
                &mut arg_err,
            )
        };
        // Bebaskan BSTR milik argumen input (bukan BYREF).
        for a in args.iter_mut() {
            if a.vt == VT_BSTR {
                unsafe {
                    VariantClear(a);
                }
            }
        }
        if hr != S_OK {
            let detail = unsafe {
                let d = bstr_to_string(excep.bstr_description);
                if !excep.bstr_description.is_null() {
                    SysFreeString(excep.bstr_description);
                }
                if !excep.bstr_source.is_null() {
                    SysFreeString(excep.bstr_source);
                }
                if !excep.bstr_help_file.is_null() {
                    SysFreeString(excep.bstr_help_file);
                }
                d
            };
            return Err(AdapterError::Sdk(format!(
                "Invoke '{name}' gagal (HRESULT=0x{hr:08X}){detail}",
                detail = if detail.is_empty() {
                    String::new()
                } else {
                    format!(": {detail}")
                }
            )));
        }
        Ok(result)
    }

    fn call_bool(&mut self, name: &str, args: &mut [Variant]) -> Result<bool, AdapterError> {
        let r = self.invoke_raw(name, args)?;
        if r.vt != VT_BOOL {
            unsafe {
                clear_result(&r);
            }
            return Err(AdapterError::Sdk(format!(
                "return '{name}' bukan BOOL (vt={})",
                r.vt
            )));
        }
        let v = r.as_bool();
        unsafe {
            clear_result(&r);
        }
        Ok(v)
    }

    fn last_error_code(&mut self) -> i32 {
        let mut code: i32 = 0;
        let mut args = [Variant::byref_i4(&mut code)];
        if let Ok(r) = self.invoke_raw("GetLastError", &mut args) {
            unsafe {
                clear_result(&r);
            }
        }
        code
    }

    /// Baca BSTR out lalu bebaskan.
    unsafe fn take_out_bstr(slot: *mut *mut u16) -> String {
        let b = *slot;
        let s = bstr_to_string(b);
        if !b.is_null() {
            SysFreeString(b);
        }
        s
    }

    pub(crate) fn read_general_log_data(&mut self, machine: i32) -> Result<(), AdapterError> {
        let mut args = [Variant::i4(machine)];
        if self.call_bool("ReadGeneralLogData", &mut args)? {
            Ok(())
        } else {
            Err(AdapterError::Sdk(format!(
                "ReadGeneralLogData gagal (ErrorCode={})",
                self.last_error_code()
            )))
        }
    }

    /// Satu baris log via `SSR_GetGeneralLogData` (demo AttLogs baris 229).
    /// `Ok(None)` = buffer habis; `Err` = error perangkat (ErrorCode != 0).
    pub(crate) fn ssr_get_general_log_data(
        &mut self,
        machine: i32,
    ) -> Result<Option<LogRow>, AdapterError> {
        let mut enroll_b: *mut u16 = std::ptr::null_mut();
        let (mut verify, mut inout) = (0i32, 0i32);
        let (mut y, mut mo, mut d, mut h, mut mi, mut s) = (0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
        let mut workcode: i32 = 0;
        // Urutan ABI (terbalik dari deklarasi): workcode, second..year, inout, verify, enroll, machine.
        let mut args = [
            Variant::byref_i4(&mut workcode),
            Variant::byref_i4(&mut s),
            Variant::byref_i4(&mut mi),
            Variant::byref_i4(&mut h),
            Variant::byref_i4(&mut d),
            Variant::byref_i4(&mut mo),
            Variant::byref_i4(&mut y),
            Variant::byref_i4(&mut inout),
            Variant::byref_i4(&mut verify),
            Variant::byref_bstr(&mut enroll_b),
            Variant::i4(machine),
        ];
        let r = self.invoke_raw("SSR_GetGeneralLogData", &mut args)?;
        let ok = if r.vt == VT_BOOL { r.as_bool() } else { false };
        unsafe {
            clear_result(&r);
        }
        let enroll = unsafe { Self::take_out_bstr(&mut enroll_b) };
        if !ok {
            let code = self.last_error_code();
            if code != 0 {
                return Err(AdapterError::Sdk(format!(
                    "SSR_GetGeneralLogData gagal (ErrorCode={code})"
                )));
            }
            return Ok(None);
        }
        Ok(Some(LogRow {
            enroll,
            verify,
            inout,
            year: y,
            month: mo,
            day: d,
            hour: h,
            minute: mi,
            second: s,
            workcode,
        }))
    }

    pub(crate) fn read_all_user_id(&mut self, machine: i32) -> Result<(), AdapterError> {
        let mut args = [Variant::i4(machine)];
        if self.call_bool("ReadAllUserID", &mut args)? {
            Ok(())
        } else {
            Err(AdapterError::Sdk(format!(
                "ReadAllUserID gagal (ErrorCode={})",
                self.last_error_code()
            )))
        }
    }

    /// Satu user via `SSR_GetAllUserInfo` (demo UserInfo baris 230).
    pub(crate) fn ssr_get_all_user_info(
        &mut self,
        machine: i32,
    ) -> Result<Option<UserRow>, AdapterError> {
        let mut enroll_b: *mut u16 = std::ptr::null_mut();
        let mut name_b: *mut u16 = std::ptr::null_mut();
        let mut pass_b: *mut u16 = std::ptr::null_mut();
        let (mut priv_level, mut enabled_i16) = (0i32, 0i16);
        let mut args = [
            Variant::byref_bool(&mut enabled_i16),
            Variant::byref_i4(&mut priv_level),
            Variant::byref_bstr(&mut pass_b),
            Variant::byref_bstr(&mut name_b),
            Variant::byref_bstr(&mut enroll_b),
            Variant::i4(machine),
        ];
        let r = self.invoke_raw("SSR_GetAllUserInfo", &mut args)?;
        let ok = if r.vt == VT_BOOL { r.as_bool() } else { false };
        unsafe {
            clear_result(&r);
        }
        let enroll = unsafe { Self::take_out_bstr(&mut enroll_b) };
        let name = unsafe { Self::take_out_bstr(&mut name_b) };
        unsafe {
            Self::take_out_bstr(&mut pass_b);
        }
        if !ok {
            let code = self.last_error_code();
            if code != 0 {
                return Err(AdapterError::Sdk(format!(
                    "SSR_GetAllUserInfo gagal (ErrorCode={code})"
                )));
            }
            return Ok(None);
        }
        Ok(Some(UserRow { enroll, name }))
    }

    pub(crate) fn ssr_set_user_info(
        &mut self,
        machine: i32,
        enroll: &str,
        name: &str,
        privilege: i32,
        enabled: bool,
    ) -> Result<(), AdapterError> {
        let w_enroll = to_wide_null(enroll);
        let w_name = to_wide_null(name);
        let w_pass = to_wide_null("");
        // SAFETY: BSTR input dibebaskan via VariantClear di invoke_raw.
        let (b_enroll, b_name, b_pass) = unsafe {
            (
                SysAllocString(w_enroll.as_ptr()),
                SysAllocString(w_name.as_ptr()),
                SysAllocString(w_pass.as_ptr()),
            )
        };
        if b_enroll.is_null() || b_name.is_null() || b_pass.is_null() {
            for b in [b_enroll, b_name, b_pass] {
                if !b.is_null() {
                    unsafe { SysFreeString(b) };
                }
            }
            return Err(AdapterError::Connection("alokasi BSTR gagal".to_string()));
        }
        // SSR_SetUserInfo(machine, enroll, name, password, privilege, enabled) — demo UserInfo baris 318.
        let mut args = [
            Variant::boolean(enabled),
            Variant::i4(privilege),
            Variant::bstr_owned(b_pass),
            Variant::bstr_owned(b_name),
            Variant::bstr_owned(b_enroll),
            Variant::i4(machine),
        ];
        if self.call_bool("SSR_SetUserInfo", &mut args)? {
            Ok(())
        } else {
            Err(AdapterError::Sdk(format!(
                "SSR_SetUserInfo gagal (ErrorCode={})",
                self.last_error_code()
            )))
        }
    }

    fn get_string_prop(&mut self, name: &str, machine: Option<i32>) -> Option<String> {
        // Varian signature terverifikasi dari demo Device Management:
        // GetSerialNumber(m, out), GetProductCode(m, out),
        // GetFirmwareVersion(m, ref), GetSDKVersion(ref), GetDeviceMAC(m, ref), GetPlatform(m, ref).
        let mut out_b: *mut u16 = std::ptr::null_mut();
        let w_empty = to_wide_null("");
        let empty_b = unsafe { SysAllocString(w_empty.as_ptr()) };
        if !empty_b.is_null() {
            // Varian `ref string`: slot menunjuk ke BSTR yang sudah dialokasikan.
            out_b = empty_b;
        }
        let mut owned: Vec<Variant>;
        let args: &mut [Variant];
        if let Some(m) = machine {
            owned = vec![Variant::byref_bstr(&mut out_b), Variant::i4(m)];
            args = owned.as_mut_slice();
        } else {
            owned = vec![Variant::byref_bstr(&mut out_b)];
            args = owned.as_mut_slice();
        }
        let r = self.invoke_raw(name, args).ok()?;
        let ok = if r.vt == VT_BOOL { r.as_bool() } else { false };
        unsafe {
            clear_result(&r);
        }
        let s = unsafe { Self::take_out_bstr(&mut out_b) };
        if ok && !s.is_empty() {
            Some(s)
        } else {
            None
        }
    }

    pub(crate) fn serial_number(&mut self, machine: i32) -> Option<String> {
        self.get_string_prop("GetSerialNumber", Some(machine))
    }
    pub(crate) fn product_code(&mut self, machine: i32) -> Option<String> {
        self.get_string_prop("GetProductCode", Some(machine))
    }
    pub(crate) fn firmware_version(&mut self, machine: i32) -> Option<String> {
        self.get_string_prop("GetFirmwareVersion", Some(machine))
    }
    pub(crate) fn sdk_version_live(&mut self) -> Option<String> {
        self.get_string_prop("GetSDKVersion", None)
    }
    pub(crate) fn device_mac(&mut self, machine: i32) -> Option<String> {
        self.get_string_prop("GetDeviceMAC", Some(machine))
    }

    pub(crate) fn record_count(&mut self, machine: i32) -> Option<i32> {
        // Demo AttLogs baris 297: GetDeviceStatus(machine, 6, ref value) = jumlah log.
        let mut value: i32 = 0;
        let mut args = [
            Variant::byref_i4(&mut value),
            Variant::i4(6),
            Variant::i4(machine),
        ];
        let r = self.invoke_raw("GetDeviceStatus", &mut args).ok()?;
        let ok = if r.vt == VT_BOOL { r.as_bool() } else { false };
        unsafe {
            clear_result(&r);
        }
        if ok {
            Some(value)
        } else {
            None
        }
    }

    pub(crate) fn disconnect_device(&mut self) {
        let mut no_args: [Variant; 0] = [];
        let _ = self.invoke_raw("Disconnect", &mut no_args);
    }

    /// Uji registrasi COM saja (tanpa konek perangkat): buat instance lalu lepas.
    pub(crate) fn probe_registered() -> Result<(), AdapterError> {
        if find_vendor_dll("zkemkeeper.dll").is_none() {
            return Err(AdapterError::Connection(
                "zkemkeeper.dll tidak ditemukan di folder exe / exe/sdk / %APPDATA%\\AttendX\\sdk."
                    .to_string(),
            ));
        }
        let guard = ComGuard::enter();
        let mut unk: *mut c_void = std::ptr::null_mut();
        // SAFETY: parameter valid; CLSID/IID terverifikasi dari typelib SDK.
        let hr = unsafe {
            CoCreateInstance(
                &CLSID_CZKEM,
                std::ptr::null_mut(),
                CLSCTX_INPROC_SERVER,
                &IID_IDISPATCH,
                &mut unk,
            )
        };
        if hr == REGDB_E_CLASSNOTREG {
            return Err(AdapterError::Connection(
                "CZKEM belum terdaftar. Jalankan 'Register_SDK x64.bat' sebagai Administrator."
                    .to_string(),
            ));
        }
        if hr != S_OK || unk.is_null() {
            return Err(AdapterError::Connection(format!(
                "CoCreateInstance CZKEM gagal (HRESULT=0x{hr:08X})"
            )));
        }
        // SAFETY: instance baru saja dibuat; lepas segera.
        unsafe {
            let disp = unk as *mut IDispatch;
            ((*(*disp).vtbl).release)(disp);
        }
        drop(guard);
        Ok(())
    }
}

/// Status DLL + registrasi COM untuk UI (`device_sdk_status`), tanpa perangkat.
pub(crate) fn zkemkeeper_status() -> serde_json::Value {
    let path = find_vendor_dll("zkemkeeper.dll");
    let (found, path_str, arch) = match &path {
        None => (false, String::new(), String::new()),
        Some(p) => {
            let a = std::fs::read(p)
                .ok()
                .and_then(|b| pe_arch_label(&b).map(str::to_string))
                .unwrap_or_default();
            (true, p.display().to_string(), a)
        }
    };
    let (registered, hint) = match ZkSession::probe_registered() {
        Ok(()) => (true, String::new()),
        Err(e) => (false, e.to_string()),
    };
    serde_json::json!({
        "adapter": "zkteco-terminal + solution-terminal",
        "dll": "zkemkeeper.dll",
        "found": found,
        "path": path_str,
        "arch": arch,
        "arch_ok": arch == "x64",
        "registered": registered,
        "hint": hint,
    })
}

impl Drop for ZkSession {
    fn drop(&mut self) {
        if !self.dispatch.is_null() {
            unsafe {
                self.disconnect_device();
                ((*(*self.dispatch).vtbl).release)(self.dispatch);
            }
            self.dispatch = std::ptr::null_mut();
        }
    }
}

// Allow moving the session between threads only within one thread of use;
// the raw pointer is never shared. Marking !Send would break the adapter
// trait, so sessions are always created AND destroyed inside one method call.
unsafe impl Send for ZkSession {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clsid_czkem_matches_verified_typelib() {
        // Nilai ini dibaca dari typelib SDK 6.3.1.54 x64 (lihat modul doc).
        assert_eq!(CLSID_CZKEM.data1, 0x0085_3A19);
        assert_eq!(CLSID_CZKEM.data2, 0xBD51);
        assert_eq!(CLSID_CZKEM.data3, 0x419B);
        assert_eq!(
            CLSID_CZKEM.data4,
            [0x92, 0x69, 0x2D, 0xAB, 0xE5, 0x7E, 0xB6, 0x1F]
        );
    }

    #[test]
    fn parse_host_port_cases() {
        assert_eq!(
            parse_host_port("192.168.1.201", 4370),
            ("192.168.1.201".into(), 4370)
        );
        assert_eq!(
            parse_host_port("192.168.1.201:4370", 4370),
            ("192.168.1.201".into(), 4370)
        );
        assert_eq!(
            parse_host_port("host.local:8080", 4370),
            ("host.local".into(), 8080)
        );
        assert_eq!(
            parse_host_port("  10.0.0.5  ", 8000),
            ("10.0.0.5".into(), 8000)
        );
        // Port non-numerik → pakai default, host utuh.
        assert_eq!(
            parse_host_port("weird:abc", 4370),
            ("weird:abc".into(), 4370)
        );
    }

    #[test]
    fn parse_machine_and_port() {
        assert_eq!(parse_machine_number("{}"), 1);
        assert_eq!(parse_machine_number(r#"{"machine":5}"#), 5);
        assert_eq!(parse_machine_number(r#"{"machine":999}"#), 1);
        assert_eq!(parse_port(r#"{"port":8000}"#, 4370), 8000);
        assert_eq!(parse_port("bukan json", 4370), 4370);
    }

    #[test]
    fn inout_mode_mapping_matches_manual() {
        assert_eq!(inout_direction(0), "in");
        assert_eq!(inout_direction(1), "out");
        assert_eq!(inout_direction(2), "in");
        assert_eq!(inout_direction(3), "out");
        assert_eq!(inout_direction(4), "in");
        assert_eq!(inout_direction(5), "out");
        assert_eq!(inout_direction(255), "mode-255");
    }

    #[test]
    fn record_id_and_event_time() {
        let row = LogRow {
            enroll: "1001".into(),
            verify: 1,
            inout: 0,
            year: 2026,
            month: 10,
            day: 13,
            hour: 7,
            minute: 5,
            second: 9,
            workcode: 0,
        };
        let id = build_record_id(&row.enroll.clone(), &row);
        assert_eq!(id, "1001|2026-10-13 07:05:09|0|1|0");
        assert_eq!(
            build_event_time(2026, 10, 13, 7, 5, 9).unwrap(),
            "2026-10-13 07:05:09"
        );
        assert!(build_event_time(1999, 1, 1, 0, 0, 0).is_none());
        assert!(build_event_time(2026, 13, 1, 0, 0, 0).is_none());
    }

    #[test]
    fn pe_arch_label_parses_minimal_headers() {
        fn minimal_pe(machine: u16) -> Vec<u8> {
            let mut b = vec![0u8; 64];
            b[0] = b'M';
            b[1] = b'Z';
            b[0x3C..0x40].copy_from_slice(&32u32.to_le_bytes());
            b.resize(64, 0);
            b[32..36].copy_from_slice(b"PE\0\0");
            b[36..38].copy_from_slice(&machine.to_le_bytes());
            b
        }
        assert_eq!(pe_arch_label(&minimal_pe(0x8664)), Some("x64"));
        assert_eq!(pe_arch_label(&minimal_pe(0x14c)), Some("x86"));
        assert_eq!(pe_arch_label(b"ELF-not-pe"), None);
        assert_eq!(pe_arch_label(&[]), None);
    }

    #[test]
    fn variant_layout_is_16_bytes() {
        assert_eq!(std::mem::size_of::<Variant>(), 16);
        assert_eq!(std::mem::size_of::<ExcepInfo>(), 64);
    }

    #[test]
    fn sdk_status_shape() {
        let v = zkemkeeper_status();
        assert_eq!(v["dll"], serde_json::json!("zkemkeeper.dll"));
        assert!(v["found"].is_boolean());
        assert!(v["registered"].is_boolean());
        assert!(v["arch_ok"].is_boolean());
        assert!(v["hint"].is_string());
    }

    // Integrasi hardware: hanya jalan bila env diset + perangkat fisik ada.
    //   set ATTENDX_TEST_ZK_HOST=192.168.1.201[:4370]
    //   cargo test -p attendx_rust_lib zkcom_hw -- --ignored --nocapture
    #[test]
    #[ignore]
    fn zkcom_hw_connect_and_info() {
        let host = std::env::var("ATTENDX_TEST_ZK_HOST").expect("set ATTENDX_TEST_ZK_HOST");
        let (ip, port) = parse_host_port(&host, 4370);
        let mut s = ZkSession::open(&ip, port).expect("connect hardware");
        let serial = s.serial_number(1);
        assert!(serial.is_some(), "serial number terbaca");
        let _ = s.record_count(1);
    }
}
