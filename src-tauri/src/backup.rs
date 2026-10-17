//! Backup SQLite manual & terjadwal: copy file + validasi + rotasi.
//! Restore memvalidasi header SQLite sebelum mengganti database aktif.

use crate::errors::{AppError, AppResult};
use std::path::{Path, PathBuf};

pub fn backup_db(db_path: &Path, backup_dir: &Path, keep: usize) -> AppResult<PathBuf> {
    std::fs::create_dir_all(backup_dir)?;
    let stamp = crate::timex::now_wib_string().replace([':', ' '], "-");
    let mut dest = backup_dir.join(format!("attendx-backup-{stamp}.db"));
    // Dua backup dalam satu detik tidak boleh saling menimpa — tambah suffix.
    let mut i = 0u32;
    while dest.exists() {
        i += 1;
        dest = backup_dir.join(format!("attendx-backup-{stamp}-{i}.db"));
    }
    std::fs::copy(db_path, &dest)?;
    // Validasi TYANG: header + integrity_check; gagal → buang file rusak
    // sebelum menimpa file backup lama via rotasi.
    if let Err(e) = validate_backup(&dest) {
        let _ = std::fs::remove_file(&dest);
        return Err(e);
    }
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

/// Validasi backup lebih dalam: header + integrity_check read-only
/// (tanpa membuat WAL di sebelah file).
pub fn validate_backup(path: &Path) -> AppResult<()> {
    validate_sqlite(path)?;
    let uri = format!("file:{}?mode=ro", path.display());
    let conn = rusqlite::Connection::open_with_flags(
        uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| AppError::Db(e.to_string()))?;
    let s: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| AppError::Db(e.to_string()))?;
    if s != "ok" {
        return Err(AppError::Validation(format!(
            "integrity_check backup gagal: {s}"
        )));
    }
    Ok(())
}

fn rotate(dir: &Path, keep: usize) -> AppResult<()> {
    // Hanya file backup milik sistem ("attendx-backup-*.db") yang dirotasi;
    // salinan pengaman pre-restore & file lain jangan dihapus tersasar.
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("attendx-backup-") && name.ends_with(".db")
        })
        .collect();
    files.sort_by_key(|e| e.file_name());
    while files.len() > keep.max(1) {
        let oldest = files.remove(0);
        std::fs::remove_file(oldest.path())?;
    }
    Ok(())
}

pub fn restore_db(db_path: &Path, backup_file: &Path) -> AppResult<()> {
    validate_backup(backup_file)?;
    // Simpan salinan pengaman sebelum replace.
    let stamp = crate::timex::now_wib_string().replace([':', ' '], "-");
    if db_path.exists() {
        let safety = db_path.with_extension(format!("pre-restore-{stamp}.db"));
        std::fs::copy(db_path, &safety)?;
    }
    // Buang WAL/SHM basi agar koneksi tak membaca halaman lama.
    remove_wal_sidecars(db_path);
    std::fs::copy(backup_file, db_path)?;
    if integrity_check(db_path)? != "ok" {
        return Err(AppError::Db("integrity_check gagal setelah restore".into()));
    }
    Ok(())
}

/// Hapus file -wal/-shm di sebelah database (dibuat ulang otomatis).
pub fn remove_wal_sidecars(db_path: &Path) {
    for ext in ["db-wal", "db-shm"] {
        let _ = std::fs::remove_file(db_path.with_extension(ext));
    }
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
    #[test]
    fn backup_dalam_satu_detik_tak_saling_timpa() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("app.db");
        {
            let c = rusqlite::Connection::open(&db).unwrap();
            c.execute_batch("CREATE TABLE t (a TEXT);").unwrap();
        }
        let bdir = dir.path().join("backups");
        let a = backup_db(&db, &bdir, 100).unwrap();
        let b = backup_db(&db, &bdir, 100).unwrap();
        assert!(a.exists() && b.exists());
        assert_ne!(a, b, "dua backup dalam detik sama harus beda nama");
    }
    #[test]
    fn rotate_tidak_menyentuh_pre_restore_dan_file_lain() {
        let dir = tempfile::tempdir().unwrap();
        let b = dir.path().join("b");
        std::fs::create_dir_all(&b).unwrap();
        // Salinan pengaman pre-restore + file asing harus selamat.
        for n in [
            "app.pre-restore-2026-10-13-08-00-00.db",
            "mitra-db-lama.db",
            "attendx-backup-2026-01-01-00-00-01.db",
            "attendx-backup-2026-01-02-00-00-01.db",
        ] {
            std::fs::write(b.join(n), b"").unwrap();
        }
        rotate(&b, 1).unwrap();
        let after: Vec<String> = std::fs::read_dir(&b)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(after.contains(&"app.pre-restore-2026-10-13-08-00-00.db".to_string()));
        assert!(after.contains(&"mitra-db-lama.db".to_string()));
        // Hanya satu attendx-backup tersisa (keep=1).
        assert_eq!(
            after
                .iter()
                .filter(|n| n.starts_with("attendx-backup-"))
                .count(),
            1
        );
    }
}
