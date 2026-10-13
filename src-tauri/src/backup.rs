//! Backup SQLite manual & terjadwal: copy file + validasi + rotasi.
//! Restore memvalidasi header SQLite sebelum mengganti database aktif.

use crate::errors::{AppError, AppResult};
use std::path::{Path, PathBuf};

pub fn backup_db(db_path: &Path, backup_dir: &Path, keep: usize) -> AppResult<PathBuf> {
    std::fs::create_dir_all(backup_dir)?;
    let stamp = crate::timex::now_wib_string().replace([':', ' '], "-");
    let dest = backup_dir.join(format!("attendx-backup-{stamp}.db"));
    std::fs::copy(db_path, &dest)?;
    validate_sqlite(&dest)?;
    rotate(backup_dir, keep)?;
    Ok(dest)
}

pub fn validate_sqlite(path: &Path) -> AppResult<()> {
    let data = std::fs::read(path)?;
    if data.len() < 100 || &data[0..16] != b"SQLite format 3\0" {
        return Err(AppError::Validation(format!(
            "file bukan database SQLite valid: {}",
            path.display()
        )));
    }
    Ok(())
}

fn rotate(dir: &Path, keep: usize) -> AppResult<()> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "db").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| e.file_name());
    while files.len() > keep.max(1) {
        let oldest = files.remove(0);
        std::fs::remove_file(oldest.path())?;
    }
    Ok(())
}

pub fn restore_db(db_path: &Path, backup_file: &Path) -> AppResult<()> {
    validate_sqlite(backup_file)?;
    // Simpan salinan pengaman sebelum replace.
    let stamp = crate::timex::now_wib_string().replace([':', ' '], "-");
    if db_path.exists() {
        let safety = db_path.with_extension(format!("pre-restore-{stamp}.db"));
        std::fs::copy(db_path, &safety)?;
    }
    std::fs::copy(backup_file, db_path)?;
    Ok(())
}

pub fn integrity_check(db_path: &Path) -> AppResult<String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| AppError::Db(e.to_string()))?;
    let s: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("app.db");
        {
            let c = rusqlite::Connection::open(&db).unwrap();
            c.execute_batch("CREATE TABLE t (a TEXT); INSERT INTO t VALUES ('x');")
                .unwrap();
        }
        let bdir = dir.path().join("backups");
        let dest = backup_db(&db, &bdir, 3).unwrap();
        assert!(dest.exists());
        std::fs::write(&db, b"rusak").unwrap();
        restore_db(&db, &dest).unwrap();
        assert_eq!(integrity_check(&db).unwrap(), "ok");
    }
    #[test]
    fn restore_rejects_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.db");
        std::fs::write(&bad, b"bukan sqlite").unwrap();
        assert!(restore_db(&dir.path().join("app.db"), &bad).is_err());
    }
}
