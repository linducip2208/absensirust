# Instalasi & Build (Windows 10/11 x64)

## Prasyarat

| Kebutuhan | Versi teruji | Keterangan |
|---|---|---|
| Windows 10/11 x64 | 11 build 26200 | target utama |
| Rust stable MSVC | 1.99.0 | via `winget install Rustlang.Rustup`, lalu `rustup toolchain install stable` |
| MSVC Build Tools | VS 18 (2026) | `cl.exe` x64 — untuk linking; build via VsDevCmd |
| Node.js | 24.15.0 | `npm install`, `npm run build` |
| WebView2 | bawaan Win10/11 | runtime UI |
| NSIS | 3.13 | `winget install NSIS.NSIS` — untuk installer |

## Build dari source

```powershell
# 1. Frontend
npm install
npm run build        # tsc --noEmit + vite build -> dist/

# 2. Backend (dari VsDevCmd x64 agar linker MSVC tersedia)
call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd src-tauri
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build          # debug; atau via Tauri di bawah

# 3. Aplikasi + installer (dari root repo, dalam VsDevCmd)
npx tauri build
```

Hasil:

- `src-tauri/target/release/attendx-rust.exe` (~13 MB)
- `src-tauri/target/release/bundle/nsis/AttendX Rust_1.0.0_x64-setup.exe` (~3,3 MB)

## Instalasi pengguna

1. Jalankan `AttendX Rust_1.0.0_x64-setup.exe`, ikuti wizard NSIS.
2. Jalankan AttendX Rust dari Start Menu.
3. **Setup pertama**: buat admin pertama (username + password min. 8 karakter).
   Tidak ada kredensial default. Proses ini hanya bisa dilakukan sekali
   (selama tabel `users` kosong).
4. Database dibuat otomatis di `%APPDATA%\AttendX\attendx.db`.

## SDK vendor: unduh & taruh (untuk koneksi mesin absensi)

Tanpa langkah ini, menu Devices hanya bisa memakai mock SIMULATED; test
koneksi hardware akan gagal dengan pesan yang jelas (bukan data karangan).

```powershell
# Dari root repo — unduh langsung semua SDK (git + curl, idempoten):
powershell -ExecutionPolicy Bypass -File vendor_sdk/download.ps1

# Taruh DLL runtime ke src-tauri/sdk/ (+ target/debug|release bila ada),
# lengkap dengan MANIFEST.json hasil verifikasi arsitektur x64:
powershell -ExecutionPolicy Bypass -File vendor_sdk/stage.ps1
```

**Di mesin kasir** (setelah install aplikasi):

1. Salin isi `src-tauri/sdk/` ke folder exe
   (`C:\Program Files\AttendX Rust\`) atau subfolder `sdk\`,
   atau ke `%APPDATA%\AttendX\sdk\`.
2. Khusus ZKTeco/Solution — daftarkan COM sekali sebagai Administrator:
   `regsvr32 zkemkeeper.dll` (atau `Register_SDK x64.bat` dari paket SDK).
   Hikvision (`HCNetSDK.dll`) tidak perlu registrasi.
3. Di aplikasi: isi IP di kolom host perangkat; untuk ZK
   `config_json` opsional `{"port":4370,"machine":1}`, untuk Hikvision wajib
   `{"port":8000,"username":"admin","password":"..."}`.
4. Verifikasi lewat command `device_sdk_status` (DLL ketemu? x64?
   terdaftar/termuat?) lalu `Test Koneksi` di menu Devices.

> **Lisensi**: paket SDK tidak mencantumkan izin redistribusi eksplisit
> (manual ZKTeco © All rights reserved; DLL Hikvision milik Hikvision).
> Karena itu installer **tidak** membundel DLL. Membundel = keputusan +
> tanggung jawab distributor (minta izin tertulis vendor). Untuk membundel
> setelah izin ada: jalankan `stage.ps1`, tambahkan `"resources": ["sdk"]`
> ke `tauri.conf.json` → `bundle.resources`, lalu `npx tauri build`.

## Offline

Aplikasi 100% lokal: tidak ada panggilan jaringan, tidak ada CDN
(Tabler dibundel dari `node_modules/@tabler/core`, bukan URL eksternal).

## Troubleshooting build

- `link.exe / cl.exe tidak ditemukan` → build dari VsDevCmd (lihat langkah 2).
- `frontendDist ../dist tidak ada` → jalankan `npm run build` dulu dari root.
- `makensis tidak ditemukan` → install NSIS, pastikan `C:\Program Files (x86)\NSIS` ada.
- Download NSIS bawaan Tauri (nsis-3.11.zip, nsis_tauri_utils.dll) butuh internet sekali
  saat bundling pertama; hasil bundel tetap offline.
