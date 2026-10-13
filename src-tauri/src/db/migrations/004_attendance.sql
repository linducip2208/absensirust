CREATE TABLE IF NOT EXISTS raw_events (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(id),
    vendor_record_id TEXT NOT NULL DEFAULT '',
    device_user_id TEXT NOT NULL,
    event_time TEXT NOT NULL,
    direction TEXT NOT NULL DEFAULT 'in',
    payload_json TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'pending',
    error_text TEXT DEFAULT '',
    ingested_at TEXT NOT NULL,
    UNIQUE(device_id, vendor_record_id, event_time, device_user_id)
);
CREATE INDEX IF NOT EXISTS idx_raw_events_time ON raw_events(event_time);
CREATE INDEX IF NOT EXISTS idx_raw_events_status ON raw_events(status);
CREATE TABLE IF NOT EXISTS attendance_results (
    id TEXT PRIMARY KEY,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    work_date TEXT NOT NULL,
    shift_id TEXT REFERENCES shifts(id),
    shift_version INTEGER NOT NULL DEFAULT 1,
    clock_in TEXT,
    clock_out TEXT,
    work_minutes INTEGER NOT NULL DEFAULT 0,
    break_minutes INTEGER NOT NULL DEFAULT 0,
    late_minutes INTEGER NOT NULL DEFAULT 0,
    early_leave_minutes INTEGER NOT NULL DEFAULT 0,
    overtime_minutes INTEGER NOT NULL DEFAULT 0,
    overtime_approved INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'NeedsReview',
    anomaly TEXT NOT NULL DEFAULT '',
    source TEXT NOT NULL DEFAULT 'engine v1',
    computed_at TEXT NOT NULL,
    UNIQUE(employee_id, work_date)
);
CREATE INDEX IF NOT EXISTS idx_att_results_date ON attendance_results(work_date);
CREATE TABLE IF NOT EXISTS corrections_audit (
    id TEXT PRIMARY KEY,
    attendance_id TEXT NOT NULL REFERENCES attendance_results(id),
    changed_by TEXT NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT DEFAULT '',
    new_value TEXT DEFAULT '',
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL
);
