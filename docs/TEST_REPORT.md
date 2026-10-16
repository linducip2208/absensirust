# Laporan Pengujian (hasil aktual — build 8: 2026-10-14, Windows 11 x64)

## 0. Build 8 — sapu bersih terakhir (tanpa `confirm()` native, anti-panic data korup)

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` / `eslint` / `vitest run` | **LULUS** — 9/9 |
| `npm run build` + `npx tauri build` (NSIS) | **LULUS** — exe 14,38 MB + setup 3,48 MB |
| `cargo fmt` / `clippy -D warnings` / `cargo test` | **LULUS — 74/74 lib**, 4 `#[ignore]` hardware |
| Smoke test 15 dtk + DB | **LULUS** — integrity ok, 26 tabel |

### Temuan & perbaikan (build 8)

1. `confirm()` native terakhir (Absensi → Kembalikan ke antrean) → `ConfirmDialog` Tabler.
2. Data shift korup di DB lama (`25:00`) membuat mesin hitung panic via
   `and_hms_opt().unwrap()` — kini `shift_from_value` memvalidasi (`valid_hm`)
   dan mengembalikan error ensin yang bisa dibaca + diperbaiki via menu Shift
   (+1 test; validasi tulis sudah ada sejak build 6).

## 0. Build 7 — kelengkapan putaran akhir (roundtrip + search + panduan)

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` / `eslint` / `vitest run` | **LULUS** — 9/9 |
| `npm run build` + `npx tauri build` (NSIS) | **LULUS** — exe 14,38 MB + setup 3,48 MB |
| `cargo fmt` / `clippy -D warnings` / `cargo test` | **LULUS — 73/73 lib**, 4 `#[ignore]` hardware |
| Smoke test 15 dtk + DB | **LULUS** — integrity ok, 26 tabel |

### Temuan & perbaikan (build 7)

1. **Roundtrip ekspor→impor jadwal rusak**: ekspor menulis
   `nik,nama,shift,efektif,akhir` tetapi impor membaca posisi tetap
   (kolom `nama` dibaca sebagai kode shift → semua baris gagal). Kini kolom
   dipetakan **by nama header** (alias id/en didukung) + fallback posisional
   tanpa header; logika preview diekstrak agar teruji (+3 test).
2. **Pencarian global kehilangan konteks**: klik hasil hanya pindah menu.
   Kini query dibawa via state ke Karyawan/Perangkat (`Crud.initialSearch`).
3. **USER_GUIDE**: shift malam (tanggal kerja asal), tanggal akhir,
   salin minggu, PDF multi-halaman, roundtrip impor jadwal.

## 0. Build 6 — aplikasi berjalan lengkap (kontrak IPC + keamanan + distribusi)

Latar: audit menemukan argumen IPC frontend camelCase vs backend snake_case
(Tauri mencocokkan by name) sehingga hampir semua CRUD/sync/laporan gagal —
inilah penyebab "belum berjalan lengkap". Semua diperbaiki + dikunci contract test.

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` | **LULUS** |
| `npx eslint "src/**/*.ts" "src/**/*.tsx" --max-warnings 0` | **LULUS** (0 warning) |
| `npx vitest run` | **LULUS — 9/9** (3 lama + 6 kontrak IPC snake_case) |
| `npm run build` | **LULUS** → `dist/` |
| `cargo fmt --all --check` | **LULUS** (0 diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **LULUS** (0 warning) |
| `cargo test` | **LULUS — 70/70 lib**, 0 gagal, 4 `#[ignore]` (gerbang hardware) |
| `npx tauri build` (NSIS) | **LULUS** — exe 14,38 MB + setup 3,48 MB |
| Smoke test exe 15 dtk + DB | **LULUS** — hidup, integrity ok, 26 tabel, migrasi DB lama aman |

### Bug nyata diperbaiki (build 6)

1. **Kontrak IPC**: 40+ argumen `api.ts` ke snake_case + 6 grup contract test.
2. **Keamanan**: guard hapus users (super_admin, bukan diri sendiri/admin
   terakhir), blok hapus `audit_log`/`app_settings`, `validate_required` &&→||
   (kolom wajib kosong kini ditolak), `password` virtual di update (hash Argon2),
   validasi jam shift HH:MM (anti-panic engine), validasi `end_date`/role.
3. **Integritas data**: `''`→NULL/default saat create/update (FK/numerik/
   `end_date IS NULL` aman), `end_date` bisa dikosongkan (NULL eksplisit),
   `schedule_bulk` tolak duplikat range + cek karyawan, `schedule_copy_week`
   boleh mundur + hanya salin baris minggu sumber + cek overlap,
   `schedule_import_commit` verifikasi ulang ID + cek overlap + validasi end.
