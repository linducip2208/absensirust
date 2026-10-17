#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use attendx_rust_lib::{auth::SessionStore, db::Repo, AppState};
use std::path::PathBuf;

fn app_dir() -> PathBuf {
    dirs_data_dir()
}

fn dirs_data_dir() -> PathBuf {
    if let Some(base) = std::env::var_os("APPDATA") {
        PathBuf::from(base).join("AttendX")
    } else {
        PathBuf::from(".").join("data")
    }
}

fn main() {
    let dir = app_dir();
    std::fs::create_dir_all(&dir).expect("buat direktori data");
    attendx_rust_lib::applog::init(&dir);
    log::info!("AttendX start v{}", env!("CARGO_PKG_VERSION"));
    let db_path = dir.join("attendx.db");
    let repo = Repo::open(&db_path)
        .unwrap_or_else(|e| panic!("gagal membuka database {}: {e}", db_path.display()));
    let sessions = SessionStore::new(8 * 3600);
    if let Ok(Some(v)) = repo.query_one::<String, _>(
        "SELECT value FROM app_settings WHERE key='session_timeout_secs'",
        &[],
        |r| r.get(0),
    ) {
        if let Ok(secs) = v.trim().parse::<u64>() {
            sessions.set_timeout(secs);
        }
    }
    let state = AppState {
        repo,
        sessions,
        db_path,
        sched_enabled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        sched_running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        sched_task: std::sync::Mutex::new(None),
    };
    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            attendx_rust_lib::commands::has_admin,
            attendx_rust_lib::commands::setup_admin,
            attendx_rust_lib::commands::login,
            attendx_rust_lib::commands::logout,
            attendx_rust_lib::commands::dashboard,
            attendx_rust_lib::commands::table_list,
            attendx_rust_lib::commands::table_get,
            attendx_rust_lib::commands::table_create,
            attendx_rust_lib::commands::table_update,
            attendx_rust_lib::commands::table_delete,
            attendx_rust_lib::commands::device_test,
            attendx_rust_lib::commands::device_sync,
            attendx_rust_lib::commands::device_registry,
            attendx_rust_lib::commands::device_sdk_status,
            attendx_rust_lib::commands::attendance_compute,
            attendx_rust_lib::commands::attendance_review,
            attendx_rust_lib::commands::attendance_correct,
            attendx_rust_lib::commands::employee_import_preview,
            attendx_rust_lib::commands::employee_import_commit,
            attendx_rust_lib::commands::report_query,
            attendx_rust_lib::commands::report_export,
            attendx_rust_lib::commands::backup_create,
            attendx_rust_lib::commands::backup_restore,
            attendx_rust_lib::commands::health_check,
            attendx_rust_lib::commands::settings_get,
            attendx_rust_lib::commands::settings_set,
            attendx_rust_lib::commands::schedule_week,
            attendx_rust_lib::commands::schedule_bulk,
            attendx_rust_lib::commands::schedule_copy_week,
            attendx_rust_lib::commands::schedule_conflicts,
            attendx_rust_lib::commands::swap_request,
            attendx_rust_lib::commands::swap_decide,
            attendx_rust_lib::commands::sync_run_all,
            attendx_rust_lib::commands::scheduler_get,
            attendx_rust_lib::commands::scheduler_set,
            attendx_rust_lib::commands::save_employee_photo,
            attendx_rust_lib::commands::save_leave_attachment,
            attendx_rust_lib::commands::events_requeue,
            attendx_rust_lib::commands::device_fetch_users,
            attendx_rust_lib::commands::device_provision_user,
            attendx_rust_lib::commands::leave_balances,
            attendx_rust_lib::commands::leave_decide,
            attendx_rust_lib::commands::overtime_decide,
            attendx_rust_lib::commands::search_global,
            attendx_rust_lib::commands::schedule_export,
            attendx_rust_lib::commands::schedule_import_preview,
            attendx_rust_lib::commands::schedule_import_commit,
            attendx_rust_lib::commands::diagnostics_bundle,
        ])
        .run(tauri::generate_context!())
        .expect("gagal menjalankan AttendX");
}
