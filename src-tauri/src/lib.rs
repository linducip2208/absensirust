pub mod applog;
pub mod attendance;
pub mod auth;
pub mod backup;
pub mod db;
pub mod devices;
pub mod errors;
pub mod reports;
pub mod scheduler;
pub mod sync;
pub mod timex;

use auth::SessionStore;
use db::Repo;
use errors::{AppError, AppResult};
use std::path::PathBuf;

pub struct AppState {
    pub repo: Repo,
    pub sessions: SessionStore,
    pub db_path: PathBuf,
    pub sched_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub sched_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

// Semua Tauri command ada di modul ini (bukan crate root) agar tidak bentrok
// dengan #[macro_export] yang digenerate makro #[tauri::command].
pub mod commands {
    use super::attendance;
    use super::auth::{self, Role};
    use super::backup;
    use super::db::Repo;
    use super::devices;
    use super::errors::{AppError, AppResult};
    use super::reports;
    use super::sync;
    use super::timex;
    use super::AppState;
    use std::sync::atomic::AtomicBool;
    use tauri::Manager;

    // ---------- helpers ----------
    fn require(state: &AppState, token: &str, perm: &str) -> AppResult<auth::Session> {
        state.sessions.check_permission(token, perm)
    }

    fn now() -> String {
        timex::now_wib_string()
    }
    fn uid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn shift_from_value(v: &serde_json::Value) -> attendance::ShiftRule {
        attendance::ShiftRule::from_row(
            v.get("code").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("name").and_then(|x| x.as_str()).unwrap_or(""),
            v.get("start_time")
                .and_then(|x| x.as_str())
                .unwrap_or("07:00"),
            v.get("end_time")
                .and_then(|x| x.as_str())
                .unwrap_or("15:00"),
            v.get("crosses_midnight")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            v.get("break_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(60),
            v.get("late_tolerance_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(10),
            v.get("clockin_early_limit_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(120),
            v.get("clockin_late_limit_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(180),
            v.get("clockout_limit_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(240),
            v.get("overtime_before_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(0),
            v.get("overtime_after_minutes")
                .and_then(|x| x.as_i64())
                .unwrap_or(60),
            v.get("applicable_days")
                .and_then(|x| x.as_str())
                .unwrap_or("1,2,3,4,5"),
            v.get("version").and_then(|x| x.as_i64()).unwrap_or(1),
        )
    }

    // ---------- auth ----------
    #[tauri::command]
    pub fn has_admin(state: tauri::State<AppState>) -> Result<bool, AppError> {
        let n: i64 = state
            .repo
            .query_one("SELECT COUNT(*) FROM users", &[], |r| r.get(0))?
            .unwrap_or(0);
        Ok(n > 0)
    }

    #[tauri::command]
    pub fn setup_admin(
        state: tauri::State<AppState>,
        username: String,
        display_name: String,
        password: String,
    ) -> Result<String, AppError> {
        let n: i64 = state
            .repo
            .query_one("SELECT COUNT(*) FROM users", &[], |r| r.get(0))?
            .unwrap_or(0);
        if n > 0 {
            return Err(AppError::Forbidden("admin sudah ada, gunakan login".into()));
        }
        if username.trim().len() < 3 {
            return Err(AppError::Validation("username minimal 3 karakter".into()));
        }
        let hash = auth::hash_password(&password)?;
        let id = uid();
        state.repo.exec(
        "INSERT INTO users (id, username, display_name, role, password_hash, created_at) VALUES (?1,?2,?3,'super_admin',?4,?5)",
        &[&id, &username, &display_name, &hash, &now()],
    )?;
        state
            .repo
            .insert_audit(&username, "setup_admin", "users", &id, "admin pertama")?;
        Ok("ok".into())
    }

    #[tauri::command]
    pub fn login(
        state: tauri::State<AppState>,
        username: String,
        password: String,
    ) -> Result<serde_json::Value, AppError> {
        let (token, uname, role) = auth::login(&state.repo, &state.sessions, &username, &password)?;
        Ok(serde_json::json!({"token": token, "username": uname, "role": role}))
    }

    #[tauri::command]
    pub fn logout(state: tauri::State<AppState>, token: String) -> Result<String, AppError> {
        state.sessions.invalidate(&token);
        Ok("ok".into())
    }

    // ---------- dashboard ----------
    #[tauri::command]
    pub fn dashboard(
        state: tauri::State<AppState>,
        token: String,
        date: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let r = &state.repo;
        let d = if date.is_empty() {
            timex::today_wib().format("%Y-%m-%d").to_string()
        } else {
            date
        };
        let q = |sql: &str| -> i64 {
            r.query_one(sql, &[], |x| x.get(0))
                .unwrap_or(None)
                .unwrap_or(0)
        };
        let active_emp: i64 = r
            .query_one(
                "SELECT COUNT(*) FROM employees WHERE status='aktif' AND deleted_at IS NULL",
                &[],
                |x| x.get(0),
            )?
            .unwrap_or(0);
        let hadir: i64 = r.query_one(
        "SELECT COUNT(*) FROM attendance_results WHERE work_date=?1 AND status IN ('Present','Late')", &[&d], |x| x.get(0))?.unwrap_or(0);
        let terlambat: i64 = r
            .query_one(
                "SELECT COUNT(*) FROM attendance_results WHERE work_date=?1 AND status='Late'",
                &[&d],
                |x| x.get(0),
            )?
            .unwrap_or(0);
        let cuti: i64 = r.query_one(
        "SELECT COUNT(*) FROM attendance_results WHERE work_date=?1 AND status IN ('Leave','Permission')", &[&d], |x| x.get(0))?.unwrap_or(0);
        let tidak_hadir = (active_emp - hadir - cuti).max(0);
        let dev_total = q("SELECT COUNT(*) FROM devices WHERE is_active=1");
        let dev_online = q("SELECT COUNT(*) FROM devices WHERE is_active=1 AND status='online'");
        let dev_offline = q("SELECT COUNT(*) FROM devices WHERE is_active=1 AND status='offline'");
        let dev_error = q("SELECT COUNT(*) FROM devices WHERE is_active=1 AND status='error'");
        let daily: Vec<serde_json::Value> = r.query_rows(
        "SELECT work_date, SUM(CASE WHEN status IN ('Present','Late') THEN 1 ELSE 0 END) hadir,
                SUM(CASE WHEN status='Late' THEN 1 ELSE 0 END) terlambat,
                COUNT(*) total FROM attendance_results WHERE work_date >= date(?1,'-6 days') GROUP BY work_date ORDER BY work_date",
        &[&d],
        |row| Ok(serde_json::json!({"date": row.get::<_, String>(0)?, "hadir": row.get::<_, i64>(1)?, "terlambat": row.get::<_, i64>(2)?, "total": row.get::<_, i64>(3)?})),
    )?;
        let recent: Vec<serde_json::Value> = r.query_rows(
        "SELECT a.work_date, e.name, a.clock_in, a.clock_out, a.status FROM attendance_results a JOIN employees e ON e.id=a.employee_id ORDER BY a.computed_at DESC LIMIT 10",
        &[],
        |row| Ok(serde_json::json!({"date": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?, "in": row.get::<_, Option<String>>(2)?, "out": row.get::<_, Option<String>>(3)?, "status": row.get::<_, String>(4)?})),
    )?;
        let shifts: Vec<serde_json::Value> = r.query_rows(
        "SELECT s.code, s.name, COUNT(a.id) FROM shifts s LEFT JOIN attendance_results a ON a.shift_id=s.id AND a.work_date=?1 GROUP BY s.id",
        &[&d],
        |row| Ok(serde_json::json!({"code": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?, "count": row.get::<_, i64>(2)?})),
    )?;
        let last_sync: Option<String> =
            r.query_one("SELECT MAX(last_sync_at) FROM devices", &[], |x| x.get(0))?;
        let alerts: Vec<serde_json::Value> = r.query_rows(
        "SELECT id, name, status FROM devices WHERE is_active=1 AND status != 'online'",
        &[],
        |row| Ok(serde_json::json!({"id": row.get::<_, String>(0)?, "name": row.get::<_, String>(1)?, "status": row.get::<_, String>(2)?})),
    )?;
        Ok(serde_json::json!({
            "date": d, "active_employees": active_emp, "hadir": hadir, "terlambat": terlambat,
            "tidak_hadir": tidak_hadir, "cuti_izin": cuti,
            "devices": {"total": dev_total, "online": dev_online, "offline": dev_offline, "error": dev_error},
            "daily": daily, "recent": recent, "shifts": shifts,
            "last_sync": last_sync, "alerts": alerts,
        }))
    }

    // ---------- generic table API (dengan permission) ----------
    fn check_table_perm(s: &auth::Session, table: &str, write: bool) -> AppResult<()> {
        let need = match table {
            "employees" | "branches" | "departments" | "positions" | "employee_device_map" => {
                if write {
                    "employee.manage"
                } else {
                    "employee.view"
                }
            }
            "devices" | "device_connection_log" => {
                if write {
                    "device.manage"
                } else {
                    "device.view"
                }
            }
            "shifts" | "shift_assignments" => {
                if write {
                    "shift.manage"
                } else {
                    "shift.view"
                }
            }
            "attendance_results" | "raw_events" => "attendance.view",
            "leave_requests" | "overtime_requests" | "leave_types" => {
                if write {
                    "leave.manage"
                } else {
                    "leave.view"
                }
            }
            "work_calendar" | "holidays" | "attendance_policy" | "shift_swaps" => {
                if write {
                    "shift.manage"
                } else {
                    "shift.view"
                }
            }
            "app_settings" => "user.manage",
            "users" | "audit_log" => {
                if write {
                    "user.manage"
                } else {
                    "user.view"
                }
            }
            _ => {
                return Err(AppError::Validation(format!(
                    "tabel {table} tidak diizinkan"
                )))
            }
        };
        // petakan ke permission model (employee.manage -> employee.*).
        let perms = auth::permissions(&s.role);
        let want: Vec<String> = vec![
            need.to_string(),
            format!("{}.*", need.split('.').next().unwrap_or("")),
        ];
        if want.iter().any(|w| perms.contains(w.as_str())) || (s.role == Role::SuperAdmin) {
            return Ok(());
        }
        // Supervisor boleh lihat attendance; viewer boleh lihat report.
        if !write
            && (need == "attendance.view")
            && (perms.contains("attendance.view") || perms.contains("report.view"))
        {
            return Ok(());
        }
        Err(AppError::Forbidden(format!(
            "role {} tidak punya {need}",
            s.role.as_str()
        )))
    }

    const TABLE_COLUMNS: &[(&str, &[&str])] = &[
        (
            "employees",
            &[
                "id",
                "nik",
                "name",
                "email",
                "phone",
                "address",
                "hire_date",
                "department_id",
                "position_id",
                "branch_id",
                "manager_id",
                "shift_pattern",
                "status",
                "photo_path",
                "deleted_at",
                "created_at",
                "updated_at",
            ],
        ),
        ("branches", &["id", "code", "name", "address", "created_at"]),
        (
            "departments",
            &["id", "code", "name", "branch_id", "created_at"],
        ),
        ("positions", &["id", "code", "name", "created_at"]),
        (
            "shifts",
            &[
                "id",
                "code",
                "name",
                "start_time",
                "end_time",
                "crosses_midnight",
                "break_minutes",
                "late_tolerance_minutes",
                "clockin_early_limit_minutes",
                "clockin_late_limit_minutes",
                "clockout_limit_minutes",
                "overtime_before_minutes",
                "overtime_after_minutes",
                "applicable_days",
                "is_active",
                "version",
                "created_at",
                "updated_at",
            ],
        ),
        (
            "shift_assignments",
            &[
                "id",
                "employee_id",
                "shift_id",
                "effective_date",
                "end_date",
                "created_by",
                "created_at",
            ],
        ),
        (
            "devices",
            &[
                "id",
                "name",
                "vendor",
                "model",
                "serial_number",
                "host",
                "branch_id",
                "location",
                "connection_type",
                "adapter_name",
                "sdk_version",
                "status",
                "last_sync_at",
                "ok_count",
                "fail_count",
                "is_active",
                "config_json",
                "created_at",
                "updated_at",
            ],
        ),
        (
            "device_connection_log",
            &["id", "device_id", "event", "detail", "created_at"],
        ),
        (
            "raw_events",
            &[
                "id",
                "device_id",
                "vendor_record_id",
                "device_user_id",
                "event_time",
                "direction",
                "payload_json",
                "status",
                "error_text",
                "ingested_at",
            ],
        ),
        (
            "attendance_results",
            &[
                "id",
                "employee_id",
                "work_date",
                "shift_id",
                "shift_version",
                "clock_in",
                "clock_out",
                "work_minutes",
                "break_minutes",
                "late_minutes",
                "early_leave_minutes",
                "overtime_minutes",
                "overtime_approved",
                "status",
                "anomaly",
                "source",
                "computed_at",
            ],
        ),
        (
            "employee_device_map",
            &[
                "id",
                "employee_id",
                "device_id",
                "device_user_id",
                "created_at",
            ],
        ),
        (
            "leave_types",
            &["id", "code", "name", "quota_days", "created_at"],
        ),
        (
            "leave_requests",
            &[
                "id",
                "employee_id",
                "leave_type_id",
                "start_date",
                "end_date",
                "reason",
                "attachment_path",
                "status",
                "approved_by",
                "decided_at",
                "created_at",
            ],
        ),
        (
            "overtime_requests",
            &[
                "id",
                "employee_id",
                "work_date",
                "minutes_requested",
                "reason",
                "status",
                "approved_by",
                "decided_at",
                "created_at",
            ],
        ),
        (
            "users",
            &[
                "id",
                "username",
                "display_name",
                "role",
                "is_active",
                "last_login_at",
                "created_at",
            ],
        ),
        (
            "audit_log",
            &[
                "id",
                "actor",
                "action",
                "entity",
                "entity_id",
                "detail",
                "created_at",
            ],
        ),
        (
            "work_calendar",
            &["id", "work_date", "kind", "description", "created_at"],
        ),
        (
            "holidays",
            &["id", "holiday_date", "name", "kind", "created_at"],
        ),
        (
            "leave_types",
            &["id", "code", "name", "quota_days", "created_at"],
        ),
        (
            "attendance_policy",
            &[
                "id",
                "rounding_minutes",
                "max_early_minutes",
                "require_overtime_approval",
                "allow_double_shift",
                "updated_at",
            ],
        ),
        (
            "shift_swaps",
            &[
                "id",
                "employee_id",
                "from_assignment_id",
                "from_shift_id",
                "to_shift_id",
                "swap_date",
                "reason",
                "status",
                "decided_by",
                "decided_at",
                "created_at",
            ],
        ),
        ("app_settings", &["key", "value", "updated_at"]),
    ];

    fn columns_of(table: &str) -> AppResult<&'static [&'static str]> {
        TABLE_COLUMNS
            .iter()
            .find(|(t, _)| *t == table)
            .map(|(_, c)| *c)
            .ok_or_else(|| AppError::Validation(format!("tabel {table} tidak dikenal")))
    }

    #[tauri::command]
    // Tanda tangan IPC Tauri dipetakan 1:1 ke argumen frontend; jumlah parameter mengikuti kontrak API.
    #[allow(clippy::too_many_arguments)]
    pub fn table_list(
        state: tauri::State<AppState>,
        token: String,
        table: String,
        page: i64,
        per_page: i64,
        search: String,
        order_by: String,
        order_dir: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        check_table_perm(&s, &table, false)?;
        let cols = columns_of(&table)?;
        let ob = if cols.contains(&order_by.as_str()) {
            order_by
        } else {
            cols[0].to_string()
        };
        let dir = if order_dir.to_lowercase() == "desc" {
            "DESC"
        } else {
            "ASC"
        };
        // Search sederhana pada kolom teks utama.
        let text_col = match table.as_str() {
            "employees" => "name",
            "devices" => "name",
            "shifts" => "name",
            "branches" => "name",
            "departments" => "name",
            "users" => "username",
            _ => cols[0],
        };
        let like = format!("%{search}%");
        let base = format!(
            "SELECT * FROM {table} WHERE (?1='' OR {text_col} LIKE ?1) ORDER BY {ob} {dir}"
        );
        let count_sql = format!("SELECT COUNT(*) FROM {table} WHERE (?1='' OR {text_col} LIKE ?1)");
        let like_s = like.as_str();
        let (rows, total) = state
            .repo
            .paginate(&base, &count_sql, &[&like_s], page, per_page)?;
        // sembunyikan password_hash bila ada
        Ok(serde_json::json!({"rows": rows, "total": total}))
    }

    #[tauri::command]
    pub fn table_get(
        state: tauri::State<AppState>,
        token: String,
        table: String,
        id: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        check_table_perm(&s, &table, false)?;
        columns_of(&table)?;
        let sql = format!("SELECT * FROM {table} WHERE id=?1");
        let g = state.repo.query_rows(&sql, &[&id], |row| {
            Ok(serde_json::json!({"id": row.get::<_, String>(0)?}))
        })?;
        if g.is_empty() {
            return Err(AppError::NotFound(format!("{table}/{id}")));
        }
        // ambil penuh sebagai map
        let (rows, _) = state.repo.paginate(
            &format!("SELECT * FROM {table} WHERE id=?1"),
            &format!("SELECT COUNT(*) FROM {table} WHERE id=?1"),
            &[&id],
            1,
            1,
        )?;
        Ok(rows.into_iter().next().unwrap_or(serde_json::Value::Null))
    }

    #[tauri::command]
    pub fn table_create(
        state: tauri::State<AppState>,
        token: String,
        table: String,
        data: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        check_table_perm(&s, &table, true)?;
        let cols = columns_of(&table)?;
        if table == "users" {
            return create_user(&state, &s, &data);
        }
        let id = uid();
        let ts = now();
        let mut keys: Vec<String> = vec!["id".into()];
        let mut vals: Vec<String> = vec![id.clone()];
        for c in cols.iter().filter(|c| **c != "id") {
            if let Some(v) = data.get(*c) {
                let sv = match v {
                    serde_json::Value::String(x) => x.clone(),
                    x => x.to_string().trim_matches('"').to_string(),
                };
                keys.push(c.to_string());
                vals.push(sv);
            } else if matches!(
                table.as_str(),
                "employees" | "shifts" | "devices" | "branches" | "departments" | "positions"
            ) && (*c == "created_at" || *c == "updated_at")
            {
                keys.push(c.to_string());
                vals.push(ts.clone());
            }
        }
        validate_required(&table, &data)?;
        let placeholders: Vec<String> = (1..=keys.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            keys.join(","),
            placeholders.join(",")
        );
        state.repo.repo_exec_raw(&sql, &vals)?;
        state
            .repo
            .insert_audit(&s.username, "create", &table, &id, "")?;
        Ok(serde_json::json!({"id": id}))
    }

    fn validate_required(table: &str, data: &serde_json::Value) -> AppResult<()> {
        let need: &[&str] = match table {
            "employees" => &["nik", "name", "hire_date"],
            "devices" => &["name", "vendor", "adapter_name"],
            "shifts" => &["code", "name", "start_time", "end_time"],
            "branches" => &["code", "name"],
            _ => &[],
        };
        for n in need {
            if data
                .get(*n)
                .and_then(|v| v.as_str())
                .map(|x| x.trim().is_empty())
                .unwrap_or(true)
                && data.get(*n).map(|v| v.is_null()).unwrap_or(true)
            {
                return Err(AppError::Validation(format!("kolom {n} wajib diisi")));
            }
        }
        if table == "employees" {
            if let Some(em) = data.get("email").and_then(|v| v.as_str()) {
                if !em.is_empty() && !em.contains('@') {
                    return Err(AppError::Validation("email tidak valid".into()));
                }
            }
        }
        Ok(())
    }

    fn create_user(
        state: &AppState,
        s: &auth::Session,
        data: &serde_json::Value,
    ) -> AppResult<serde_json::Value> {
        if s.role != Role::SuperAdmin {
            return Err(AppError::Forbidden(
                "hanya super_admin yang boleh kelola pengguna".into(),
            ));
        }
        let username = data
            .get("username")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let display = data
            .get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or(&username)
            .to_string();
        let role = data
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("viewer")
            .to_string();
        let password = data
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if Role::from_code(&role).is_none() {
            return Err(AppError::Validation("role tidak dikenal".into()));
        }
        let hash = auth::hash_password(&password)?;
        let id = uid();
        state.repo.exec(
        "INSERT INTO users (id, username, display_name, role, password_hash, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
        &[&id, &username, &display, &role, &hash, &now()],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") { AppError::Conflict("username sudah dipakai".into()) } else { e }
    })?;
        state
            .repo
            .insert_audit(&s.username, "create_user", "users", &id, &role)?;
        Ok(serde_json::json!({"id": id}))
    }

    #[tauri::command]
    pub fn table_update(
        state: tauri::State<AppState>,
        token: String,
        table: String,
        id: String,
        data: serde_json::Value,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        check_table_perm(&s, &table, true)?;
        let cols = columns_of(&table)?;
        if table == "attendance_results" {
            return Err(AppError::Forbidden(
                "gunakan correction untuk mengubah hasil absensi (audit wajib)".into(),
            ));
        }
        let mut sets: Vec<String> = vec![];
        let mut vals: Vec<String> = vec![];
        for (k, v) in data.as_object().cloned().unwrap_or_default() {
            if k == "id" || !cols.contains(&k.as_str()) {
                continue;
            }
            if k == "password_hash" || k == "password" {
                continue;
            }
            sets.push(format!("{k}=?"));
            vals.push(match v {
                serde_json::Value::String(x) => x,
                x => x.to_string(),
            });
        }
        if sets.is_empty() {
            return Err(AppError::Validation("tidak ada kolom yang diubah".into()));
        }
        if cols.contains(&"updated_at") {
            sets.push("updated_at=datetime('now','localtime')".into());
        }
        let sql = format!("UPDATE {table} SET {} WHERE id=?", sets.join(","));
        vals.push(id.clone());
        // Ambil nilai lama untuk riwayat (karyawan & shift).
        let old_vals: Vec<(String, String)> = if table == "employees" || table == "shifts" {
            let keys: Vec<String> = sets
                .iter()
                .filter_map(|x| x.strip_suffix("=?").map(|k| k.to_string()))
                .collect();
            let mut out = vec![];
            for k in keys {
                let q = format!("SELECT {k} FROM {table} WHERE id=?1");
                let old: Option<String> = state
                    .repo
                    .query_one(&q, &[&id], |r| r.get(0))
                    .unwrap_or(None);
                out.push((k, old.unwrap_or_default()));
            }
            out
        } else {
            vec![]
        };
        state.repo.repo_exec_raw(&sql, &vals)?;
        if table == "employees" {
            let reason = data
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ts = now();
            for (k, old) in old_vals {
                let new_v = data.get(&k).map(|v| match v {
                    serde_json::Value::String(x) => x.clone(),
                    x => x.to_string(),
                });
                if let Some(nv) = new_v {
                    if nv != old {
                        let hid = uid();
                        state.repo.exec(
                            "INSERT INTO employee_history (id, employee_id, changed_by, field_name, old_value, new_value, reason, changed_at)
                             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                            &[&hid, &id, &s.username, &k, &old, &nv, &reason, &ts],
                        )?;
                    }
                }
            }
        }
        if table == "shifts" {
            state
                .repo
                .exec("UPDATE shifts SET version=version+1 WHERE id=?1", &[&id])?;
            let payload: String = state
                .repo
                .query_one(
                    "SELECT printf('{\"code\":\"%s\",\"name\":\"%s\",\"start\":\"%s\",\"end\":\"%s\",\"cross\":%d,\"tol\":%d}',
                     code, name, start_time, end_time, crosses_midnight, late_tolerance_minutes)
                     FROM shifts WHERE id=?1",
                    &[&id],
                    |r| r.get(0),
                )?
                .unwrap_or_else(|| "{}".into());
            let ver: i64 = state
                .repo
                .query_one("SELECT version FROM shifts WHERE id=?1", &[&id], |r| {
                    r.get(0)
                })?
                .unwrap_or(1);
            let hid = uid();
            state.repo.exec(
                "INSERT INTO shift_history (id, shift_id, version, payload_json, changed_by, changed_at)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                &[&hid, &id, &ver.to_string(), &payload, &s.username, &now()],
            )?;
        }
        state
            .repo
            .insert_audit(&s.username, "update", &table, &id, "")?;
        Ok("ok".into())
    }

    #[tauri::command]
    pub fn table_delete(
        state: tauri::State<AppState>,
        token: String,
        table: String,
        id: String,
        hard: bool,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        check_table_perm(&s, &table, true)?;
        if table == "employees" && !hard {
            state.repo.exec(
                "UPDATE employees SET deleted_at=?1, status='nonaktif' WHERE id=?2",
                &[&now(), &id],
            )?;
        } else if table == "raw_events" || table == "attendance_results" {
            return Err(AppError::Forbidden(
                "raw events & hasil absensi tidak boleh dihapus via UI".into(),
            ));
        } else {
            let sql = format!("DELETE FROM {table} WHERE id=?1");
            state.repo.exec(&sql, &[&id])?;
        }
        state.repo.insert_audit(
            &s.username,
            "delete",
            &table,
            &id,
            if hard { "hard" } else { "soft" },
        )?;
        Ok("ok".into())
    }

    // ---------- devices: test & sync ----------
    #[tauri::command]
    pub fn device_test(
        state: tauri::State<AppState>,
        token: String,
        device_id: String,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&state, &token, "device.view")?;
        let row: Option<(String, String, String, String)> = state.repo.query_one(
            "SELECT id, host, adapter_name, config_json FROM devices WHERE id=?1",
            &[&device_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let Some((did, host, adapter_name, config_json)) = row else {
            return Err(AppError::NotFound("perangkat".into()));
        };
        let mut a = sync::service::build_adapter(&adapter_name, &config_json);
        let t0 = std::time::Instant::now();
        let host_s: &str = if host.is_empty() {
            "local"
        } else {
            host.as_str()
        };
        let res = (|| -> Result<serde_json::Value, String> {
            a.initialize().map_err(|e| e.to_string())?;
            a.connect(host_s, std::time::Duration::from_secs(8))
                .map_err(|e| e.to_string())?;
            let st = a.health_check().map_err(|e| e.to_string())?;
            let info = a.fetch_device_info().map_err(|e| e.to_string())?;
            let _ = a.disconnect();
            Ok(
                serde_json::json!({"status": format!("{st:?}"), "info": info, "sdk": a.get_sdk_version(), "ms": t0.elapsed().as_millis()}),
            )
        })();
        let now_s = now();
        match res {
            Ok(v) => {
                state
                    .repo
                    .exec("UPDATE devices SET status='online' WHERE id=?1", &[&did])?;
                state.repo.exec("INSERT INTO device_connection_log (id, device_id, event, detail, created_at) VALUES (?1,?2,'test_ok',?3,?4)",
                &[&uid(), &did, &v.to_string(), &now_s])?;
                Ok(v)
            }
            Err(e) => {
                state
                    .repo
                    .exec("UPDATE devices SET status='error' WHERE id=?1", &[&did])?;
                state.repo.exec("INSERT INTO device_connection_log (id, device_id, event, detail, created_at) VALUES (?1,?2,'test_fail',?3,?4)",
                &[&uid(), &did, &e, &now_s])?;
                Err(AppError::Device(e))
            }
        }
    }

    #[tauri::command]
    pub fn device_sync(
        state: tauri::State<AppState>,
        token: String,
        device_id: String,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&state, &token, "sync.*")
            .or_else(|_| require(&state, &token, "device.manage"))?;
        let (adapter_name, config_json): (String, String) = state
            .repo
            .query_one(
                "SELECT adapter_name, config_json FROM devices WHERE id=?1",
                &[&device_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?
            .ok_or_else(|| AppError::NotFound("perangkat".into()))?;
        let mut a = sync::service::build_adapter(&adapter_name, &config_json);
        let cancel = AtomicBool::new(false);
        let stats = sync::service::sync_device(&state.repo, &device_id, &mut a, 3, 200, &cancel)?;
        Ok(serde_json::to_value(stats).unwrap())
    }

    #[tauri::command]
    pub fn device_registry() -> Result<serde_json::Value, AppError> {
        Ok(serde_json::to_value(devices::registry::DeviceRegistry::entries()).unwrap())
    }

    /// Status DLL vendor di mesin ini (ditemukan? arsitektur x64? terdaftar/
    /// termuat?) — tanpa butuh perangkat. Untuk indikator "SDK belum dipasang".
    #[tauri::command]
    pub fn device_sdk_status(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&state, &token, "device.view")?;
        Ok(serde_json::json!([
            devices::zkcom::zkemkeeper_status(),
            devices::hikvision::hcnet_status(),
        ]))
    }

    // ---------- attendance: compute & review & correction ----------
    fn load_shift(repo: &Repo, shift_id: &str) -> AppResult<attendance::ShiftRule> {
        let v: Option<serde_json::Value> = repo.query_one(
        "SELECT code, name, start_time, end_time, crosses_midnight, break_minutes, late_tolerance_minutes, clockin_early_limit_minutes, clockin_late_limit_minutes, clockout_limit_minutes, overtime_before_minutes, overtime_after_minutes, applicable_days, version FROM shifts WHERE id=?1",
        &[&shift_id],
        |r| Ok(serde_json::json!({
            "code": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?,
            "start_time": r.get::<_, String>(2)?, "end_time": r.get::<_, String>(3)?,
            "crosses_midnight": r.get::<_, i64>(4)?, "break_minutes": r.get::<_, i64>(5)?,
            "late_tolerance_minutes": r.get::<_, i64>(6)?,
            "clockin_early_limit_minutes": r.get::<_, i64>(7)?,
            "clockin_late_limit_minutes": r.get::<_, i64>(8)?,
            "clockout_limit_minutes": r.get::<_, i64>(9)?,
            "overtime_before_minutes": r.get::<_, i64>(10)?,
            "overtime_after_minutes": r.get::<_, i64>(11)?,
            "applicable_days": r.get::<_, String>(12)?, "version": r.get::<_, i64>(13)?,
        })),
    )?;
        v.map(|x| shift_from_value(&x))
            .ok_or_else(|| AppError::NotFound("shift".into()))
    }

    #[tauri::command]
    pub fn attendance_compute(
        state: tauri::State<AppState>,
        token: String,
        work_date: String,
        employee_id: Option<String>,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&state, &token, "attendance.view")?;
        // Tentukan daftar karyawan.
        let emps: Vec<(String,)> = if let Some(e) = employee_id {
            vec![(e,)]
        } else {
            state.repo.query_rows(
                "SELECT id FROM employees WHERE status='aktif' AND deleted_at IS NULL",
                &[],
                |r| Ok((r.get(0)?,)),
            )?
        };
        let policy: (i64, i64) = state.repo.query_one(
        "SELECT rounding_minutes, require_overtime_approval FROM attendance_policy WHERE id='default'", &[],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?.unwrap_or((1, 1));
        let wd = check_work_date(&work_date)?;
        let is_holiday: bool = state
            .repo
            .query_one(
                "SELECT COUNT(*) FROM holidays WHERE holiday_date=?1",
                &[&work_date],
                |r| {
                    let n: i64 = r.get(0)?;
                    Ok(n > 0)
                },
            )?
            .unwrap_or(false);
        let mut done = 0i64;
        for (eid,) in emps {
            // assignment efektif terbaru
            let shift_id: Option<(String,)> = state.repo.query_one(
            "SELECT shift_id FROM shift_assignments WHERE employee_id=?1 AND effective_date<=?2 AND (end_date IS NULL OR end_date>=?2) ORDER BY effective_date DESC LIMIT 1",
            &[&eid, &work_date],
            |r| Ok((r.get(0)?,)),
        )?;
            let shift_id = match shift_id {
                Some((s,)) => s,
                None => {
                    // fallback pola shift karyawan
                    let pat: Option<String> = state.repo.query_one(
                        "SELECT shift_pattern FROM employees WHERE id=?1",
                        &[&eid],
                        |r| r.get(0),
                    )?;
                    let code = pat.unwrap_or("PAGI".into());
                    state
                        .repo
                        .query_one("SELECT id FROM shifts WHERE code=?1", &[&code], |r| {
                            r.get(0)
                        })?
                        .unwrap_or("sh-pagi".into())
                }
            };
            let shift = load_shift(&state.repo, &shift_id)?;
            // kumpulkan event karyawan (via mapping + event langsung bila device_user cocok nik? hanya via mapping)
            let maps: Vec<String> = state.repo.query_rows(
                "SELECT device_user_id FROM employee_device_map WHERE employee_id=?1",
                &[&eid],
                |r| r.get(0),
            )?;
            let mut ev_times: Vec<chrono::NaiveDateTime> = vec![];
            let mut used_ids: Vec<String> = vec![];
            for du in &maps {
                let rows: Vec<(String, String)> = state.repo.query_rows(
                "SELECT id, event_time FROM raw_events WHERE device_user_id=?1 AND event_time >= ?2 AND event_time <= ?3 ORDER BY event_time",
                &[du, &format!("{work_date} 00:00:00"), &format!("{} 23:59:59", (wd + chrono::Duration::days(1)).format("%Y-%m-%d"))],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
                for (rid, t) in rows {
                    if let Some(dt) = timex::parse_wib(&t) {
                        ev_times.push(dt);
                        used_ids.push(rid);
                    }
                }
            }
            let grouped = attendance::group_events_for_shift(&shift, &wd, ev_times);
            let on_leave: bool = state.repo.query_one(
            "SELECT COUNT(*) FROM leave_requests WHERE employee_id=?1 AND status='approved' AND start_date<=?2 AND end_date>=?2",
            &[&eid, &work_date], |r| { let n: i64 = r.get(0)?; Ok(n > 0) })?.unwrap_or(false);
            let ot_appr: i64 = state.repo.query_one(
            "SELECT COALESCE(SUM(minutes_requested),0) FROM overtime_requests WHERE employee_id=?1 AND work_date=?2 AND status='approved'",
            &[&eid, &work_date], |r| r.get(0))?.unwrap_or(0);
            let inp = attendance::ComputeInput {
                work_date: wd,
                shift: shift.clone(),
                events: grouped,
                is_holiday,
                is_day_off: false,
                on_leave,
                on_permission: false,
                rounding_minutes: policy.0,
                overtime_approved_minutes: ot_appr,
                require_overtime_approval: policy.1 != 0,
            };
            let o = attendance::compute_day(&inp);
            let id = format!("{eid}-{work_date}");
            state.repo.exec(
            "INSERT INTO attendance_results (id, employee_id, work_date, shift_id, shift_version, clock_in, clock_out, work_minutes, break_minutes, late_minutes, early_leave_minutes, overtime_minutes, overtime_approved, status, anomaly, source, computed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)
             ON CONFLICT(employee_id, work_date) DO UPDATE SET shift_id=excluded.shift_id, shift_version=excluded.shift_version, clock_in=excluded.clock_in, clock_out=excluded.clock_out, work_minutes=excluded.work_minutes, break_minutes=excluded.break_minutes, late_minutes=excluded.late_minutes, early_leave_minutes=excluded.early_leave_minutes, overtime_minutes=excluded.overtime_minutes, overtime_approved=excluded.overtime_approved, status=excluded.status, anomaly=excluded.anomaly, source=excluded.source, computed_at=excluded.computed_at",
            &[&id, &eid, &work_date, &shift_id, &shift.version.to_string(),
              &o.clock_in.unwrap_or_default(), &o.clock_out.unwrap_or_default(),
              &o.work_minutes.to_string(), &o.break_minutes.to_string(), &o.late_minutes.to_string(),
              &o.early_leave_minutes.to_string(), &o.overtime_minutes.to_string(),
              &(if o.overtime_approved { "1" } else { "0" }).to_string(),
              &o.status.as_str().to_string(), &o.anomaly, &o.source, &now()],
        )?;
            // Tandai event yang dipakai agar review hanya menampilkan yang belum diproses.
            if !used_ids.is_empty() {
                let ph: Vec<String> = (1..=used_ids.len()).map(|i| format!("?{i}")).collect();
                let sql = format!(
                    "UPDATE raw_events SET status='processed' WHERE id IN ({})",
                    ph.join(",")
                );
                state.repo.repo_exec_raw(&sql, &used_ids)?;
            }
            done += 1;
        }
        log::info!("hitung absensi {work_date}: {done} karyawan");
        Ok(serde_json::json!({"computed": done, "date": work_date}))
    }

    #[tauri::command]
    pub fn attendance_review(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&state, &token, "attendance.view")?;
        let rows: Vec<serde_json::Value> = state.repo.query_rows(
        "SELECT a.id, e.name, a.work_date, a.status, a.anomaly, a.clock_in, a.clock_out FROM attendance_results a JOIN employees e ON e.id=a.employee_id WHERE a.status IN ('Incomplete','NeedsReview','Absent') ORDER BY a.work_date DESC LIMIT 100",
        &[],
        |r| Ok(serde_json::json!({"id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "date": r.get::<_, String>(2)?, "status": r.get::<_, String>(3)?, "anomaly": r.get::<_, String>(4)?, "in": r.get::<_, String>(5)?, "out": r.get::<_, String>(6)?})),
    )?;
        let unmapped: Vec<serde_json::Value> = state.repo.query_rows(
        "SELECT id, device_id, device_user_id, event_time FROM raw_events WHERE status='pending' AND device_user_id NOT IN (SELECT device_user_id FROM employee_device_map) ORDER BY event_time DESC LIMIT 100",
        &[],
        |r| Ok(serde_json::json!({"id": r.get::<_, String>(0)?, "device": r.get::<_, String>(1)?, "device_user": r.get::<_, String>(2)?, "time": r.get::<_, String>(3)?})),
    )?;
        Ok(serde_json::json!({"review": rows, "unmapped": unmapped}))
    }

    #[tauri::command]
    pub fn attendance_correct(
        state: tauri::State<AppState>,
        token: String,
        attendance_id: String,
        field: String,
        new_value: String,
        reason: String,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "correction.*")
            .or_else(|_| require(&state, &token, "attendance.manage"))?;
        if reason.trim().is_empty() {
            return Err(AppError::Validation("alasan wajib diisi".into()));
        }
        let allowed = ["clock_in", "clock_out", "status"];
        if !allowed.contains(&field.as_str()) {
            return Err(AppError::Validation("kolom tidak boleh dikoreksi".into()));
        }
        let old: Option<String> = state.repo.query_one(
            &format!("SELECT {field} FROM attendance_results WHERE id=?1"),
            &[&attendance_id],
            |r| r.get(0),
        )?;
        let old = old.ok_or_else(|| AppError::NotFound("absensi".into()))?;
        let sql = format!("UPDATE attendance_results SET {field}=?1, computed_at=?2 WHERE id=?3");
        let ts = now();
        state.repo.exec(&sql, &[&new_value, &ts, &attendance_id])?;
        let cid = uid();
        state.repo.exec(
        "INSERT INTO corrections_audit (id, attendance_id, changed_by, field_name, old_value, new_value, reason, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        &[&cid, &attendance_id, &s.username, &field, &old, &new_value, &reason, &ts],
    )?;
        state.repo.insert_audit(
            &s.username,
            "correct",
            "attendance_results",
            &attendance_id,
            &format!("{field}: {old} -> {new_value} | {reason}"),
        )?;
        Ok("ok".into())
    }

    // ---------- pengaturan ----------
    fn setting_get(repo: &Repo, key: &str) -> String {
        repo.query_one::<String, _>(
            "SELECT value FROM app_settings WHERE key=?1",
            &[&key.to_string()],
            |r| r.get(0),
        )
        .unwrap_or(None)
        .unwrap_or_default()
    }

    #[tauri::command]
    pub fn settings_get(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let keys = [
            "sync_interval_secs",
            "backup_keep_files",
            "session_timeout_secs",
            "scheduler_enabled",
            "backup_auto",
        ];
        let mut m = serde_json::Map::new();
        for k in keys {
            m.insert(
                k.to_string(),
                serde_json::Value::String(setting_get(&state.repo, k)),
            );
        }
        m.insert(
            "session_timeout_active".to_string(),
            serde_json::json!(state.sessions.timeout()),
        );
        Ok(serde_json::Value::Object(m))
    }

    #[tauri::command]
    pub fn settings_set(
        state: tauri::State<AppState>,
        token: String,
        key: String,
        value: String,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "user.manage")?;
        if s.role != Role::SuperAdmin {
            return Err(AppError::Forbidden(
                "hanya super_admin yang boleh mengubah pengaturan".into(),
            ));
        }
        let v = value.trim().to_string();
        match key.as_str() {
            "sync_interval_secs" => {
                let n: u64 = v
                    .parse()
                    .map_err(|_| AppError::Validation("harus angka detik >= 60".into()))?;
                if n < 60 {
                    return Err(AppError::Validation("minimal 60 detik".into()));
                }
            }
            "backup_keep_files" => {
                let n: i64 = v
                    .parse()
                    .map_err(|_| AppError::Validation("harus angka 1-100".into()))?;
                if !(1..=100).contains(&n) {
                    return Err(AppError::Validation("rentang 1-100".into()));
                }
            }
            "session_timeout_secs" => {
                let n: u64 = v
                    .parse()
                    .map_err(|_| AppError::Validation("harus angka detik >= 60".into()))?;
                if n < 60 {
                    return Err(AppError::Validation("minimal 60 detik".into()));
                }
                state.sessions.set_timeout(n);
            }
            "scheduler_enabled" => {
                if v != "0" && v != "1" {
                    return Err(AppError::Validation("harus 0 atau 1".into()));
                }
            }
            "backup_auto" => {
                if v != "0" && v != "1" {
                    return Err(AppError::Validation("harus 0 atau 1".into()));
                }
            }
            _ => {
                return Err(AppError::Validation(
                    "kunci pengaturan tidak dikenal".into(),
                ))
            }
        }
        let ts = now();
        state.repo.exec(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1,?2,?3)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            &[&key, &v, &ts],
        )?;
        state
            .repo
            .insert_audit(&s.username, "settings_set", "app_settings", &key, &v)?;
        Ok("ok".into())
    }

    // ---------- jadwal shift ----------
    fn parse_date(s: &str) -> AppResult<chrono::NaiveDate> {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| AppError::Validation(format!("tanggal harus YYYY-MM-DD: {s}")))
    }

    /// Tanggal kerja tidak boleh masa depan: hari yang belum selesai
    /// tidak boleh disimpulkan Absent.
    fn check_work_date(s: &str) -> AppResult<chrono::NaiveDate> {
        let d = parse_date(s)?;
        if d > timex::today_wib() {
            return Err(AppError::Validation(
                "tanggal kerja belum selesai (masa depan WIB)".into(),
            ));
        }
        Ok(d)
    }

    fn shift_code(repo: &Repo, shift_id: &str) -> String {
        repo.query_one::<String, _>(
            "SELECT code FROM shifts WHERE id=?1",
            &[&shift_id.to_string()],
            |r| r.get(0),
        )
        .unwrap_or(None)
        .unwrap_or_else(|| "?".into())
    }

    fn effective_shift(repo: &Repo, employee_id: &str, date: &str) -> Option<(String, String)> {
        repo.query_one(
            "SELECT shift_id, id FROM shift_assignments WHERE employee_id=?1 AND effective_date<=?2 AND (end_date IS NULL OR end_date>=?2) ORDER BY effective_date DESC LIMIT 1",
            &[&employee_id.to_string(), &date.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or(None)
    }

    #[tauri::command]
    pub fn schedule_week(
        state: tauri::State<AppState>,
        token: String,
        start_date: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let start = parse_date(&start_date)?;
        let days: Vec<String> = (0..7)
            .map(|i| {
                (start + chrono::Duration::days(i))
                    .format("%Y-%m-%d")
                    .to_string()
            })
            .collect();
        let emps: Vec<(String, String)> = state.repo.query_rows(
            "SELECT id, name FROM employees WHERE status='aktif' AND deleted_at IS NULL ORDER BY name",
            &[],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut rows = vec![];
        let mut empty_days = vec![];
        for d in &days {
            let n: i64 = state
                .repo
                .query_one(
                    "SELECT COUNT(*) FROM shift_assignments WHERE effective_date<=?1 AND (end_date IS NULL OR end_date>=?1)",
                    &[&d.clone()],
                    |r| r.get(0),
                )?
                .unwrap_or(0);
            if n == 0 {
                empty_days.push(d.clone());
            }
        }
        for (eid, name) in emps {
            let mut cells = serde_json::Map::new();
            for d in &days {
                if let Some((sid, _)) = effective_shift(&state.repo, &eid, d) {
                    cells.insert(
                        d.clone(),
                        serde_json::Value::String(shift_code(&state.repo, &sid)),
                    );
                } else {
                    cells.insert(d.clone(), serde_json::Value::Null);
                }
            }
            rows.push(serde_json::json!({"employee_id": eid, "name": name, "cells": cells}));
        }
        Ok(serde_json::json!({"days": days, "rows": rows, "empty_days": empty_days}))
    }

    #[tauri::command]
    pub fn schedule_bulk(
        state: tauri::State<AppState>,
        token: String,
        employee_ids: Vec<String>,
        shift_id: String,
        start_date: String,
        end_date: String,
        dry_run: bool,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "shift.manage")?;
        let start = parse_date(&start_date)?;
        let end = parse_date(&end_date)?;
        if end < start {
            return Err(AppError::Validation(
                "tanggal akhir sebelum tanggal mulai".into(),
            ));
        }
        let exists: i64 = state
            .repo
            .query_one(
                "SELECT COUNT(*) FROM shifts WHERE id=?1",
                &[&shift_id],
                |r| r.get(0),
            )?
            .unwrap_or(0);
        if exists == 0 {
            return Err(AppError::NotFound("shift".into()));
        }
        if employee_ids.is_empty() {
            return Err(AppError::Validation("pilih minimal satu karyawan".into()));
        }
        let mut created = 0i64;
        let mut skipped = vec![];
        for eid in &employee_ids {
            // Bentrok: sudah ada penugasan shift LAIN yang tumpang tindih rentang.
            let clash: Vec<(String, String, String)> = state.repo.query_rows(
                "SELECT s.code, a.effective_date, COALESCE(a.end_date,'') FROM shift_assignments a JOIN shifts s ON s.id=a.shift_id
                 WHERE a.employee_id=?1 AND a.shift_id<>?2 AND a.effective_date<=?3 AND (a.end_date IS NULL OR a.end_date>=?4)",
                &[&eid.clone(), &shift_id, &end_date, &start_date],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            if !clash.is_empty() {
                skipped.push(serde_json::json!({
                    "employee_id": eid,
                    "reason": format!("bentrok dengan shift {} ({} s/d {})", clash[0].0, clash[0].1, if clash[0].2.is_empty() { "…" } else { &clash[0].2 }),
                }));
                continue;
            }
            if !dry_run {
                let id = uid();
                state.repo.exec(
                    "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, end_date, created_by, created_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    &[&id, eid, &shift_id, &start_date, &end_date, &s.username, &now()],
                )?;
            }
            created += 1;
        }
        if !dry_run {
            state.repo.insert_audit(
                &s.username,
                "schedule_bulk",
                "shift_assignments",
                &shift_id,
                &format!("{} s/d {}: {created} dibuat", start_date, end_date),
            )?;
        }
        Ok(serde_json::json!({"created": created, "skipped": skipped, "dry_run": dry_run}))
    }

    #[tauri::command]
    pub fn schedule_copy_week(
        state: tauri::State<AppState>,
        token: String,
        src_start: String,
        dst_start: String,
        dry_run: bool,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "shift.manage")?;
        let src = parse_date(&src_start)?;
        let dst = parse_date(&dst_start)?;
        if dst <= src {
            return Err(AppError::Validation(
                "minggu tujuan harus setelah minggu sumber".into(),
            ));
        }
        let delta = (dst - src).num_days();
        let src_end = (src + chrono::Duration::days(6))
            .format("%Y-%m-%d")
            .to_string();
        let rows: Vec<(String, String, String, Option<String>)> = state.repo.query_rows(
            "SELECT employee_id, shift_id, effective_date, end_date FROM shift_assignments
             WHERE effective_date<=?1 AND (end_date IS NULL OR end_date>=?2)",
            &[&src_end, &src_start],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let mut copied = 0i64;
        let mut skipped = 0i64;
        for (eid, sid, eff, end) in rows {
            let eff_d = parse_date(&eff)?;
            let new_eff = (eff_d + chrono::Duration::days(delta))
                .format("%Y-%m-%d")
                .to_string();
            let new_end = match end {
                Some(e) => {
                    let e_d = parse_date(&e)?;
                    (e_d + chrono::Duration::days(delta))
                        .format("%Y-%m-%d")
                        .to_string()
                }
                None => String::new(),
            };
            let dup: i64 = state
                .repo
                .query_one(
                    "SELECT COUNT(*) FROM shift_assignments WHERE employee_id=?1 AND shift_id=?2 AND effective_date=?3",
                    &[&eid, &sid, &new_eff],
                    |r| r.get(0),
                )?
                .unwrap_or(0);
            if dup > 0 {
                skipped += 1;
                continue;
            }
            if !dry_run {
                let id = uid();
                let new_end_opt: Option<String> = if new_end.is_empty() {
                    None
                } else {
                    Some(new_end.clone())
                };
                // end_date NULL bila kosong: dua cabang agar binding aman.
                if new_end_opt.is_none() {
                    state.repo.exec(
                        "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, end_date, created_by, created_at)
                         VALUES (?1,?2,?3,?4,NULL,?5,?6)",
                        &[&id, &eid, &sid, &new_eff, &s.username, &now()],
                    )?;
                } else {
                    state.repo.exec(
                        "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, end_date, created_by, created_at)
                         VALUES (?1,?2,?3,?4,?5,?6,?7)",
                        &[&id, &eid, &sid, &new_eff, &new_end, &s.username, &now()],
                    )?;
                }
            }
            copied += 1;
        }
        if !dry_run {
            state.repo.insert_audit(
                &s.username,
                "schedule_copy_week",
                "shift_assignments",
                &format!("{src_start}->{dst_start}"),
                &format!("{copied} disalin, {skipped} dilewati"),
            )?;
        }
        Ok(serde_json::json!({"copied": copied, "skipped": skipped, "dry_run": dry_run}))
    }

    #[tauri::command]
    pub fn schedule_conflicts(
        state: tauri::State<AppState>,
        token: String,
        start_date: String,
        end_date: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let start = parse_date(&start_date)?;
        let end = parse_date(&end_date)?;
        if end < start {
            return Err(AppError::Validation("rentang tanggal terbalik".into()));
        }
        // Tumpang tindih: dua penugasan shift berbeda untuk karyawan yang sama pada tanggal yang sama.
        let overlaps: Vec<serde_json::Value> = state.repo.query_rows(
            "SELECT a.employee_id, e.name, s1.code, s2.code, MAX(a.effective_date, b.effective_date)
             FROM shift_assignments a
             JOIN shift_assignments b ON b.employee_id=a.employee_id AND b.id<>a.id AND b.shift_id<>a.shift_id
               AND b.effective_date<=COALESCE(a.end_date,'9999-12-31') AND COALESCE(b.end_date,'9999-12-31')>=a.effective_date
             JOIN employees e ON e.id=a.employee_id
             JOIN shifts s1 ON s1.id=a.shift_id
             JOIN shifts s2 ON s2.id=b.shift_id
             WHERE a.effective_date<=?1 AND COALESCE(a.end_date,'9999-12-31')>=?2
             GROUP BY a.employee_id",
            &[&end_date, &start_date],
            |r| {
                Ok(serde_json::json!({
                    "employee_id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "shift_a": r.get::<_, String>(2)?,
                    "shift_b": r.get::<_, String>(3)?,
                    "since": r.get::<_, String>(4)?,
                }))
            },
        )?;
        // Jadwal kosong: karyawan aktif tanpa penugasan efektif pada tiap tanggal.
        let emps: Vec<(String, String)> = state.repo.query_rows(
            "SELECT id, name FROM employees WHERE status='aktif' AND deleted_at IS NULL",
            &[],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut empty = vec![];
        let mut d = start;
        while d <= end {
            let ds = d.format("%Y-%m-%d").to_string();
            for (eid, name) in &emps {
                if effective_shift(&state.repo, eid, &ds).is_none() {
                    empty.push(serde_json::json!({"date": ds, "employee_id": eid, "name": name}));
                }
            }
            d += chrono::Duration::days(1);
        }
        Ok(serde_json::json!({"overlaps": overlaps, "empty": empty}))
    }

    // ---------- tukar shift ----------
    #[tauri::command]
    pub fn swap_request(
        state: tauri::State<AppState>,
        token: String,
        employee_id: String,
        from_shift_id: String,
        to_shift_id: String,
        swap_date: String,
        reason: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "shift.manage")?;
        parse_date(&swap_date)?;
        if from_shift_id == to_shift_id {
            return Err(AppError::Validation("shift asal dan tujuan sama".into()));
        }
        if reason.trim().is_empty() {
            return Err(AppError::Validation("alasan wajib diisi".into()));
        }
        let from_assign: Option<String> = state
            .repo
            .query_one(
                "SELECT id FROM shift_assignments WHERE employee_id=?1 AND shift_id=?2 AND effective_date<=?3 AND (end_date IS NULL OR end_date>=?3) ORDER BY effective_date DESC LIMIT 1",
                &[&employee_id, &from_shift_id, &swap_date],
                |r| r.get(0),
            )?;
        let id = uid();
        state.repo.exec(
            "INSERT INTO shift_swaps (id, employee_id, from_assignment_id, from_shift_id, to_shift_id, swap_date, reason, status, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'pending',?8)",
            &[
                &id,
                &employee_id,
                &from_assign.unwrap_or_default(),
                &from_shift_id,
                &to_shift_id,
                &swap_date,
                &reason,
                &now(),
            ],
        )?;
        state.repo.insert_audit(
            &s.username,
            "swap_request",
            "shift_swaps",
            &id,
            &format!("{employee_id} {from_shift_id}->{to_shift_id} @ {swap_date}"),
        )?;
        Ok(serde_json::json!({"id": id}))
    }

    #[tauri::command]
    pub fn swap_decide(
        state: tauri::State<AppState>,
        token: String,
        swap_id: String,
        approve: bool,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "shift.manage")?;
        let row: Option<(String, String, String, String)> = state.repo.query_one(
            "SELECT employee_id, to_shift_id, swap_date, status FROM shift_swaps WHERE id=?1",
            &[&swap_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let Some((eid, to_sid, sdate, status)) = row else {
            return Err(AppError::NotFound("pengajuan tukar".into()));
        };
        if status != "pending" {
            return Err(AppError::Conflict("pengajuan sudah diputuskan".into()));
        }
        let ts = now();
        let new_status = if approve { "approved" } else { "rejected" };
        state.repo.exec(
            "UPDATE shift_swaps SET status=?1, decided_by=?2, decided_at=?3 WHERE id=?4",
            &[&new_status.to_string(), &s.username, &ts, &swap_id],
        )?;
        if approve {
            // Penugasan satu hari pada tanggal swap; histori penugasan lama tetap utuh.
            let aid = uid();
            state.repo.exec(
                "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, end_date, created_by, created_at)
                 VALUES (?1,?2,?3,?4,?4,?5,?6)",
                &[&aid, &eid, &to_sid, &sdate, &s.username, &ts],
            )?;
        }
        state.repo.insert_audit(
            &s.username,
            "swap_decide",
            "shift_swaps",
            &swap_id,
            new_status,
        )?;
        Ok("ok".into())
    }

    // ---------- sinkronisasi massal + scheduler ----------
    fn sync_one(repo: &Repo, device_id: &str) -> Result<sync::service::SyncStats, AppError> {
        let (adapter_name, config_json): (String, String) = repo
            .query_one(
                "SELECT adapter_name, config_json FROM devices WHERE id=?1",
                &[&device_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?
            .ok_or_else(|| AppError::NotFound("perangkat".into()))?;
        let mut a = sync::service::build_adapter(&adapter_name, &config_json);
        let cancel = AtomicBool::new(false);
        sync::service::sync_device(repo, device_id, &mut a, 3, 200, &cancel)
    }

    #[tauri::command]
    pub fn sync_run_all(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "sync.*").or_else(|_| require(&state, &token, "device.manage"))?;
        let ids: Vec<String> =
            state
                .repo
                .query_rows("SELECT id FROM devices WHERE is_active=1", &[], |r| {
                    r.get(0)
                })?;
        let mut out = vec![];
        for id in ids {
            match sync_one(&state.repo, &id) {
                Ok(st) => {
                    log::info!("sync ok {id}: fetch={} baru={}", st.fetched, st.inserted);
                    out.push(serde_json::json!({"device": id, "ok": true,
                    "fetched": st.fetched, "inserted": st.inserted,
                    "duplicates": st.duplicates, "unmapped": st.unmapped,
                    "checkpoint": st.checkpoint}))
                }
                Err(e) => {
                    log::warn!("sync gagal {id}: {e}");
                    out.push(serde_json::json!({"device": id, "ok": false, "error": e.to_string()}))
                }
            }
        }
        Ok(serde_json::Value::Array(out))
    }

    #[tauri::command]
    pub fn scheduler_get(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        Ok(serde_json::json!({
            "enabled": setting_get(&state.repo, "scheduler_enabled") == "1",
            "interval_secs": setting_get(&state.repo, "sync_interval_secs"),
            "running": state.sched_running.load(std::sync::atomic::Ordering::Relaxed),
        }))
    }

    #[tauri::command]
    pub fn scheduler_set(
        app: tauri::AppHandle,
        state: tauri::State<AppState>,
        token: String,
        enabled: bool,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "user.manage")?;
        if s.role != Role::SuperAdmin {
            return Err(AppError::Forbidden(
                "hanya super_admin yang boleh mengatur scheduler".into(),
            ));
        }
        let ts = now();
        state.repo.exec(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('scheduler_enabled',?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            &[&(if enabled { "1" } else { "0" }).to_string(), &ts],
        )?;
        state
            .sched_enabled
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
        if enabled
            && !state
                .sched_running
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            state
                .sched_running
                .store(true, std::sync::atomic::Ordering::Relaxed);
            let running = state.sched_running.clone();
            let flag = state.sched_enabled.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    let interval: u64 = app
                        .state::<AppState>()
                        .repo
                        .query_one::<String, _>(
                            "SELECT value FROM app_settings WHERE key='sync_interval_secs'",
                            &[],
                            |r| r.get(0),
                        )
                        .unwrap_or(None)
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(300)
                        .max(60);
                    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                    if !flag.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    let ids: Vec<String> = app
                        .state::<AppState>()
                        .repo
                        .query_rows("SELECT id FROM devices WHERE is_active=1", &[], |r| {
                            r.get(0)
                        })
                        .unwrap_or_default();
                    for id in ids {
                        if !flag.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        let _ = sync_one(&app.state::<AppState>().repo, &id);
                    }
                    // Backup otomatis harian bila diaktifkan (sekali per tanggal WIB).
                    let st = app.state::<AppState>();
                    let auto = st
                        .repo
                        .query_one::<String, _>(
                            "SELECT value FROM app_settings WHERE key='backup_auto'",
                            &[],
                            |r| r.get(0),
                        )
                        .unwrap_or(None)
                        .unwrap_or_default();
                    if auto.trim() == "1" {
                        let today = timex::today_wib().format("%Y-%m-%d").to_string();
                        let last: String = st
                            .repo
                            .query_one::<String, _>(
                                "SELECT value FROM app_settings WHERE key='backup_last_auto'",
                                &[],
                                |r| r.get(0),
                            )
                            .unwrap_or(None)
                            .unwrap_or_default();
                        if last != today {
                            let keep: usize = st
                                .repo
                                .query_one::<String, _>(
                                    "SELECT value FROM app_settings WHERE key='backup_keep_files'",
                                    &[],
                                    |r| r.get(0),
                                )
                                .unwrap_or(None)
                                .and_then(|v| v.trim().parse().ok())
                                .unwrap_or(10);
                            let dir = st
                                .db_path
                                .parent()
                                .map(|p| p.join("backups"))
                                .unwrap_or("backups".into());
                            if let Ok(dest) = backup::backup_db(&st.db_path, &dir, keep.max(1)) {
                                log::info!("backup otomatis: {}", dest.display());
                                let ts = timex::now_wib_string();
                                let _ = st.repo.exec(
                                    "INSERT INTO app_settings (key, value, updated_at) VALUES ('backup_last_auto',?1,?2)
                                     ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
                                    &[&today, &ts],
                                );
                            }
                        }
                    }
                }
                running.store(false, std::sync::atomic::Ordering::Relaxed);
            });
        }
        Ok("ok".into())
    }

    // ---------- import/export CSV karyawan ----------
    #[tauri::command]
    pub fn employee_import_preview(
        _state: tauri::State<AppState>,
        _token: String,
        csv_text: String,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&_state, &_token, "attendance.view")?;
        let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
        let headers: Vec<String> = rdr
            .headers()
            .map(|h| h.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default();
        let mut rows: Vec<serde_json::Value> = vec![];
        let mut errors: Vec<String> = vec![];
        for (i, rec) in rdr.records().enumerate().take(200) {
            match rec {
                Ok(r) => {
                    let m: std::collections::HashMap<String, String> = headers
                        .iter()
                        .cloned()
                        .zip(r.iter().map(|s| s.to_string()))
                        .collect();
                    let mut errs = vec![];
                    if m.get("nik").map(|s| s.trim().is_empty()).unwrap_or(true) {
                        errs.push("nik kosong");
                    }
                    if m.get("name").map(|s| s.trim().is_empty()).unwrap_or(true) {
                        errs.push("nama kosong");
                    }
                    rows.push(serde_json::json!({"line": i + 2, "data": m, "errors": errs}));
                    if !errs.is_empty() {
                        errors.push(format!("baris {}: {}", i + 2, errs.join(", ")));
                    }
                }
                Err(e) => errors.push(format!("baris {}: {e}", i + 2)),
            }
        }
        Ok(serde_json::json!({"headers": headers, "rows": rows, "errors": errors}))
    }

    #[tauri::command]
    pub fn employee_import_commit(
        state: tauri::State<AppState>,
        token: String,
        rows: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "employee.manage")?;
        let mut ok = 0i64;
        let mut fail = 0i64;
        let mut messages: Vec<String> = vec![];
        for (i, r) in rows.iter().enumerate() {
            let nik = r.get("nik").and_then(|v| v.as_str()).unwrap_or("").trim();
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            if nik.is_empty() || name.is_empty() {
                fail += 1;
                messages.push(format!("baris {i}: nik/nama wajib"));
                continue;
            }
            let id = uid();
            let ts = now();
            let hire = r
                .get("hire_date")
                .and_then(|v| v.as_str())
                .unwrap_or("2026-01-01");
            match state.repo.exec(
            "INSERT INTO employees (id, nik, name, email, phone, hire_date, status, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,'aktif',?7,?7)",
            &[&id, &nik.to_string(), &name.to_string(),
              &r.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
              &r.get("phone").and_then(|v| v.as_str()).unwrap_or("").to_string(),
              &hire.to_string(), &ts],
        ) {
            Ok(_) => ok += 1,
            Err(e) => { fail += 1; messages.push(format!("baris {i} ({nik}): {e}")); }
        }
        }
        state.repo.insert_audit(
            &s.username,
            "import_employees",
            "employees",
            "-",
            &format!("ok={ok} fail={fail}"),
        )?;
        Ok(serde_json::json!({"ok": ok, "fail": fail, "messages": messages}))
    }

    // ---------- upload foto & lampiran (via file picker di UI) ----------
    fn store_upload(
        state: &AppState,
        subdir: &str,
        stem: &str,
        src_path: &str,
        allowed: &[&str],
        max_bytes: u64,
    ) -> AppResult<String> {
        let src = std::path::Path::new(src_path);
        if !src.is_file() {
            return Err(AppError::Validation("file sumber tidak ditemukan".into()));
        }
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !allowed.contains(&ext.as_str()) {
            return Err(AppError::Validation(format!(
                "tipe file tidak diizinkan (boleh: {})",
                allowed.join(", ")
            )));
        }
        let size = std::fs::metadata(src)?.len();
        if size > max_bytes {
            return Err(AppError::Validation(format!(
                "file terlalu besar (maks {} MB)",
                max_bytes / 1024 / 1024
            )));
        }
        let dir = state
            .db_path
            .parent()
            .map(|p| p.join("uploads").join(subdir))
            .unwrap_or_else(|| std::path::PathBuf::from("uploads").join(subdir));
        std::fs::create_dir_all(&dir)?;
        let stem_safe: String = stem
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let dest = dir.join(format!("{stem_safe}.{ext}"));
        std::fs::copy(src, &dest)?;
        Ok(dest.display().to_string())
    }

    #[tauri::command]
    pub fn save_employee_photo(
        state: tauri::State<AppState>,
        token: String,
        employee_id: String,
        src_path: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "employee.manage")?;
        let exists: i64 = state
            .repo
            .query_one(
                "SELECT COUNT(*) FROM employees WHERE id=?1",
                &[&employee_id],
                |r| r.get(0),
            )?
            .unwrap_or(0);
        if exists == 0 {
            return Err(AppError::NotFound("karyawan".into()));
        }
        let dest = store_upload(
            &state,
            "photos",
            &employee_id,
            &src_path,
            &["png", "jpg", "jpeg"],
            2 * 1024 * 1024,
        )?;
        state.repo.exec(
            "UPDATE employees SET photo_path=?1, updated_at=datetime('now','localtime') WHERE id=?2",
            &[&dest, &employee_id],
        )?;
        state
            .repo
            .insert_audit(&s.username, "save_photo", "employees", &employee_id, &dest)?;
        Ok(serde_json::json!({"path": dest}))
    }

    #[tauri::command]
    pub fn save_leave_attachment(
        state: tauri::State<AppState>,
        token: String,
        request_id: String,
        src_path: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "leave.manage")?;
        let exists: i64 = state
            .repo
            .query_one(
                "SELECT COUNT(*) FROM leave_requests WHERE id=?1",
                &[&request_id],
                |r| r.get(0),
            )?
            .unwrap_or(0);
        if exists == 0 {
            return Err(AppError::NotFound("pengajuan".into()));
        }
        let dest = store_upload(
            &state,
            "attachments",
            &request_id,
            &src_path,
            &["pdf", "jpg", "jpeg", "png"],
            5 * 1024 * 1024,
        )?;
        state.repo.exec(
            "UPDATE leave_requests SET attachment_path=?1 WHERE id=?2",
            &[&dest, &request_id],
        )?;
        state.repo.insert_audit(
            &s.username,
            "save_attachment",
            "leave_requests",
            &request_id,
            &dest,
        )?;
        Ok(serde_json::json!({"path": dest}))
    }

    // ---------- reports ----------
    pub struct ReportFilter {
        pub start: String,
        pub end: String,
        pub branch_id: String,
        pub department_id: String,
        pub shift_id: String,
        pub status: String,
        pub employee_id: String,
    }

    fn report_data(
        repo: &Repo,
        kind: &str,
        f: &ReportFilter,
    ) -> AppResult<(Vec<String>, Vec<Vec<String>>)> {
        let mut cond = vec!["a.work_date>=?1".to_string(), "a.work_date<=?2".to_string()];
        let mut args: Vec<String> = vec![f.start.clone(), f.end.clone()];
        let mut push = |c: &str, v: &str| {
            if !v.is_empty() {
                cond.push(c.to_string());
                args.push(v.to_string());
            }
        };
        push("e.branch_id=?", f.branch_id.as_str());
        push("e.department_id=?", f.department_id.as_str());
        push("a.shift_id=?", f.shift_id.as_str());
        push("a.status=?", f.status.as_str());
        push("a.employee_id=?", f.employee_id.as_str());
        let where_s = cond.join(" AND ");
        let base = match kind {
        "harian" | "karyawan" | "departemen" | "cabang" | "shift" | "terlambat" | "pulang_cepat"
        | "lembur" | "tidak_hadir" | "izin_cuti" | "rekon" => format!(
            "SELECT a.work_date, e.nik, e.name, s.name, a.clock_in, a.clock_out, a.work_minutes, a.late_minutes, a.early_leave_minutes, a.overtime_minutes, a.status, a.anomaly
             FROM attendance_results a JOIN employees e ON e.id=a.employee_id LEFT JOIN shifts s ON s.id=a.shift_id WHERE {where_s} ORDER BY a.work_date, e.name LIMIT 2000"),
        "rekap_bulanan" => format!(
            "SELECT substr(a.work_date,1,7), e.nik, e.name,
                    COUNT(*),
                    SUM(CASE WHEN a.status IN ('Present','Late') THEN 1 ELSE 0 END),
                    SUM(CASE WHEN a.status='Late' THEN 1 ELSE 0 END),
                    SUM(CASE WHEN a.status='Absent' THEN 1 ELSE 0 END),
                    COALESCE(SUM(a.work_minutes),0), COALESCE(SUM(a.overtime_minutes),0)
             FROM attendance_results a JOIN employees e ON e.id=a.employee_id LEFT JOIN shifts s ON s.id=a.shift_id WHERE {where_s} GROUP BY substr(a.work_date,1,7), e.id ORDER BY 1, e.name LIMIT 2000"),
            "log_perangkat" => "SELECT event_time, device_id, device_user_id, direction, status FROM raw_events WHERE event_time>=?1 AND event_time<=?2 ORDER BY event_time LIMIT 2000".to_string(),
        "error_sync" => "SELECT created_at, device_id, event, detail FROM device_connection_log ORDER BY created_at DESC LIMIT 500".to_string(),
        "riwayat_koreksi" => "SELECT created_at, attendance_id, changed_by, field_name, old_value, new_value, reason FROM corrections_audit ORDER BY created_at DESC LIMIT 500".to_string(),
        _ => return Err(AppError::Validation("jenis laporan tidak dikenal".into())),
    };
        // jalankan dengan params dinamis
        let rows: Vec<Vec<String>> = query_dynamic(repo, &base, &args)?;
        let headers = match kind {
            "log_perangkat" => vec!["Waktu (WIB)", "Perangkat", "User Device", "Arah", "Status"],
            "error_sync" => vec!["Waktu (WIB)", "Perangkat", "Event", "Detail"],
            "rekap_bulanan" => vec![
                "Bulan",
                "NIK",
                "Nama",
                "Hari tercatat",
                "Hadir",
                "Telat",
                "Absen",
                "Kerja (m)",
                "Lembur (m)",
            ],
            "riwayat_koreksi" => vec![
                "Waktu (WIB)",
                "ID Absensi",
                "Oleh",
                "Kolom",
                "Lama",
                "Baru",
                "Alasan",
            ],
            _ => vec![
                "Tanggal",
                "NIK",
                "Nama",
                "Shift",
                "Masuk",
                "Pulang",
                "Kerja (m)",
                "Terlambat (m)",
                "Pulang cepat (m)",
                "Lembur (m)",
                "Status",
                "Anomali",
            ],
        }
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
        Ok((headers, rows))
    }

    #[tauri::command]
    // Tanda tangan IPC Tauri dipetakan 1:1 ke argumen frontend; jumlah parameter mengikuti kontrak API.
    #[allow(clippy::too_many_arguments)]
    pub fn report_query(
        state: tauri::State<AppState>,
        token: String,
        kind: String,
        start: String,
        end: String,
        branch_id: String,
        department_id: String,
        shift_id: String,
        status: String,
        employee_id: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        let _ = check_table_perm(&s, "attendance_results", false);
        let filter = ReportFilter {
            start,
            end,
            branch_id,
            department_id,
            shift_id,
            status,
            employee_id,
        };
        let (headers, rows) = report_data(&state.repo, &kind, &filter)?;
        Ok(serde_json::json!({"headers": headers, "rows": rows}))
    }

    fn query_dynamic(repo: &Repo, sql: &str, args: &[String]) -> AppResult<Vec<Vec<String>>> {
        repo.query_strings(sql, args)
    }

    #[tauri::command]
    // Tanda tangan IPC Tauri dipetakan 1:1 ke argumen frontend; jumlah parameter mengikuti kontrak API.
    #[allow(clippy::too_many_arguments)]
    pub fn report_export(
        state: tauri::State<AppState>,
        token: String,
        kind: String,
        format: String,
        start: String,
        end: String,
        branch_id: String,
        department_id: String,
        shift_id: String,
        status: String,
        employee_id: String,
        title: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "attendance.view")?;
        let _ = check_table_perm(&s, "attendance_results", false);
        let filter = ReportFilter {
            start,
            end,
            branch_id,
            department_id,
            shift_id,
            status,
            employee_id,
        };
        let (headers, rows) = report_data(&state.repo, &kind, &filter)?;
        match format.as_str() {
            "csv" => {
                let s = reports::export::to_csv(&headers, &rows)?;
                Ok(serde_json::json!({"format": "csv", "text": s}))
            }
            "xlsx" => {
                let b = reports::export::to_xlsx(&headers, &rows, "Laporan")?;
                Ok(serde_json::json!({"format": "xlsx", "base64": base64_encode(&b)}))
            }
            "pdf" => {
                let meta = format!("Dibuat: {} WIB · jenis: {kind}", now());
                let b = reports::export::to_pdf(&title, &meta, &headers, &rows);
                Ok(serde_json::json!({"format": "pdf", "base64": base64_encode(&b)}))
            }
            _ => Err(AppError::Validation("format harus csv/xlsx/pdf".into())),
        }
    }

    fn base64_encode(b: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        for c in b.chunks(3) {
            let n = ((c[0] as u32) << 16)
                | ((c.get(1).copied().unwrap_or(0) as u32) << 8)
                | (c.get(2).copied().unwrap_or(0) as u32);
            o.push(T[((n >> 18) & 63) as usize] as char);
            o.push(T[((n >> 12) & 63) as usize] as char);
            o.push(if c.len() > 1 {
                T[((n >> 6) & 63) as usize] as char
            } else {
                '='
            });
            o.push(if c.len() > 2 {
                T[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        o
    }

    // ---------- backup ----------
    #[tauri::command]
    pub fn backup_create(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "backup.*")
            .or_else(|_| require(&state, &token, "user.manage"))?;
        let dir = state
            .db_path
            .parent()
            .map(|p| p.join("backups"))
            .unwrap_or("backups".into());
        let keep: usize = setting_get(&state.repo, "backup_keep_files")
            .trim()
            .parse()
            .unwrap_or(10);
        let dest = backup::backup_db(&state.db_path, &dir, keep.clamp(1, 100))?;
        log::info!("backup manual: {}", dest.display());
        state.repo.insert_audit(
            &s.username,
            "backup",
            "db",
            "-",
            &dest.display().to_string(),
        )?;
        Ok(serde_json::json!({"file": dest.display().to_string()}))
    }

    #[tauri::command]
    pub fn backup_restore(
        state: tauri::State<AppState>,
        token: String,
        file: String,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "backup.*")
            .or_else(|_| require(&state, &token, "user.manage"))?;
        backup::restore_db(&state.db_path, std::path::Path::new(&file))?;
        log::info!("restore dari {file}");
        state
            .repo
            .insert_audit(&s.username, "restore", "db", "-", &file)?;
        Ok("ok — restart aplikasi untuk memuat database".into())
    }

    #[tauri::command]
    pub fn health_check(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        let _s = require(&state, &token, "attendance.view")?;
        let integrity = backup::integrity_check(&state.db_path).unwrap_or_else(|e| e.to_string());
        let db_version: i64 = state
            .repo
            .query_one("SELECT MAX(version) FROM schema_migrations", &[], |r| {
                r.get(0)
            })?
            .unwrap_or(0);
        Ok(serde_json::json!({
            "app": env!("CARGO_PKG_VERSION"),
            "db_version": db_version,
            "integrity": integrity,
            "wib_now": now(),
        }))
    }

    // ---------- antrean event: kembalikan processed -> pending untuk hitung ulang ----------
    #[tauri::command]
    pub fn events_requeue(
        state: tauri::State<AppState>,
        token: String,
        start_date: String,
        end_date: String,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "sync.*")
            .or_else(|_| require(&state, &token, "device.manage"))?;
        parse_date(&start_date)?;
        parse_date(&end_date)?;
        let start_s = format!("{start_date} 00:00:00");
        let end_s = format!("{end_date} 23:59:59");
        let n = state.repo.exec(
            "UPDATE raw_events SET status='pending' WHERE status='processed' AND event_time>=?1 AND event_time<=?2",
            &[&start_s, &end_s],
        )?;
        state.repo.insert_audit(
            &s.username,
            "events_requeue",
            "raw_events",
            &format!("{start_date}..{end_date}"),
            &format!("{n} dikembalikan ke antrean"),
        )?;
        Ok(serde_json::json!({"requeued": n}))
    }

    // ---------- user di perangkat (bila SDK mendukung) ----------
    #[tauri::command]
    pub fn device_fetch_users(
        state: tauri::State<AppState>,
        token: String,
        device_id: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "device.view")?;
        let row: Option<(String, String, String, String)> = state.repo.query_one(
            "SELECT id, host, adapter_name, config_json FROM devices WHERE id=?1",
            &[&device_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let Some((_did, host, adapter_name, config_json)) = row else {
            return Err(AppError::NotFound("perangkat".into()));
        };
        let mut a = sync::service::build_adapter(&adapter_name, &config_json);
        a.initialize()
            .map_err(|e| AppError::Device(e.to_string()))?;
        a.connect(
            if host.is_empty() {
                "local"
            } else {
                host.as_str()
            },
            std::time::Duration::from_secs(10),
        )
        .map_err(|e| AppError::Device(e.to_string()))?;
        let users = a
            .fetch_device_users()
            .map_err(|e| AppError::Device(e.to_string()))?;
        let _ = a.disconnect();
        Ok(serde_json::to_value(users).unwrap())
    }

    #[tauri::command]
    pub fn device_provision_user(
        state: tauri::State<AppState>,
        token: String,
        device_id: String,
        device_user_id: String,
        name: String,
    ) -> Result<String, AppError> {
        let s = require(&state, &token, "device.manage")?;
        if device_user_id.trim().is_empty() || name.trim().is_empty() {
            return Err(AppError::Validation("user ID dan nama wajib diisi".into()));
        }
        let (adapter_name, host, config_json): (String, String, String) = state
            .repo
            .query_one(
                "SELECT adapter_name, host, config_json FROM devices WHERE id=?1",
                &[&device_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?
            .ok_or_else(|| AppError::NotFound("perangkat".into()))?;
        let mut a = sync::service::build_adapter(&adapter_name, &config_json);
        a.initialize()
            .map_err(|e| AppError::Device(e.to_string()))?;
        a.connect(
            if host.is_empty() {
                "local"
            } else {
                host.as_str()
            },
            std::time::Duration::from_secs(10),
        )
        .map_err(|e| AppError::Device(e.to_string()))?;
        a.provision_user(&devices::adapter::DeviceUser {
            device_user_id: device_user_id.clone(),
            name: name.clone(),
        })
        .map_err(|e| AppError::Device(e.to_string()))?;
        let _ = a.disconnect();
        state.repo.insert_audit(
            &s.username,
            "provision_user",
            "devices",
            &device_id,
            &format!("{device_user_id} / {name}"),
        )?;
        Ok("ok".into())
    }

    // ---------- saldo cuti ----------
    #[tauri::command]
    pub fn leave_balances(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let rows: Vec<serde_json::Value> = state.repo.query_rows(
            "SELECT e.id, e.nik, e.name, t.code, t.name, t.quota_days,
                    COALESCE((SELECT SUM(CAST(julianday(end_date)-julianday(start_date) AS INTEGER)+1)
                              FROM leave_requests r WHERE r.employee_id=e.id AND r.leave_type_id=t.id AND r.status='approved'),0) AS used
             FROM employees e CROSS JOIN leave_types t
             WHERE e.status='aktif' AND e.deleted_at IS NULL
             ORDER BY e.name, t.code",
            &[],
            |r| {
                let quota: i64 = r.get(5)?;
                let used: i64 = r.get(6)?;
                Ok(serde_json::json!({
                    "employee_id": r.get::<_, String>(0)?,
                    "nik": r.get::<_, String>(1)?,
                    "name": r.get::<_, String>(2)?,
                    "leave_code": r.get::<_, String>(3)?,
                    "leave_name": r.get::<_, String>(4)?,
                    "quota": quota,
                    "used": used,
                    "remaining": quota - used,
                }))
            },
        )?;
        Ok(serde_json::Value::Array(rows))
    }

    // ---------- pencarian global ----------
    #[tauri::command]
    pub fn search_global(
        state: tauri::State<AppState>,
        token: String,
        query: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let q = format!("%{}%", query.trim());
        if q.len() < 3 {
            return Err(AppError::Validation("kata kunci minimal 1 karakter".into()));
        }
        let emps: Vec<serde_json::Value> = state.repo.query_rows(
            "SELECT id, nik, name, status FROM employees WHERE deleted_at IS NULL AND (nik LIKE ?1 OR name LIKE ?1) LIMIT 10",
            &[&q],
            |r| {
                Ok(serde_json::json!({"kind": "karyawan", "id": r.get::<_, String>(0)?,
                    "label": format!("{} — {}", r.get::<_, String>(1)?, r.get::<_, String>(2)?),
                    "sub": r.get::<_, String>(3)?}))
            },
        )?;
        let devs: Vec<serde_json::Value> = state.repo.query_rows(
            "SELECT id, name, status FROM devices WHERE is_active=1 AND name LIKE ?1 LIMIT 10",
            &[&q],
            |r| {
                Ok(
                    serde_json::json!({"kind": "perangkat", "id": r.get::<_, String>(0)?,
                    "label": r.get::<_, String>(1)?, "sub": r.get::<_, String>(2)?}),
                )
            },
        )?;
        Ok(serde_json::json!({"employees": emps, "devices": devs}))
    }

    // ---------- ekspor/impor jadwal ----------
    #[tauri::command]
    pub fn schedule_export(
        state: tauri::State<AppState>,
        token: String,
        start_date: String,
        end_date: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        parse_date(&start_date)?;
        parse_date(&end_date)?;
        let rows: Vec<Vec<String>> = state.repo.query_rows(
            "SELECT e.nik, e.name, s.code, a.effective_date, COALESCE(a.end_date,'') FROM shift_assignments a
             JOIN employees e ON e.id=a.employee_id JOIN shifts s ON s.id=a.shift_id
             WHERE a.effective_date<=?1 AND (a.end_date IS NULL OR a.end_date>=?2)
             ORDER BY e.name, a.effective_date",
            &[&end_date, &start_date],
            |r| {
                Ok(vec![
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ])
            },
        )?;
        let csv = reports::export::to_csv(
            &["nik", "nama", "shift", "efektif", "akhir"]
                .into_iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>(),
            &rows,
        )?;
        Ok(serde_json::json!({"csv": csv, "count": rows.len()}))
    }

    #[tauri::command]
    pub fn schedule_import_preview(
        state: tauri::State<AppState>,
        token: String,
        csv_text: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "shift.manage")?;
        let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
        let mut rows = vec![];
        let mut errors = vec![];
        // Toleran: boleh dengan atau tanpa baris header. Bila baris pertama
        // justru berisi data (kolom tanggal valid), perlakukan sebagai data.
        let mut records: Vec<Vec<String>> = vec![];
        let headers: Vec<String> = rdr
            .headers()
            .map(|h| h.iter().map(|x| x.to_string()).collect())
            .unwrap_or_default();
        if headers.len() >= 3
            && parse_date(headers[2].trim()).is_ok()
            && !headers[0].trim().eq_ignore_ascii_case("nik")
        {
            records.push(headers);
        }
        for rec in rdr.records().take(500) {
            match rec {
                Ok(r) => records.push(r.iter().map(|x| x.to_string()).collect()),
                Err(e) => errors.push(format!("CSV rusak: {e}")),
            }
        }
        for (i, rec) in records.into_iter().enumerate() {
            let line = i + 2;
            let g = |n: usize| rec.get(n).map(|x| x.trim().to_string()).unwrap_or_default();
            let (nik, code, eff, end) = (g(0), g(1), g(2), g(3));
            let mut errs = vec![];
            let emp: Option<String> = state
                .repo
                .query_one("SELECT id FROM employees WHERE nik=?1", &[&nik], |r| {
                    r.get(0)
                })
                .unwrap_or(None);
            if emp.is_none() {
                errs.push(format!("nik {nik} tidak dikenal"));
            }
            let sh: Option<String> = state
                .repo
                .query_one("SELECT id FROM shifts WHERE code=?1", &[&code], |r| {
                    r.get(0)
                })
                .unwrap_or(None);
            if sh.is_none() {
                errs.push(format!("shift {code} tidak dikenal"));
            }
            if parse_date(&eff).is_err() {
                errs.push("kolom efektif harus YYYY-MM-DD".into());
            }
            if !end.is_empty() && parse_date(&end).is_err() {
                errs.push("kolom akhir harus YYYY-MM-DD/kosong".into());
            }
            if !errs.is_empty() {
                errors.push(format!("baris {line}: {}", errs.join(", ")));
            }
            rows.push(serde_json::json!({
                "line": line, "nik": nik, "shift": code, "eff": eff, "end": end,
                "employee_id": emp, "shift_id": sh, "errors": errs,
            }));
        }
        Ok(serde_json::json!({"rows": rows, "errors": errors}))
    }

    #[tauri::command]
    pub fn schedule_import_commit(
        state: tauri::State<AppState>,
        token: String,
        rows: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, AppError> {
        let s = require(&state, &token, "shift.manage")?;
        let mut ok = 0i64;
        let mut fail = 0i64;
        let mut messages = vec![];
        for (i, r) in rows.iter().enumerate() {
            let eid = r.get("employee_id").and_then(|v| v.as_str()).unwrap_or("");
            let sid = r.get("shift_id").and_then(|v| v.as_str()).unwrap_or("");
            let eff = r.get("eff").and_then(|v| v.as_str()).unwrap_or("");
            let end = r.get("end").and_then(|v| v.as_str()).unwrap_or("");
            if eid.is_empty() || sid.is_empty() || parse_date(eff).is_err() {
                fail += 1;
                messages.push(format!("baris {i}: data tidak valid"));
                continue;
            }
            let dup: i64 = state
                .repo
                .query_one(
                    "SELECT COUNT(*) FROM shift_assignments WHERE employee_id=?1 AND shift_id=?2 AND effective_date=?3",
                    &[&eid.to_string(), &sid.to_string(), &eff.to_string()],
                    |x| x.get(0),
                )?
                .unwrap_or(0);
            if dup > 0 {
                fail += 1;
                messages.push(format!("baris {i}: duplikat, dilewati"));
                continue;
            }
            let id = uid();
            let res = if end.is_empty() {
                state.repo.exec(
                    "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, end_date, created_by, created_at)
                     VALUES (?1,?2,?3,?4,NULL,?5,?6)",
                    &[&id, &eid.to_string(), &sid.to_string(), &eff.to_string(), &s.username, &now()],
                )
            } else {
                state.repo.exec(
                    "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, end_date, created_by, created_at)
                     VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    &[&id, &eid.to_string(), &sid.to_string(), &eff.to_string(), &end.to_string(), &s.username, &now()],
                )
            };
            match res {
                Ok(_) => ok += 1,
                Err(e) => {
                    fail += 1;
                    messages.push(format!("baris {i}: {e}"));
                }
            }
        }
        state.repo.insert_audit(
            &s.username,
            "schedule_import",
            "shift_assignments",
            "-",
            &format!("ok={ok} fail={fail}"),
        )?;
        Ok(serde_json::json!({"ok": ok, "fail": fail, "messages": messages}))
    }

    // ---------- bundel diagnostik (tanpa rahasia) ----------
    #[tauri::command]
    pub fn diagnostics_bundle(
        state: tauri::State<AppState>,
        token: String,
    ) -> Result<serde_json::Value, AppError> {
        require(&state, &token, "attendance.view")?;
        let integrity = backup::integrity_check(&state.db_path).unwrap_or_else(|e| e.to_string());
        let db_version: i64 = state
            .repo
            .query_one("SELECT MAX(version) FROM schema_migrations", &[], |r| {
                r.get(0)
            })?
            .unwrap_or(0);
        let counts: Vec<serde_json::Value> = state.repo.query_rows(
            "SELECT 'employees', COUNT(*) FROM employees UNION ALL
             SELECT 'devices', COUNT(*) FROM devices UNION ALL
             SELECT 'raw_events', COUNT(*) FROM raw_events UNION ALL
             SELECT 'attendance_results', COUNT(*) FROM attendance_results",
            &[],
            |r| Ok(serde_json::json!({"table": r.get::<_, String>(0)?, "count": r.get::<_, i64>(1)?})),
        )?;
        let logs = super::applog::tail_logs(200);
        let text = format!(
            "AttendX diagnostik {wib}\napp={app} db_version={dbv} integrity={integ}\ncounts={counts}\n--- log aplikasi (maks 200 baris) ---\n{logs}",
            wib = now(),
            app = env!("CARGO_PKG_VERSION"),
            dbv = db_version,
            integ = integrity,
            counts = serde_json::to_string(&counts).unwrap_or_default(),
            logs = logs,
        );
        Ok(serde_json::json!({"text": text}))
    }

    #[cfg(test)]
    mod tests {
        use super::super::auth::SessionStore;
        use super::*;

        fn test_state() -> AppState {
            let dir = tempfile::tempdir().unwrap();
            let db_path = dir.path().join("t.db");
            // Pertahankan dir agar tidak terhapus selama test.
            std::mem::forget(dir);
            AppState {
                repo: Repo::open(&db_path).unwrap(),
                sessions: SessionStore::new(3600),
                db_path,
                sched_enabled: std::sync::Arc::new(AtomicBool::new(false)),
                sched_running: std::sync::Arc::new(AtomicBool::new(false)),
            }
        }

        #[test]
        fn parse_date_validates_format() {
            assert!(parse_date("2026-05-04").is_ok());
            assert!(parse_date("04-05-2026").is_err());
            assert!(parse_date("besok").is_err());
        }

        #[test]
        fn effective_shift_picks_latest() {
            let st = test_state();
            st.repo
                .exec(
                    "INSERT INTO employees (id, nik, name, hire_date, status, created_at, updated_at)
                     VALUES ('e1','N1','Uji','2024-01-01','aktif','2026-01-01 00:00:00','2026-01-01 00:00:00')",
                    &[],
                )
                .unwrap();
            for (id, shift, eff) in [
                ("a1", "sh-pagi", "2026-01-01"),
                ("a2", "sh-siang", "2026-05-01"),
            ] {
                st.repo
                    .exec(
                        "INSERT INTO shift_assignments (id, employee_id, shift_id, effective_date, created_by, created_at)
                         VALUES (?1,'e1',?2,?3,'t','2026-01-01 00:00:00')",
                        &[&id.to_string(), &shift.to_string(), &eff.to_string()],
                    )
                    .unwrap();
            }
            assert_eq!(
                effective_shift(&st.repo, "e1", "2026-03-01").map(|x| x.0),
                Some("sh-pagi".to_string())
            );
            assert_eq!(
                effective_shift(&st.repo, "e1", "2026-06-01").map(|x| x.0),
                Some("sh-siang".to_string())
            );
        }

        #[test]
        fn store_upload_rejects_bad_file() {
            let st = test_state();
            assert!(store_upload(&st, "photos", "e1", "tidak-ada.png", &["png"], 100).is_err());
            let dir = tempfile::tempdir().unwrap();
            let bad = dir.path().join("x.exe");
            std::fs::write(&bad, b"mz").unwrap();
            assert!(store_upload(
                &st,
                "photos",
                "e1",
                bad.to_str().unwrap(),
                &["png", "jpg"],
                100
            )
            .is_err());
            let big = dir.path().join("b.png");
            std::fs::write(&big, vec![0u8; 200]).unwrap();
            assert!(
                store_upload(&st, "photos", "e1", big.to_str().unwrap(), &["png"], 10).is_err()
            );
            let ok = dir.path().join("o.png");
            std::fs::write(&ok, vec![0u8; 50]).unwrap();
            let dest =
                store_upload(&st, "photos", "e1", ok.to_str().unwrap(), &["png"], 1000).unwrap();
            assert!(std::path::Path::new(&dest).is_file());
        }

        #[test]
        fn setting_keys_seeded_by_migration() {
            let st = test_state();
            assert_eq!(setting_get(&st.repo, "sync_interval_secs"), "300");
            assert_eq!(setting_get(&st.repo, "scheduler_enabled"), "0");
            assert_eq!(setting_get(&st.repo, "backup_auto"), "0");
        }

        #[test]
        fn future_work_date_rejected() {
            let future = (crate::timex::today_wib() + chrono::Duration::days(2))
                .format("%Y-%m-%d")
                .to_string();
            assert!(check_work_date(&future).is_err());
            let past = (crate::timex::today_wib() - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string();
            assert!(check_work_date(&past).is_ok());
        }

        #[test]
        fn role_permission_matrix() {
            use auth::Role::*;
            // Cerminkan logika wildcard SessionStore::check_permission.
            let has = |r: auth::Role, p: &str| {
                let perms = auth::permissions(&r);
                perms.contains(p)
                    || perms
                        .iter()
                        .any(|x| x.ends_with(".*") && p.starts_with(&x[..x.len() - 2]))
            };
            assert!(has(SuperAdmin, "device.sync"));
            assert!(has(HrAdmin, "correction.approve"));
            assert!(!has(Viewer, "employee.manage"));
            assert!(!has(Viewer, "user.manage"));
            assert!(has(OperatorDevice, "sync.run"));
            assert!(!has(OperatorDevice, "leave.manage"));
            assert!(has(Supervisor, "leave.approve"));
            assert!(!has(Supervisor, "device.manage"));
            assert!(has(Viewer, "report.view"));
            assert!(!has(HrAdmin, "user.manage"));
        }
    }

    // ---------- perluasan Repo: eksekusi SQL dinamis aman (kolom tervalidasi) ----------
} // end mod commands

// ---------- perluasan Repo: eksekusi SQL dinamis aman (kolom tervalidasi) ----------
impl Repo {
    pub fn query_strings(&self, sql: &str, args: &[String]) -> AppResult<Vec<Vec<String>>> {
        let guard = self
            .conn
            .lock()
            .map_err(|e| AppError::Db(format!("lock: {e}")))?;
        let mut stmt = guard.prepare(sql)?;
        let n = stmt.column_count();
        let mut out: Vec<Vec<String>> = vec![];
        let mut rows = stmt.query(rusqlite::params_from_iter(args.iter()))?;
        while let Some(row) = rows.next()? {
            let mut v = vec![];
            for i in 0..n {
                let val: rusqlite::types::Value = row.get(i)?;
                v.push(match val {
                    rusqlite::types::Value::Null => String::new(),
                    rusqlite::types::Value::Integer(x) => x.to_string(),
                    rusqlite::types::Value::Real(x) => x.to_string(),
                    rusqlite::types::Value::Text(x) => x,
                    rusqlite::types::Value::Blob(b) => format!("<{}B>", b.len()),
                });
            }
            out.push(v);
        }
        Ok(out)
    }
    pub fn repo_exec_raw(&self, sql: &str, vals: &[String]) -> AppResult<usize> {
        use rusqlite::ToSql;
        // Hanya izinkan DML ke tabel yang dikenal (cegah SQL injection via nama tabel:
        // pemanggil sudah memvalidasi columns_of; di sini validasi ulang prefiks).
        let ok_prefix = [
            "INSERT INTO employees",
            "INSERT INTO branches",
            "INSERT INTO departments",
            "INSERT INTO positions",
            "INSERT INTO shifts",
            "INSERT INTO shift_assignments",
            "INSERT INTO devices",
            "INSERT INTO leave",
            "INSERT INTO overtime",
            "INSERT INTO users",
            "INSERT INTO employee_device_map",
            "INSERT INTO work_calendar",
            "INSERT INTO holidays",
            "INSERT INTO shift_swaps",
            "INSERT INTO app_settings",
            "UPDATE employees",
            "UPDATE branches",
            "UPDATE departments",
            "UPDATE positions",
            "UPDATE shifts",
            "UPDATE shift_assignments",
            "UPDATE devices",
            "UPDATE leave",
            "UPDATE overtime",
            "UPDATE users",
            "UPDATE employee_device_map",
            "UPDATE attendance_policy",
            "UPDATE work_calendar",
            "UPDATE holidays",
            "UPDATE shift_swaps",
            "UPDATE app_settings",
            "DELETE FROM branches",
            "DELETE FROM departments",
            "DELETE FROM positions",
            "DELETE FROM shifts",
            "DELETE FROM shift_assignments",
            "DELETE FROM devices",
            "DELETE FROM leave",
            "DELETE FROM overtime",
            "DELETE FROM users",
            "DELETE FROM employee_device_map",
            "DELETE FROM work_calendar",
            "DELETE FROM holidays",
            "DELETE FROM employees",
            "DELETE FROM shift_swaps",
            "UPDATE devices SET",
            "UPDATE employees SET",
            "INSERT INTO audit",
            "SELECT 1",
            "UPDATE attendance_results",
            "UPDATE raw_events",
        ];
        if !ok_prefix.iter().any(|p| sql.starts_with(p)) {
            return Err(AppError::Validation("statement tidak diizinkan".into()));
        }
        let guard = self
            .conn
            .lock()
            .map_err(|e| AppError::Db(format!("lock: {e}")))?;
        let mut stmt = guard.prepare(sql)?;
        let refs: Vec<&dyn ToSql> = vals.iter().map(|v| v as &dyn ToSql).collect();
        Ok(stmt.execute(refs.as_slice())?)
    }
}
