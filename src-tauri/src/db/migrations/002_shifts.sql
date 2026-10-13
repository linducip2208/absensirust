CREATE TABLE IF NOT EXISTS shifts (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    crosses_midnight INTEGER NOT NULL DEFAULT 0,
    break_minutes INTEGER NOT NULL DEFAULT 60,
    late_tolerance_minutes INTEGER NOT NULL DEFAULT 10,
    clockin_early_limit_minutes INTEGER NOT NULL DEFAULT 120,
    clockin_late_limit_minutes INTEGER NOT NULL DEFAULT 180,
    clockout_limit_minutes INTEGER NOT NULL DEFAULT 240,
    overtime_before_minutes INTEGER NOT NULL DEFAULT 0,
    overtime_after_minutes INTEGER NOT NULL DEFAULT 60,
    applicable_days TEXT NOT NULL DEFAULT '1,2,3,4,5',
    is_active INTEGER NOT NULL DEFAULT 1,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS shift_assignments (
    id TEXT PRIMARY KEY,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    shift_id TEXT NOT NULL REFERENCES shifts(id),
    effective_date TEXT NOT NULL,
    end_date TEXT,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_shift_assign_emp ON shift_assignments(employee_id, effective_date);
CREATE TABLE IF NOT EXISTS shift_history (
    id TEXT PRIMARY KEY,
    shift_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    changed_by TEXT NOT NULL,
    changed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS work_calendar (
    id TEXT PRIMARY KEY,
    work_date TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL DEFAULT 'kerja',
    description TEXT DEFAULT '',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS holidays (
    id TEXT PRIMARY KEY,
    holiday_date TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'nasional',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS attendance_policy (
    id TEXT PRIMARY KEY,
    rounding_minutes INTEGER NOT NULL DEFAULT 1,
    max_early_minutes INTEGER NOT NULL DEFAULT 120,
    require_overtime_approval INTEGER NOT NULL DEFAULT 1,
    allow_double_shift INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
