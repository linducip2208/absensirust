# Laporan Pengujian (hasil aktual — build 10: 2026-10-17, Windows 11 x64)

## 0. Build 10 — audit menyeluruh kedua (arah punch, pelindung data, perbaikan laporan & UI)

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` / eslint / `vitest run` | **LULUS** — 12/12 |
| `cargo fmt` / `clippy -D warnings` | **LULUS** (0 error, 0 warning) |
| `cargo test` | **LULUS — 88/88 lib**, 4 `#[ignore]` hardware |
| `npm run build` + `npx tauri build` (NSIS) | **LULUS** — exe 14,49 MB + setup 3,51 MB |
| Smoke test DB | **LULUS** — **migrasi v10 teraplikasi**, integrity ok |

### Bug nyata diperbaiki (build 10) — hasil audit 3 utas + verifikasi manual

**Backend:**

1. **Izin baca tetap ditutup untuk halaman baca** (regresi build 9): Users
   Audit & halaman Absensi tampak "kosong" memaksa error karena
   `check_table_perm` menutup READ semua tabel terlarang. Kini WRITE tabel
   sistem (`attendance_results`/`raw_events`/`audit_log`/`app_settings`)
   tetap terlarang, tapi **read dikembalikan**: `attendance_results`
   (`attendance.view`), `audit_log` (`user.view`), `raw_events` tetap
   tertutup, `app_settings` → pesan Validation "via menu Pengaturan".
2. **Pengelompokan event mengabaikan `raw_events.direction`**: punch `out`
   shift malam (06:58) bisa menjadi clock-in palsu shift pagi berikutnya.
   `group_events_for_shift` kini bawa arah (`in`/`out`/''=in): clock-in
   hanya `'in'`, clock-out hanya `'out'` terakhir ≥5 menit; jendela tanpa
   punch `'in'` sama sekali → kosong (tidak direkayasa). Perangkat
   modeless (semua `'in'`) memakai heuristik lama. (+4 test: modeless,
   lintas-shift pagi tak palsu, hanya-`out`, klaster duplikat.)
3. **Pemetaan karyawan↔mesin per `device_user_id` saja** padahal UNIQUE
   `(device_id, device_user_id)` — dua perangkat dengan user-id sama
   tercampur. `attendance_compute` kini memetakan per pasangan
   `(device_id, device_user_id)` (query per perangkat).
4. **Koreksi manual lenyap saat hitung ulang** → kolom `manually_corrected`
   (migrasi **v10** `010_attendance_guard.sql`); `attendance_correct`
   menandai `manually_corrected=1` + trim nilai; `attendance_compute`
   melewatinya (`skipped_manual` di JSON hasil).
5. **Backup otomatis scheduler tanpa `checkpoint_wal()` + tanpa validasi**
   → WAL tidak ikut tersalin. Kini checkpoint dulu, salin, **validasi
   integrity** (`backup_db` memakai `validate_backup`), gagal = file
   dibuang (tidak menunggu rotasi membuang backup lama).
6. **Nama backup detik-an bisa saling menimpa** → suffix `-1`, `-2`, …;
   **rotasi menyapu salinan pre-restore** → kini hanya
   `attendx-backup-*.db` yang dirotasi. (+2 test: nama unik 1 detik,
   rotasi selamat menyisakan pre-restore & file asing.)
7. **`build_adapter` diam-diam mensimulasikan nama tak dikenal** (mis.
   salah kapital `ZkTeco`) → kini "`Validation: adapter tidak dikenal`";
   SIMULATED hanya untuk `mock*`/kosong yang disengaja. 5 pemanggil
   disesuaikan ke `AppResult<Box<dyn DeviceAdapter>>`.
8. **`SyncStats.attempts` selalu 1** meski loop retry menghitung `attempts`
   → kini benar (`SyncStats { attempts, ..s }`).
9. **`applog` file harian tak berbatas** (loop error = banjir) → batas
   10 MB/file/hari, penanda sekali sehari saat berhenti menulis.
10. **`to_csv` tanpa pelindung formula** (`=CMD` dieksekusi Excel saat
    laporan berisi teks berbahaya) → sel diawali `'` bila diawali
    `=`/`+`/`@` (dan `-` non-angka); angka negatif sah tidak tersentuh
    (+test dilindungi, angka utuh).
11. **Form pengaturan tanpa validasi angka**: `rounding_minutes` berisi
    "lima"/"" bisa membuat komputasi baca `i64` gagal → validasi backend
    `validate_value_rules` (angka ≥0, rounding ≥1, flag 0/1; kuota izin
    ≥0) dipanggil di create+update, ditiru di klien Settings (revert
    nilai gagal + tombol per-kolom "Menyimpan…").