4. **Hitung absensi**: permission tulis (viewer/operator/supervisor tak bisa
   hitung), libur/weekday/Izin (IZIN) dihitung, event luar window tak ditandai
   processed, waktu korup dilewati + dihitung, tanpa jadwal → dilewati (tak
   dikarang), koreksi tervalidasi + NULL aman, dashboard tak hitung libur
   sebagai mangkir + COALESCE + validasi tanggal.
5. **Impor karyawan**: validasi tanggal/email/duplikat NIK (file+DB),
   hire kosong → hari ini WIB, batas 200 baris diberi tahu.
6. **Backup/restore**: checkpoint WAL, validasi integrity read-only, path
   wajib dalam folder backups, reopen koneksi (tanpa restart), hapus WAL basi.
7. **Upload**: magic bytes PNG/JPG/PDF, respons nama file (bukan path absolut),
   WIB konsisten.
8. **Lainnya**: search ESCAPE + min 2 chars, requeue batas 62 hari + cek rentang,
   saldo cuti butuh leave.view (supervisor dapat leave.view+employee.view),
   disconnect selalu dipanggil, rate-limit case-insensitive + purge,
   purge sesi kedaluwarsa, mutex anti double-sync, label role Indonesia,
   pencarian min 2 chars + empty state.
9. **Distribusi**: `distribusi/` disegarkan ke build 6 (exe+installer+dokumen),
   panduan DLL per vendor, VERSI/BACA-DULU baru.

### Test baru (build 6): 70 lib + 9 frontend

Backend +5: `validate_required_rejects_empty`, `insert_parts_skips_empty_and_stamps_time`,
`update_parts_password_and_null`, `guard_user_delete_protects_self_and_last_admin`,
`table_columns_have_no_duplicate_tables`. Frontend +6 grup kontrak IPC.

## 0. Build 5 — finalisasi total (semua perintah benar-benar dijalankan)

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` | **LULUS** |
| `npx eslint "src/**/*.ts" "src/**/*.tsx" --max-warnings 0` | **LULUS** (0 warning) |
| `npx vitest run` | **LULUS — 3/3** |
| `npm run build` | **LULUS** → `dist/` |
| `cargo fmt --all --check` | **LULUS** (0 diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **LULUS** (0 warning) |
| `cargo test` | **LULUS — 65/65 lib**, 0 gagal, 4 `#[ignore]` (gerbang hardware) |
| `cargo check --all-targets` | LULUS (via clippy/build) |
| `npx tauri build` (NSIS) | **LULUS** — exe 14,33 MB + `AttendX Rust_1.0.0_x64-setup.exe` 3,47 MB |
| Smoke test exe 15 dtk | **LULUS** — proses hidup, `integrity_check = ok`, 26 tabel, shift seed = 3, users = 0 (tanpa kredensial default) |
| Validasi PDF via pypdf | **LULUS** — 120 baris → 3 halaman, xref utuh, header berulang, nomor halaman, ringkasan |

### Bug nyata yang ditemukan & diperbaiki (build 5)

1. **KRITIS — form setup/login tertukar**: `hasAdmin().then(setNeedsSetup)` terbalik
   (`src/pages/Login.tsx`) + argumen IPC `displayName` vs `display_name`
   (`src/api.ts`) → admin pertama tak bisa dibuat dari UI. Keduanya diperbaiki.
2. **Kebocoran `password_hash`** via `table_list`/`table_get` users (komentar
   mengklaim disembunyikan, kode tidak) → kini di-strip di backend.
3. **Ganti password tidak berfungsi** (`table_update` mengabaikan `password`)
   → kini di-hash Argon2 bila diisi (super_admin saja, role divalidasi).
4. **FK violation `swap_request`** tanpa assignment cocok (`''` vs NULL)
   → kini NULL; tambah 3 test swap + 1 test overlap (helper
   `find_shift_overlaps` / `apply_swap_decision` yang dipakai command).
5. Laporan: filter cabang/departemen/shift tak ada input (selalu '') →
   dropdown terisi dari DB; status → select enum Inggris + label Indonesia;
   tombol ekspor anti double-submit.
6. Devices: banner basi → status akurat + tabel DLL (`device_sdk_status`,
   command backend build 4 yang belum ada UI-nya) + kolom `config_json` +
   adapter select + perbaiki stale-state `fetchUsers`.
7. Attendance: nama karyawan (bukan ID), empty state, guard double-submit koreksi.
8. Schedule: `confirm()` native → `ConfirmDialog`, nama (bukan ID) di swap,
   empty states. Dashboard: error state + retry (sebelumnya loading selamanya).
   Leaves: lampiran readOnly (via unggah). Users: kolom password di-mask.
   Settings: Aktifkan/Matikan backup langsung tersimpan.
9. PDF ditulis ulang: multi-halaman A4, header berulang, nomor halaman,
   ringkasan, transliterasi ASCII (xref aman) — teruji + tervalidasi pypdf.
10. Engine +4 test: pulang-saja, scan ganda, co-sebelum-ci non-malam, libur ber-event.

### 16 → 27 test baru (build 3→5)

