CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL
);
INSERT OR IGNORE INTO app_settings (key, value, updated_at)
VALUES
('sync_interval_secs', '300', '2026-01-01 00:00:00'),
('backup_keep_files', '10', '2026-01-01 00:00:00'),
('session_timeout_secs', '28800', '2026-01-01 00:00:00'),
('scheduler_enabled', '0', '2026-01-01 00:00:00');
CREATE TABLE IF NOT EXISTS shift_swaps (
    id TEXT PRIMARY KEY,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    from_assignment_id TEXT REFERENCES shift_assignments(id),
    from_shift_id TEXT NOT NULL REFERENCES shifts(id),
    to_shift_id TEXT NOT NULL REFERENCES shifts(id),
    swap_date TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    decided_by TEXT DEFAULT '',
    decided_at TEXT,
    created_at TEXT NOT NULL
);