12. **`shifts.applicable_days` bisa kosong/tak valid** → dinormalisasi
    saat create (kosong → `0,1,2,3,4`), ketat 0..=6 unik saat create+
    update (+test `normalize_applicable_days_aturan_konvensi`); data
    lama `'1,2,3,4,5'` ikut dibereskan migrasi v10.
13. **Pengajuan izin/lembur bisa diubah statusnya lewat edit generik**
    (bypass keputusan) → kolom `status`/`approved_by`/`decided_at`
    dikunci: hanya lewat `leave_decide`/`overtime_decide` (perintah baru —
    dulu lembur tidak punya jalur keputusan apa pun):
    +test `overtime_decide_pending_sekali_saja`, guard di create/update,
    frontend registry + kartu Keputusan lembur.
14. **Kalender kerja tak bisa membatalkan libur mingguan** → helper
    `resolve_day_off`: `kind='kerja'` = hari kerja khusus (menang atas
    pola mingguan), `kind='libur'` = libur walau shift berlaku
    (+test `kalender_khusus_menang_diatas_pola_mingguan`).
15. **Dashboard `tidak_hadir` menghitung belum dihitung** → kartu baru
    `belum_dihitung = aktif − baris_hasil` (jujur saat mesin belum
    jalan); `tidak_hadir` = baris_hasil − hadir − cuti − libur −
    belum_lengkap (tak pernah negatif).
16. **Rate-limit >10rb reset menyapu lockout aktif** → berjenjang:
    buang kedaluwarsa → sementara simpan yang ≥3 gagalan → terakhir
    baru `clear()`.
17. **`zkcom::build_event_time` menerima 31 Februari** (jam rusak
    dikirim berulang, checkpoint tersangkut selamanya) → tanggal
    divalidasi kalender (`NaiveDate::from_ymd_opt`).
18. **`zkteco::checkpoint_time` bisa panic** saat memotong checkpoint
    non-ASCII (DB rusak) → `is_ascii()` dijaga dulu (None = dari awal).
19. **`table_list` pencarian salah kolom & wildcard liar**: kini kolom
    teks utama per tabel (leave/overtime→`reason`, holiday→`name`,
    mapping→`device_user_id`, dsb.) + wildcard `% _ \` di-escape
    (`LIKE ?1 ESCAPE '\`) di create pencarian.
20. **`search_global` tanpa batas panjang** → maks 100 karakter.

**Frontend:**

21. **Modal koreksi absensi memakai `busy`**: spinner tidak muncul saat
    koreksi berjalan (sudah این pegangan `saving` baru) — kini benar dan
    tombol per-peran: Hitung (hr_admin/super_admin/operator_device),
    Antrean ulang (operator_device/super_admin), Koreksi (hr/super).
22. **Crud `readOnly` hanya dinonaktifkan saat edit** — status pengajuan
    masih terkirim "approved" saat tambah → kini readOnly selalu
    dinonaktifkan & tidak dikirim pada payload (status pengajuan tidak
    bisa dipalsukan lewat form).
23. Leaves: `status` kini readOnly (badge, form terkunci), keputusan
    hanya via kartu; kartu Keputusan digeneralisasi + **kartu Keputusan
    lembur** (dulu tidak ada jalur); saldo cuti diberi label jujur
    "sepanjang sejarah" (tanpa aturan periode — lihat KNOWN_LIMITATIONS).
24. Settings: validasi angka klien + pesan batal tersimpan (revert) +
    tombol per-field "Menyimpan…"; kebijakan divalidasi sebelum kirim.
25. Jadwal: kartu bentrok punya **status gagal + coba lagi** (dulu
    "Memuat" abadi bila scheduleConflicts gagal); toast jujur untuk
    pratinjau vs penulisan (`[Pratinjau]` vs sukses).
26. Pencarian global `state.q` kini ikut saat navigasi ke rute yang sama
    (dulu `initialSearch` hanya terbaca sekali; kini `key` Crud diperbarui
    + kembali ke tab Data); login kembali ke `loc.state.from` (dulu selalu
    `/`).
27. Dasbor: kartu "Belum dihitung" + kunci baris komposit (tanggal-nama)
    menghindari duplikat-kunci React.
28. Ekspor karyawan CSV kini diberi **BOM UTF-8** (Excel tak salah
    encoding nama non-ASCII).
