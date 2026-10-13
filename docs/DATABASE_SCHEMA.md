# Skema Database

SQLite (`%APPDATA%\AttendX\attendx.db`, WAL, `foreign_keys=ON`).
Migrasi versioned di `src-tauri/src/db/migrations/00N_*.sql`, tercatat di
`schema_migrations(version, name, applied_at)`. Versi saat ini: **7**.

## 001_init — inti kepegawaian

- `branches(id, code UNIQUE, name, address, created_at)`
- `departments(id, code UNIQUE, name, branch_id→branches, created_at)`
- `positions(id, code UNIQUE, name, created_at)`
- `employees(id, nik UNIQUE, name, email, phone, address, hire_date,
  department_id, position_id, branch_id, manager_id→employees,
  shift_pattern, status, photo_path, deleted_at, created_at, updated_at)`
- `employee_history(id, employee_id, changed_by, field_name, old_value,
  new_value, reason, changed_at)`
- `employee_device_map(id, employee_id, device_id, device_user_id,
  created_at, UNIQUE(device_id, device_user_id))`

## 002_shifts — shift & kalender

- `shifts(id, code UNIQUE, name, start_time, end_time, crosses_midnight,
  break_minutes, late_tolerance_minutes, clockin_early_limit_minutes,
  clockin_late_limit_minutes, clockout_limit_minutes, overtime_before_minutes,
  overtime_after_minutes, applicable_days, is_active, version,
  created_at, updated_at)`
- `shift_assignments(id, employee_id, shift_id, effective_date, end_date,
  created_by, created_at)` + index `(employee_id, effective_date)`
- `shift_history(id, shift_id, version, payload_json, changed_by, changed_at)`
- `work_calendar(id, work_date UNIQUE, kind, description, created_at)`
- `holidays(id, holiday_date UNIQUE, name, kind, created_at)`
- `attendance_policy(id, rounding_minutes, max_early_minutes,
  require_overtime_approval, allow_double_shift, updated_at)`

## 003_devices — perangkat & sync

- `devices(id, name, vendor, model, serial_number, host, branch_id, location,
  connection_type, adapter_name, sdk_version, status, last_sync_at,
  ok_count, fail_count, is_active, config_json, created_at, updated_at)`
- `device_sync_state(device_id PK, checkpoint, last_success_at,
  consecutive_failures, updated_at)`
- `device_connection_log(id, device_id, event, detail, created_at)`

## 004_attendance — event & hasil

- `raw_events(id, device_id, vendor_record_id, device_user_id, event_time,
  direction, payload_json, status, error_text, ingested_at,
  UNIQUE(device_id, vendor_record_id, event_time, device_user_id))`
  + index `(event_time)`, `(status)`
- `attendance_results(id, employee_id, work_date, shift_id, shift_version,
  clock_in, clock_out, work_minutes, break_minutes, late_minutes,
  early_leave_minutes, overtime_minutes, overtime_approved, status, anomaly,
  source, computed_at, UNIQUE(employee_id, work_date))` + index `(work_date)`
- `corrections_audit(id, attendance_id, changed_by, field_name, old_value,
  new_value, reason, created_at)`

## 005_hr — cuti, lembur, pengguna

- `leave_types(id, code UNIQUE, name, quota_days, created_at)`
- `leave_requests(id, employee_id, leave_type_id, start_date, end_date, reason,
  attachment_path, status, approved_by, decided_at, created_at)`
- `overtime_requests(id, employee_id, work_date, minutes_requested, reason,
  status, approved_by, decided_at, created_at)`
- `users(id, username UNIQUE, display_name, role, password_hash, is_active,
  last_login_at, created_at)`
- `audit_log(id, actor, action, entity, entity_id, detail, created_at)`
  + index `(actor)`

## 006_seed — data awal

Cabang HQ, departemen Umum, posisi Staff, 3 shift preset (Pagi/Siang/Malam),
`attendance_policy` default (pembulatan 1 mnt, lembur wajib approval),
4 jenis cuti. **Tanpa user default** (admin dibuat via setup sekali pakai).

## 007_settings_swaps — pengaturan & tukar shift

- `app_settings(key PK, value, updated_at)` + seed:
  `sync_interval_secs=300`, `backup_keep_files=10`,
  `session_timeout_secs=28800`, `scheduler_enabled=0`.
- `shift_swaps(id, employee_id, from_assignment_id, from_shift_id, to_shift_id,
  swap_date, reason, status, decided_by, decided_at, created_at)`

## Konvensi

- ID: UUID v4 string. Waktu: TEXT WIB `YYYY-MM-DD HH:MM:SS`.
- Tulis penting memakai transaksi; semua nilai via parameter binding.
- `raw_events` dan `corrections_audit` bersifat append-only dari UI.
