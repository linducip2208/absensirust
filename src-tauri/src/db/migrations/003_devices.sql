CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    vendor TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT '',
    serial_number TEXT NOT NULL DEFAULT '',
    host TEXT NOT NULL DEFAULT '',
    branch_id TEXT REFERENCES branches(id),
    location TEXT DEFAULT '',
    connection_type TEXT NOT NULL DEFAULT 'sdk',
    adapter_name TEXT NOT NULL,
    sdk_version TEXT NOT NULL DEFAULT 'unknown',
    status TEXT NOT NULL DEFAULT 'offline',
    last_sync_at TEXT,
    ok_count INTEGER NOT NULL DEFAULT 0,
    fail_count INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS device_sync_state (
    device_id TEXT PRIMARY KEY REFERENCES devices(id),
    checkpoint TEXT NOT NULL DEFAULT '',
    last_success_at TEXT,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS device_connection_log (
    id TEXT PRIMARY KEY,
    device_id TEXT NOT NULL REFERENCES devices(id),
    event TEXT NOT NULL,
    detail TEXT DEFAULT '',
    created_at TEXT NOT NULL
);
