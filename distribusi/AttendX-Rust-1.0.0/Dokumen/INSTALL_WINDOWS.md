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

## Offline

Aplikasi 100% lokal: tidak ada panggilan jaringan, tidak ada CDN
(Tabler dibundel dari `node_modules/@tabler/core`, bukan URL eksternal).

## Troubleshooting build

- `link.exe / cl.exe tidak ditemukan` → build dari VsDevCmd (lihat langkah 2).
- `frontendDist ../dist tidak ada` → jalankan `npm run build` dulu dari root.
- `makensis tidak ditemukan` → install NSIS, pastikan `C:\Program Files (x86)\NSIS` ada.
- Download NSIS bawaan Tauri (nsis-3.11.zip, nsis_tauri_utils.dll) butuh internet sekali
  saat bundling pertama; hasil bundel tetap offline.