Build 3: 12 (COM/parse/record/PE/VARIANT/checkpoint/Hikvision/Solution/Timetech).
Build 4: +2 (`sdk_status_shape` zkcom/hikvision).
Build 5: +4 engine, +3 swap/overlap, +4 PDF/wrap (total lib 65; frontend 3).

## 0. Build 4 — SDK bisa diunduh & ditaruh (skrip + status)

| Perintah | Hasil |
|---|---|
| `vendor_sdk/download.ps1` (4 sumber, idempoten) | **LULUS** — semua dilewati (sudah ada) |
| `vendor_sdk/stage.ps1` (29 DLL + manifest, cek arsitektur PE) | **LULUS** — 29 file x64 ke `src-tauri/sdk/` + `target/debug|release/sdk` |
| `cargo fmt --all --check` | **LULUS** (0 diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **LULUS** (0 warning) |
| `cargo test` | **LULUS — 55/55**, 0 gagal, 4 `#[ignore]` (gerbang hardware) |

### Perubahan (build 4)

- Baru: `vendor_sdk/download.ps1`, `vendor_sdk/stage.ps1`,
  command `device_sdk_status` (terdaftar di `main.rs`), test
  `sdk_status_shape` ×2 (zkcom, hikvision).
- Fakta lisensi: paket SDK tanpa izin redistribusi eksplisit → installer
  TIDAK membundel DLL (opt-in terdokumentasi di `INSTALL_WINDOWS.md`).
- Frontend: tidak diubah.

## 0. Build 3 — SDK vendor diunduh + adapter nyata (backend saja)

| Perintah | Hasil |
|---|---|
| SDK ZKTeco 6.2.4.11 x86 (GitHub resmi) + PullSDK 6.3.1.37 x64 (NuGet) + Solution 6.3.1.54 x64+x86 (MediaFire support) + HCNetSDK 6.1.6.3 x64 + manual 267hlm + demo | **TERUNDUH** → `vendor_sdk/` (+`README.md`); arsitektur terverifikasi header PE; CLSID CZKEM + 339 method dibaca dari typelib DLL |
| `cargo fmt --all --check` | **LULUS** (0 diff) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **LULUS** (0 warning) |
| `cargo test` | **LULUS — 53/53**, 0 gagal, 4 `#[ignore]` (gerbang hardware) |
| `cargo check --all-targets` | LULUS |

### 16 test baru (build 3)

- `zkcom` (7): `clsid_czkem_matches_verified_typelib`, `parse_host_port_cases`,
  `parse_machine_and_port`, `inout_mode_mapping_matches_manual`,
  `record_id_and_event_time`, `pe_arch_label_parses_minimal_headers`,
  `variant_layout_is_16_bytes` — ok.
- `zkteco` (1): `checkpoint_parsing` — ok.
- `hikvision` (3): `conn_config_defaults`, `serial_parsing_trims_nul`,
  `unsupported_ops_are_honest` — ok.
- `solution` (1): `identity_is_solution` — ok.
- `timetech` (1): `timetech_refuses_honestly` — ok.
- Hardware (4, `#[ignore]`, butuh perangkat fisik + env — lihat
  `vendor_sdk/README.md`): `zkcom_hw_connect_and_info`, `zkteco_hw_fetch_smoke`,
  `solution_hw_connect_smoke`, `hik_hw_login_smoke` — dilewati di CI/lokal.

### Perubahan kode (build 3)

- Baru: `devices/zkcom.rs` (COM IDispatch hand-rolled, tanpa dep baru),
  `devices/zkteco.rs`, `devices/solution.rs`, `devices/hikvision.rs`
  (+`libloading 0.7`, sudah ada di lockfile sebagai transitif),
  `devices/timetech.rs`; `vendor_sdk/README.md`.
- Ubah: `devices/registry.rs` (status jujur per vendor),
  `sync/service.rs::build_adapter(nama, config_json)`, 5 call-site `lib.rs`
  (teruskan `config_json`; provision pakai host perangkat, bukan `"local"`),
  `Cargo.toml`, `.gitignore` (abaikan biner SDK), `SDK_INVENTORY.md`,
  `KNOWN_LIMITATIONS.md`.
- Frontend: tidak diubah (kontrak IPC sama) — tidak diuji ulang build 3.

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

- **Pengujian hardware nyata (ZKTeco, Solution, Hikvision): BLOCKED** —
  SDK sudah diunduh/diverifikasi/diimplementasikan (build 3), tetapi butuh
  perangkat fisik + registrasi DLL. Tidak ada klaim kompatibilitas.
  TIME TECH AC6 tetap tanpa SDK (Wiegand-only menurut vendor).
- **Installer diuji ada & terbentuk**; instalasi wizard end-to-end di mesin bersih
  belum dijalankan (disarankan sebelum distribusi).
- Screenshot UI di `docs/screenshots/` belum diambil dari sesi interaktif.
