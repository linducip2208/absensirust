# Arsitektur AttendX Rust

```
┌──────────────────────────────────────────────────────────┐
│ UI (React+TS, Tabler lokal)  src/                        │
│  TIDAK menyimpan kebenaran data; semua via IPC           │
└────────────────────── IPC (Tauri invoke) ────────────────┘
┌──────────────────────────────────────────────────────────┐
│ Command handler  src-tauri/src/lib.rs (mod commands)     │
│  auth guard per-perintah (permission, bukan sekadar UI)  │
├──────────────┬──────────────┬──────────────┬─────────────┤
│ attendance/  │ devices/     │ sync/        │ reports/    │
│ shift window │ trait +      │ checkpoint,  │ CSV/XLSX/   │
│ + engine     │ ZK/Hikvision │ retry,       │ PDF multi-  │
│ (murni)      │ +mock+reg.   │ idempotent   │ halaman     │
├──────────────┴──────────────┴──────────────┴─────────────┤
│ db/repo (rusqlite, parameter binding, transaksi)         │
│ SQLite %APPDATA%\AttendX\attendx.db + migrasi versioned  │
├──────────────────────────────────────────────────────────┤
│ auth (Argon2, session, rate-limit) · backup · scheduler  │
│ timex (Asia/Jakarta) · errors                            │
└──────────────────────────────────────────────────────────┘
│ SDK vendor: ZKEM via COM CZKEM x64, HCNetSDK via FFI     │
│ (DLL + registrasi manual; tanpa hardware = error jelas) │
└──────────────────────────────────────────────────────────┘
```

## Batas yang dijaga

- **UI ↔ domain**: UI tidak menghitung absensi; satu-satunya penulis hasil adalah
  `attendance_compute` (engine) dan `attendance_correct` (audit wajib).
- **Hasil ↔ event mentah**: `raw_events` tidak pernah diubah/dihapus via UI koreksi.
  Koreksi menulis ke `attendance_results` + baris `corrections_audit`.
- **Domain ↔ hardware**: hanya lewat trait `DeviceAdapter` (`devices/adapter.rs`).
  Operasi yang tidak didukung = `Capability::Unsupported`, bukan tebakan protokol.
- **Waktu**: `timex.rs` satu-satunya sumber "sekarang (WIB)"; engine murni tanpa jam sistem
  (input tanggal eksplisit) sehingga dapat diuji deterministik.
- **DB**: semua query tulis penting dalam transaksi (`transaction_ingest_events`,
  migrasi per versi, upsert hasil + audit koreksi).

## Alur sinkronisasi

```
adapter.fetch_attendance_logs(checkpoint, limit, cancel)
  -> Repo::transaction_ingest_events (INSERT OR IGNORE = idempotent)
  -> update checkpoint + last_success + connection_log   (sukses)
  -> backoff retry N kali, lalu status=error + log gagal (gagal)
```

## Alur hitung absensi

```
raw_events (+ employee_device_map) + shift_assignments (efektif)
  + holidays/work_calendar + leave/overtime approved + policy
  -> group_events_for_shift (malam: tarik event hingga end+limit ke work_date asal)
  -> compute_day -> attendance_results (upsert per employee+work_date)
```

## Keputusan penting

- Timestamp operasional = TEXT WIB `YYYY-MM-DD HH:MM:SS` (bukan UTC, bukan epoch),
  agar terbaca manusia dan konsisten lintas mesin. Detail: `docs/SHIFT_ENGINE.md`.
- Tanpa server jaringan: tidak ada port yang dibuka; IPC Tauri lokal saja.
- Mock adapter dikompilasi bersama biner tetapi hanya dipakai bila `adapter_name`
  diawali `mock` — dan selalu dilaporkan sebagai SIMULATED.
