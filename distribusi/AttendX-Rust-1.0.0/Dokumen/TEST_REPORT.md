# Laporan Pengujian (hasil aktual — build 2: 2026-10-14, Windows 11 x64)

## 1. Rust backend

| Perintah | Hasil |
|---|---|
| `cargo fmt --all --check` | **LULUS** (0 diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **LULUS** (0 warning) |
| `cargo test` | **LULUS — 37/37** (33 sebelumnya + 4 baru, lihat bawah), 0 gagal |
| `cargo check --all-targets` | LULUS |

### 4 test baru (build 2)

- `commands::tests::parse_date_validates_format` — ok.
- `commands::tests::effective_shift_picks_latest` ( histori rotasi: penugasan
  terbaru menang per tanggal) — ok.
- `commands::tests::store_upload_rejects_bad_file` (file hilang/ekstensi salah/
  oversize ditolak; file valid tersalin) — ok.
- `commands::tests::setting_keys_seeded_by_migration` (migrasi v7 seed
  `app_settings`) — ok.
- Test migrasi diperketat: `migrations_apply_on_empty` kini menuntut ≥ 7.
- Test PDF diperketat: memeriksa baris "Jumlah baris" + waktu pembuatan.

## 1. Rust backend

| Perintah | Hasil |
|---|---|
| `cargo fmt --all --check` | **LULUS** (0 diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **LULUS** (0 warning; `too_many_arguments` hanya di-allow pada mapper baris DB, konstruktor hasil, dan tanda tangan IPC dengan alasan tercatat) |
| `cargo test` | **LULUS — 33/33** (lihat rincian di bawah), 0 gagal |
| `cargo check --all-targets` | LULUS (setelah `dist/` ada; sebelum frontend dibangun, target bin gagal wajar karena `frontendDist` belum ada) |

### Rincian 33 test (nama — status)

timex: `wib_is_utc_plus_7` — ok.
engine (14): `pagi_tepat_waktu`, `grace_period_10_menit`, `terlambat_11_menit`,
`pulang_cepat`, `malam_lintas_tengah_malam`, `malam_terlambat_dan_clockout_besok`,
`window_malam_milik_work_date_asal`, `grouping_malam_menarik_event_setelah_tengah_malam`,
`incomplete_tanpa_clockout`, `lembur_butuh_approval`, `libur_tanpa_event`,
`cuti_menang_atas_event`, `shift_berdekatan_pagi_siang`,
`pembulatan_dapat_dikonfigurasi` — semua ok.
auth (4): `hash_verify_roundtrip`, `viewer_cannot_manage_users`,
`super_admin_wildcard`, `weak_password_rejected` — ok.
devices (3): `backoff_grows_exponentially`, `mock_checkpoint_pagination`,
`mock_cancel` — ok.
sync (2): `sync_inserts_and_checkpoints`, `sync_retry_lalu_gagal_tertulis` — ok.
repo (3): `migrations_apply_on_empty`, `ingest_is_idempotent`,
`ingest_rejects_bad_time` — ok.
reports/export (3): `csv_export_ok`, `xlsx_export_ok`, `pdf_export_valid_header` — ok.
backup (2): `backup_restore_roundtrip`, `restore_rejects_invalid_file` — ok.
scheduler (1): `scheduler_ticks_and_stops` — ok.

Cakupan kriteria wajib: shift pagi/siang/malam ✓, lintas tengah malam ✓,
lintas tanggal ✓, shift berdekatan ✓, grace ✓, telat ✓, pulang cepat ✓,
lembur+approval ✓, libur ✓, rotasi/efektif-tanggal (via assignment query +
recompute upsert; rotasi historis dilindungi `shift_history`) ✓,
duplikat ✓, tanpa mapping ✓, retry+checkpoint ✓, koreksi+audit
(append-only, diuji via skema + API) ✓, permission peran ✓,
backup/restore ✓, import/export (CSV preview/commit + XLSX/PDF writer) ✓,
migrasi ✓, offline startup (smoke test exe → DB dibuat + 24 tabel) ✓.

## 2. Frontend

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` | **LULUS** |
| `npx eslint "src/**/*.ts" "src/**/*.tsx" --max-warnings 0` | **LULUS** (0 error/warning) |
| `npx vitest run` | **LULUS — 3/3** |
| `npm run build` | **LULUS** → `dist/` (Tabler CSS lokal, tanpa CDN; + `@tauri-apps/plugin-dialog`) |

## 3. Build produksi Windows (build 2: 2026-10-14)

| Perintah | Hasil |
|---|---|
| `npx tauri build` (VsDevCmd x64, Rust 1.99, NSIS 3.13) | **LULUS** — release dalam ~60 dtk (inkremental; + `tauri-plugin-dialog`) |
| `src-tauri/target/release/attendx-rust.exe` (14,0 MB) | ada; smoke test: berjalan >10 dtk pada DB produksi lama → migrasi v7 teraplikasi otomatis (26 tabel, v1–v7, `integrity_check = ok`) |
| `src-tauri/target/release/bundle/nsis/AttendX Rust_1.0.0_x64-setup.exe` (3,39 MB) | ada |
| Build 1 (2026-10-13, arsip): exe 13,3 MB + installer 3,26 MB, DB 24 tabel v1–v6 | LULUS saat itu |

## 4. Error yang ditemukan & diperbaiki selama pekerjaan

1. `tauri.conf.json` butuh `icons/icon.ico` → ikon placeholder lokal digenerate (PNG+ICO, tanpa aset eksternal).
2. `#[tauri::command]` di crate root bentrok `#[macro_export]` (E0255) → command dipindah ke `mod commands`.
3. `rust_xlsxwriter 0.84`: `save_to_vec` tidak ada → dipakai `save_to_buffer`.
4. Query dinamis butuh binding aman → `Repo::query_strings` + `params_from_iter`.
5. Clippy 16 warning → diperbaiki semua (rename `from_code`, hapus borrow/clone/format tak perlu, struct `ReportFilter`, allow beralasan).
6. ESLint butuh parser TS → tambah `@typescript-eslint/*`, hapus disable-comment plugin yang tak terpasang.
7. `cargo` tak ada di PATH sesi VsDevCmd → `set PATH=%USERPROFILE%\.cargo\bin` di skrip build.
8. Insert blok command baru menduplikasi ekor `attendance_correct` (kurung kurawal
   berlebih) → duplikat dibuang, diverifikasi dengan skrip keseimbangan kurung + `cargo test`.
9. `Map::insert` butuh `String` eksplisit untuk kunci kedua di `settings_get` → diperbaiki.

## 5. Error / status tersisa (BLOCKED — bukan kegagalan kode)

- **Pengujian SDK nyata (TIME TECH AC6, ZKTeco, Hikvision, Solution): BLOCKED** —
  file SDK + perangkat fisik belum tersedia; tidak ada klaim kompatibilitas.
- **Installer diuji ada & terbentuk**; instalasi wizard end-to-end di mesin bersih
  belum dijalankan (disarankan sebelum distribusi).
- Screenshot UI di `docs/screenshots/` belum diambil dari sesi interaktif.
