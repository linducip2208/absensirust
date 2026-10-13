//! Log file aplikasi dengan rotasi harian (simpan 7 hari).
//! Tidak berisi kredensial, token, PIN, atau data biometrik — hanya info operasional.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

struct FileLogger;

impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &log::Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        if let Some(dir) = LOG_DIR.get() {
            let day = crate::timex::today_wib().format("%Y-%m-%d").to_string();
            let path = dir.join(format!("app-{day}.log"));
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = writeln!(
                    f,
                    "[{}][{}] {}",
                    crate::timex::now_wib_string(),
                    r.level(),
                    r.args()
                );
            }
        }
    }
    fn flush(&self) {}
}

static LOGGER: FileLogger = FileLogger;

pub fn init(data_dir: &Path) {
    let dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&dir);
    // Rotasi: hapus app-YYYY-MM-DD.log yang lebih dari 7 hari.
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !(name.starts_with("app-") && name.ends_with(".log")) {
                continue;
            }
            let date_part = name.trim_start_matches("app-").trim_end_matches(".log");
            if let Ok(dt) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
                if (crate::timex::today_wib() - dt).num_days() > 7 {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }
    let _ = LOG_DIR.set(dir);
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}

/// Ambil ekor log (maks N baris, dari file terbaru). Untuk bundel diagnostik.
pub fn tail_logs(max_lines: usize) -> String {
    let Some(dir) = LOG_DIR.get() else {
        return String::new();
    };
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().to_string();
                    n.starts_with("app-") && n.ends_with(".log")
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|e| e.file_name());
    let mut lines: Vec<String> = vec![];
    for f in files.iter().rev().take(3) {
        if let Ok(text) = std::fs::read_to_string(f.path()) {
            for l in text.lines().rev() {
                lines.push(l.to_string());
                if lines.len() >= max_lines {
                    break;
                }
            }
        }
        if lines.len() >= max_lines {
            break;
        }
    }
    lines.reverse();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn log_roundtrip_and_rotation() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path());
        log::info!("uji log {}", 123);
        let tail = tail_logs(50);
        assert!(tail.contains("uji log 123"), "tail: {tail}");
        // File lama (>7 hari) dirotasi pada init berikutnya.
        let old = dir.path().join("logs").join("app-2020-01-01.log");
        std::fs::write(&old, "lama").unwrap();
        init(dir.path());
        assert!(!old.exists());
    }
}