29. Kontrak IPC diperluas: `overtime_decide`, `has_admin`, `login`,
    `logout`, `settings_get/set`, `scheduler_get/set`, `sync_run_all`,
    `backup_create/restore`, `health_check`, `search_global`,
    `diagnostics_bundle`, `attendance_review`, impor jadwal+karyawan
    (test frontend 10→12).

### Yang disengaja tidak diubah (build 10)

- `setup_admin` TOCTOU, bocor BSTR zkcom jalur error, pemetaan pesan
  UNIQUE per-kolom, matriks peran→tabel tulis penuh (lihat build 9).
- Zkteco: mundur jam perangkat tak bisa dibedakan dari baris lama oleh
  checkpoint (lihat KNOWN_LIMITATIONS no. 16) — baris lama aman, hanya
  deteksi mundurnya yang terbatas.
- Dedup ganda-punch < 1 detik pada pasangan sama (perangkat kirim dua
  event milidetik yang sama) — masih masuk raw_events dan dihitung
  duplikat oleh heuristik klaster (lihat KNOWN_LIMITATIONS no. 17).

## 0. Build 9 — audit menyeluruh (konvensi hari, laporan log diperbaiki, guard tulis tabel)

| Perintah | Hasil |
|---|---|
| `npx tsc --noEmit` / `eslint` / `vitest run` | **LULUS** — 10/10 |
| `npm run build` + `npx tauri build` (NSIS) | **LULUS** — exe 14,35 MB + setup 3,49 MB |
| `cargo fmt` / `clippy -D warnings` / `cargo test` | **LULUS — 77/77 lib**, 4 `#[ignore]` hardware |
| Smoke test 15 dtk + DB | **LULUS** — integrity ok, 26 tabel, **migrasi v9 teraplikasi**, seed shift kini `0,1,2,3,4` |

### Bug nyata diperbaiki (build 9) — hasil audit 3 utas + verifikasi manual

**Backend:**

1. **Konvensi `applicable_days` salah untuk seed** (braket: kode 0=Sen..6=Min,
   seed pakai `'1,2,3,4,5'` → Senin dianggap libur untuk semua shift seed).
   Konvensi tetap 0-based; data dimigrasi (migrasi **v9** `009_fix_applicable_days.sql`
   `1,2,3,4,5`→`0,1,2,3,4` untuk PAGI/SIANG/MALAM), seed & fallback
   `shift_from_value` ikut diperbaiki (+test `applicable_days_konvensi_senin_nol`).
2. **Laporan `error_sync` & `riwayat_koreksi` 100% rusak** (rusqlite
   `InvalidParameterCount`: placeholder ≠ jumlah argumen; `log_perangkat`
   rusak bila filter opsional diisi). `report_data` ditulis ulang: pasangan
   kondisi+placeholder `?N` selalu sejumlah argumen, validasi tanggal pad
   + penolakan rentang terbalik, filter per-jenis yang jujur (`terlambat`
   → `late_minutes>0`, dst), 3 jenis log memakai rentang penuh hari + tepat 2 argumen.
3. **Kebocoran otorisasi via tabel API generik**: viewer/supervisor/operator
   bisa MENGUBAH `raw_events`/`attendance_results`; `audit_log` bisa diedit;
   `app_settings` rusak via create (injeksi kolom `id`). Kini semua ditolak
   eksplisit di create/update/delete dengan pesan jelas.
4. **Poison-pill ingest**: satu `event_time` korup menggulung balik seluruh
   batch selamanya → kini baris korup dilewati per-baris (lapor `invalid`),
   batch tetap masuk; UI sinkronisasi menampilkan jumlah dilewati (+test
   `ingest_skips_bad_time_not_poison_batch`).
5. **`device_sync` manual melewati `sync_lock()`** (bisa tabrakan dengan
   scheduler) → kini mengambil lock dengan fail-fast `Conflict`.
6. **Data race `LAST_HOST` global static** di zkcom → `thread_local`.
7. **zkteco**: pasangan `EnableDevice(false)`/re-enable dihapus (baca log
   tidak mengunci perangkat; jika re-enable gagal terminal nyaris mati);
   `enable_device` tak terpakai dihapus (dead code clippy `-D warnings`).
8. **Approve tukar jadwal tanpa cek bentrok** → kini cek clash (contoh
   assignment asal dikecualikan) + eksistensi shift tujuan & karyawan pada
   `swap_request`/approve; tanggal tanpa nol-pad ditolak (`parse_date` ketat).
9. **Sesi target dibatalkan saat users diubah/hapus** (role/password/
   is_active baru berlaku segera, bukan saat sesi lama kedaluwarsa).
