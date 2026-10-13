# Vendor SDK — sumber, versi & status verifikasi (per 2026-10-14)

Folder ini berisi SDK asli vendor untuk implementasi adapter nyata
(`src-tauri/src/devices/`). Semua klaim di bawah terverifikasi dari file
itu sendiri (header PE, typelib COM, demo, manual) — bukan dari ingatan.

## Isi

| Folder/file | Vendor | Versi | Arsitektur | Sumber |
|---|---|---|---|---|
| `ZKTeco-Standalone-SDK/` | ZKTeco | 6.2.4.11 | x86 (13 DLL, terverifikasi PE) | `github.com/ZKTeco/Standalone-SDK` (resmi) |
| `zkteco-x64/` (+`.nupkg`) | ZKTeco | PullSDK 6.3.1.37 | x64 (15 DLL, terverifikasi PE) | `nuget.org/packages/ZKTecoStandAlonePullSDKx64` |
| `solution-6.3.1.54/` (+`.zip`) | Solution (= platform ZKEM ZKTeco) | 6.3.1.54 | x64 (25 DLL) + x86 (25 DLL), terverifikasi PE | mirror `sol-download-center.blogspot.com` → MediaFire (link resmi support Solution) |
| `solution-6.3.1.54/.../doc/` | ZKTeco/Solution | Manual V2.2_A.3-EN (267 hlm PDF+DOCX) + demo C#/VB | — | sama dengan di atas |
| `Hikvision-Set/` | Hikvision | HCNetSDK 6.1.6.3 + `CHCNetSDK.cs` (809KB wrapper) | x64 (`HCNetSDK.dll` 842 ekspor, terverifikasi PE) | `github.com/jussivirkkala/Hikvision-Set` (cermin komunitas; DLL = biner resmi) |
| `izkem_methods.txt` | — | 327 nama method `IZKEM` hasil ekstraksi typelib `zkemkeeper.dll` x64 | — | digenerate (`comtypes`), untuk referensi dev |
| `manual_full.txt` | — | teks hasil ekstraksi manual PDF (267 hlm) | — | digenerate (`pypdf`), untuk referensi dev |

Tidak ada SDK PC untuk **TIME TECH AC6** (access controller Wiegand-only
menurut distributor resmi `taruna-jaya.com`) — adapter-nya stub `Unsupported`.

## Fakta kunci hasil verifikasi

- `zkemkeeper.dll` = **COM ActiveX murni** (hanya ekspor `DllGetClassObject`
  dkk). Dari typelib-nya: coclass `CZKEM` = `{00853A19-BD51-419B-9269-2DABE57EB61F}`,
  `IZKEM` = `{102F4206-E43D-4FC9-BAB0-331CFFE4D25B}` (339 fungsi).
- `plcommpro.dll` (Pull SDK) = fungsi C biasa (`Connect`, `GetDeviceData`,
  …) tetapi **tanpa dokumen signature C** di paket ini → TIDAK dipakai
  (dilarang mengarang signature). Adapter memakai jalur COM terdokumentasi.
- `HCNetSDK.dll` = fungsi C `__stdcall` (`NET_DVR_Init`, `NET_DVR_Login_V30`,
  … sesuai `CHCNetSDK.cs`). Build ini **tidak** mengekspos API event absensi
  terminal (AcsEvent/ISAPI) → `fetch_logs` Hikvision = `Unsupported` jujur.
- `InOutMode` (manual §5.2.1.5): 0=Check-In, 1=Check-Out, 2=Break-Out,
  3=Break-In, 4=OT-In, 5=OT-Out → dipetakan ke `in`/`out`.

## Lisensi & redistribusi (hati-hati)

- SDK ZKTeco/Solution: gratis untuk customer; **izin redistribusi DLL di
  installer belum terverifikasi** → aplikasi TIDAK membundel DLL. Pengguna
  menyalin `SDK/x64/*.dll` ke folder exe (atau `%APPDATA%\AttendX\sdk`) lalu
  menjalankan `Register_SDK x64.bat` sebagai Administrator.
- HCNetSDK: unduh paket resmi di `hikvision.com` (butuh akun) untuk produksi;
  DLL di sini untuk dev/test. Salin `HCNetSDK.dll` + `HCCore.dll` +
  `libeay32.dll` + `ssleay32.dll` + folder `HCNetSDKCom/` ke folder exe.

## Unduh ulang (bila folder ini diabaikan git)

Biner (`*.zip`, `*.dll`, `*.exe`, `*.pdf`, `*.docx`) diabaikan git — hanya
file teks kecil yang ikut repo. Untuk merekonstruksi, satu perintah:

```powershell
powershell -ExecutionPolicy Bypass -File vendor_sdk/download.ps1
```

Skrip di atas idempoten (melewati yang sudah ada) dan mengunduh:
GitHub ZKTeco resmi → NuGet x64 → Solution/MediaFire (link langsung di-parse
ulang dari halaman support karena kedaluwarsa) → cermin Hikvision-Set.

## Taruh DLL runtime (staging)

```powershell
powershell -ExecutionPolicy Bypass -File vendor_sdk/stage.ps1
```

Menyalin 25 DLL ZK x64 + 4 DLL Hikvision x64 + `HCNetSDKCom/` ke
`src-tauri/sdk/` (+ `target/debug|release/sdk` bila ada), lengkap
`MANIFEST.json` (nama/ukuran/arsitektur PE terverifikasi) dan `README.txt`.
Aplikasi mencari DLL di: folder exe, folder exe/`sdk`,
`%APPDATA%\AttendX\sdk`. Cek ketersediaan dari UI via command
`device_sdk_status` (ketemu? x64? terdaftar/termuat?) — tanpa perangkat.

## Uji hardware (wajib sebelum klaim "supported")

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
# ZKTeco/Solution:
$env:ATTENDX_TEST_ZK_HOST = "192.168.1.201:4370"
cargo test -p attendx_rust_lib zkcom_hw -- --ignored --nocapture
cargo test -p attendx_rust_lib zkteco_hw -- --ignored --nocapture
# Hikvision:
$env:ATTENDX_TEST_HIK_HOST = "192.168.1.64"
$env:ATTENDX_TEST_HIK_CONFIG = '{"port":8000,"username":"admin","password":"..."}'
cargo test -p attendx_rust_lib hik_hw -- --ignored --nocapture
```
