CREATE TABLE IF NOT EXISTS branches (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    address TEXT DEFAULT '',
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS departments (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    branch_id TEXT REFERENCES branches(id),
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS positions (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS employees (
    id TEXT PRIMARY KEY,
    nik TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    email TEXT DEFAULT '',
    phone TEXT DEFAULT '',
    address TEXT DEFAULT '',
    hire_date TEXT NOT NULL,
    department_id TEXT REFERENCES departments(id),
    position_id TEXT REFERENCES positions(id),
    branch_id TEXT REFERENCES branches(id),
    manager_id TEXT REFERENCES employees(id),
    shift_pattern TEXT DEFAULT 'PAGI',
    status TEXT NOT NULL DEFAULT 'aktif',
    photo_path TEXT DEFAULT '',
    deleted_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS employee_history (
    id TEXT PRIMARY KEY,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    changed_by TEXT NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT DEFAULT '',
    new_value TEXT DEFAULT '',
    reason TEXT DEFAULT '',
    changed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS employee_device_map (
    id TEXT PRIMARY KEY,
    employee_id TEXT NOT NULL REFERENCES employees(id),
    device_id TEXT NOT NULL,
    device_user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(device_id, device_user_id)
);