10. **`guard_user_update`**: larang ubah role/nonaktifkan diri sendiri,
    larang menurunkan/nonaktifkan super_admin terakhir, role divalidasi.
11. **JSON bool → "true"/"false" di kolom INTEGER** (merusak filter
    `is_active`) → ditulis `"1"/"0"`.
12. **Race enable/disable scheduler** (`sched_enabled` vs tugas lama masih
    hidup) → `AppState.sched_task: Mutex<Option<JoinHandle>>`; enable =
    hentikan lama + tarik tugas baru, disable = hentikan.
13. **Perintah `leave_decide` baru** (butuh `leave.approve`) — supervisor
    tadinya dead-end (punya `leave.approve` tapi update generik butuh
    `leave.manage`); inti logika diekstrak +1 test
    (`leave_decide_pending_sekali_saja`).
14. **Validasi kolom tanggal** (`leave_requests` start≤end via `parse_date`,
    `holidays`/`work_calendar`/`overtime_requests`/`employees` wajib pad),
    whitelist `work_calendar.kind` = `kerja|libur`, lembur `minutes>=0`.
15. **Rate-limit**: entri lama dipurge massal + map dibersihkan saat >10rb;
    pesan login seragam (anti-enumerasi).
16. **Sinkronisasi**: batal dicek pertama (tanpa catatan gagal perangkat);
    error non-retryable langsung berhenti; `SyncStats.failed` jujur =
    jumlah baris korup yang dilewati.
17. `shift_history` payload disusun via serde (SQL `printf` dihapus);
    `device_registry` kini wajib token + `device.view`; dashboard
    `tidak_hadir` tak lagi menghitung belum-lengkap (kolom baru).
18. **Mesin absensi**: cluster scan ganda di akhir masuk tak lagi menjadi
    clock-out (0 menit "Present") — clock-out = event ≥5 menit setelah
    clock-in (+test); lembur sebelum shift dead-code dihapus (kebijakan
    `overtime_before_minutes` dikikis —lihat KNOWN_LIMITATIONS); "Absent"
    tak ditulis bila jendela shift belum berakhir (skip +
    `skipped_window_not_ended` di JSON hitung).

**Frontend:**

19. **Sesi kedaluwarsa dead-end** → wrapper `invoke` mendeteksi error auth,
    membersihkan sesi + mengarahkan ke `#/login` otomatis.
20. `ToastProvider` stabil (refuse re-render berantai refetch); Crud
    anti dobel-submit (`saving`/`deleteBusy`) + tarik-ulang halaman luar
    rentang; GlobalSearch debtor 300 ms + gunakan-gilira (race safe).
21. Karyawan: **ekspor CSV halaman-loop** (dulu terpotong diam-diam di
    200 baris), foto 500→200, nama pakai helper `fetchEmployeeNameMap`
    (lebih tahan bila role tanpa `employee.view`).
22. Devices: status error registry/SDK dinyatakan jelas (bukan "Memuat…"
    abadi), reload setelah tes/sinkron, sinkron menampilkan jumlah dilewati.
23. Jadwal/Presensi/Dasbor: kirim-permintaan berurutan (kalahkan race
    `myReq`), konflik/tukar reset sebelum muat, footer `-` untuk kosong.
24. Leaves: **kartu Keputusan** (pilih pengajuan pending + Setujui/Tolak
    via `leave_decide` + ConfirmDialog) + lampiran menampilkan nama
    karyawan (bukan UUID).
25. Users: **password wajib saat tambah** (`requiredOnCreate`),
    `is_active` hanya tersedia saat ubah (default ada = aktif).
26. Kalender: opsi "setengah" dihapus (backend kini menolak).
27. Laporan: baris filter cabang/departemen/shift/status/karyawan
    disembunyikan untuk 3 jenis log + petunjuk (dulu tampil tapi diabaikan).
28. Kontrak IPC diperluas: `device_registry(token)`, `leave_decide`
    (test frontend 9→10).

### Yang disengaja tidak diubah (build 9)

- `setup_admin` TOCTOU jendela kecil (kredensial default memang tidak ada;
  tabel tetap kosong hingga admin dibuat — risiko rendah, terdokumentasi).
- Bocor BSTR zkcom jalur error (lihat KNOWN_LIMITATIONS no. 12).
- Pemetaan pesan `UNIQUE` per-kolom (masih pesan generik yang jelas).
- Matriks peran→tabel tulis penuh (permisive column guard sudah diberkas).

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
