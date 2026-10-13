use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("database: {0}")]
    Db(String),
    #[error("validasi: {0}")]
    Validation(String),
    #[error("tidak ditemukan: {0}")]
    NotFound(String),
    #[error("izin ditolak: {0}")]
    Forbidden(String),
    #[error("autentikasi gagal: {0}")]
    Auth(String),
    #[error("perangkat: {0}")]
    Device(String),
    #[error("konflik: {0}")]
    Conflict(String),
    #[error("io: {0}")]
    Io(String),
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(e: csv::Error) -> Self {
        AppError::Validation(format!("CSV: {e}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
