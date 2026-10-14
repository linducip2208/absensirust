# AttendX Rust

Aplikasi absensi karyawan desktop **Windows 10/11 x64** — Rust + Tauri 2 + React + TypeScript + SQLite.
Beroperasi **offline**, zona waktu operasional **Asia/Jakarta (WIB)**, dengan dukungan multi-hardware
melalui trait adapter vendor.

> Kejujuran integrasi: **belum ada SDK vendor yang teruji pada hardware nyata**
> (lihat `docs/SDK_INVENTORY.md`). Adapter nyata hanya diklaim *supported* setelah pengujian
> pada perangkat fisik. Yang tersedia saat ini: interface, registry, dan mock adapter
> bertanda **SIMULATED** khusus test/demo.

## Mulai cepat

```powershell
npm install
npm run build            # frontend -> dist/
cd src-tauri
cargo test               # 33 test backend
cargo run                # mode dev (butuh WebView2, bawaan Win10/11)
npx tauri build          # produksi + installer NSIS (jalankan dari VsDevCmd x64)
```

Installer: `src-tauri/target/release/bundle/nsis/AttendX Rust_1.0.0_x64-setup.exe`

Data aplikasi: `%APPDATA%\AttendX\attendx.db` (SQLite + WAL). Backup: `%APPDATA%\AttendX\backups\`.

## Fitur

- Dashboard real-time dari query database (hadir, terlambat, tidak hadir, cuti, perangkat, grafik 7 hari, shift).
- Karyawan: CRUD, NIK unik, departemen/jabatan/cabang/atasan, status, foto opsional, pola shift,
  import CSV (preview + validasi + laporan error), export CSV, pencarian/filter/pagination/sorting,
  riwayat perubahan, soft delete, mapping ID mesin absensi.
- Organisasi: multi-cabang, departemen, jabatan, kalender kerja, libur nasional/perusahaan,
  toleransi telat, pembulatan, kebijakan lembur — semua dapat dikonfigurasi.
- Shift engine: preset Pagi 07:00–15:00, Siang 15:00–23:00, Malam 23:00–07:00 (dapat diedit);
  shift malam lintas tengah malam, grace period, telat/pulang cepat/kurang jam, rotasi,
  penugasan bertanggal efektif + riwayat versi, koreksi beraudit, hitung ulang aman.
- Jadwal & kalender: kalender mingguan, penugasan massal + salin minggu dengan
  pratinjau dampak, deteksi bentrok & jadwal kosong, tukar shift berpersetujuan.
- Pengaturan: interval sync, retensi backup, timeout sesi (berlaku segera),
  scheduler sync otomatis, sync semua perangkat.
- Perangkat: registry, tambah/edit/nonaktif, test koneksi, sync log manual, checkpoint per perangkat,
  retry + exponential backoff, idempotent ingest, deduplikasi, karantina event gagal,
  halaman review log tanpa mapping.
- Izin/cuti/lembur: jenis + kuota, pengajuan, persetujuan, lampiran (batas tipe/ukuran),
  kalender, riwayat, audit. Laporan hanya menghitung yang *approved*.
- Laporan: harian, bulanan (via filter tanggal), per karyawan/departemen/cabang/shift,
  telat, pulang cepat, lembur, tidak hadir, izin/cuti, libur, log perangkat, error sync,
  riwayat koreksi, rekonsiliasi raw vs hasil. Export CSV/XLSX/PDF (tanggal WIB).
- Keamanan: Argon2, session token + timeout 8 jam + lock otomatis (re-login),
  permission per role di backend, rate-limit login, parameter binding, audit log,
  tanpa kredensial default (admin pertama dibuat sekali saat instalasi).
- Backup/restore SQLite tervalidasi + rotasi, health check + integrity check,
  bundle diagnostik tanpa rahasia.

## Struktur

```
src/                 React + TypeScript UI (Tabler lokal via npm, tanpa CDN)
src-tauri/           Rust backend (Tauri 2 command handler, domain, DB)
  src/db/            migrasi versioned + repository
  src/attendance/    shift window + mesin hitung (murni, deterministik)
  src/devices/       trait adapter, registry, mock SIMULATED
  src/sync/          sinkronisasi (checkpoint, retry, idempotent)
  src/reports/       ekspor CSV/XLSX/PDF
  src/timex.rs       helper Asia/Jakarta
docs/                12 dokumen (lihat daftar di bawah)
```

## Dokumentasi

- `docs/ARCHITECTURE.md` — batas modul UI/domain/DB/hardware.
- `docs/INSTALL_WINDOWS.md` — instalasi & build dari source.
- `docs/USER_GUIDE.md` — manual pengguna (Bahasa Indonesia).
- `docs/SHIFT_ENGINE.md` — aturan waktu & konversi WIB.
- `docs/SDK_INVENTORY.md` — **status jujur tiap vendor** (semua BUTUH SDK/hardware).
- `docs/SDK_ADAPTER_DEVELOPMENT.md` — cara menulis adapter baru + aturan DLL 32/64-bit.
- `docs/DATABASE_SCHEMA.md` — skema & migrasi.
- `docs/BACKUP_RESTORE.md` — prosedur backup/restore.
- `docs/SECURITY.md` — model keamanan & peran.
- `docs/TEST_REPORT.md` — perintah, hasil aktual, error tersisa.
- `docs/KNOWN_LIMITATIONS.md` — keterbatasan yang diketahui.
- `docs/screenshots/` — tangkapan layar aplikasi.

## Status penerimaan

Semua kriteria yang dapat diverifikasi pada lingkungan ini **lulus** (build 6:
70 test Rust + 9 test frontend, fmt/clippy/eslint/tsc bersih, installer NSIS
terbentuk, smoke test exe + migrasi + integritas DB ok, paket `distribusi/`
disegarkan ke build 6).
Lihat `docs/TEST_REPORT.md` dan `docs/KNOWN_LIMITATIONS.md` untuk status BLOCKED
(pengujian hardware SDK nyata — membutuhkan DLL terdaftar + perangkat fisik;
cek kesiapan DLL dari UI via Devices → DLL vendor di mesin ini).
