//! Autentikasi lokal: Argon2 hashing, session token, permission-based roles.
//! Tidak ada kredensial default — admin pertama dibuat via setup sekali pakai.

use crate::db::Repo;
use crate::errors::{AppError, AppResult};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    SuperAdmin,
    HrAdmin,
    Supervisor,
    OperatorDevice,
    Viewer,
}

impl Role {
    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "super_admin" => Some(Self::SuperAdmin),
            "hr_admin" => Some(Self::HrAdmin),
            "supervisor" => Some(Self::Supervisor),
            "operator_device" => Some(Self::OperatorDevice),
            "viewer" => Some(Self::Viewer),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SuperAdmin => "super_admin",
            Self::HrAdmin => "hr_admin",
            Self::Supervisor => "supervisor",
            Self::OperatorDevice => "operator_device",
            Self::Viewer => "viewer",
        }
    }
}

/// Permission per role — dicek di setiap command sensitif, bukan hanya di UI.
pub fn permissions(role: &Role) -> HashSet<&'static str> {
    let mut s = HashSet::new();
    match role {
        Role::SuperAdmin => {
            for p in [
                "employee.*",
                "device.*",
                "shift.*",
                "attendance.*",
                "leave.*",
                "report.*",
                "user.*",
                "backup.*",
                "correction.*",
                "sync.*",
            ] {
                s.insert(p);
            }
        }
        Role::HrAdmin => {
            for p in [
                "employee.*",
                "shift.*",
                "attendance.*",
                "leave.*",
                "report.*",
                "correction.*",
            ] {
                s.insert(p);
            }
        }
        Role::Supervisor => {
            for p in [
                "attendance.view",
                "leave.approve",
                "report.view",
                "shift.view",
            ] {
                s.insert(p);
            }
        }
        Role::OperatorDevice => {
            for p in ["device.*", "sync.*", "attendance.view"] {
                s.insert(p);
            }
        }
        Role::Viewer => {
            for p in ["attendance.view", "report.view"] {
                s.insert(p);
            }
        }
    }
    s
}

pub fn hash_password(plain: &str) -> AppResult<String> {
    if plain.len() < 8 {
        return Err(AppError::Validation("password minimal 8 karakter".into()));
    }
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|e| AppError::Auth(e.to_string()))?
        .to_string())
}

pub fn verify_password(hash: &str, plain: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(plain.as_bytes(), &parsed)
        .is_ok()
}

#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub role: Role,
    pub created_at: std::time::Instant,
}

pub struct SessionStore {
    inner: Mutex<HashMap<String, Session>>,
    timeout_secs: std::sync::RwLock<u64>,
}

impl SessionStore {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            timeout_secs: std::sync::RwLock::new(timeout_secs),
        }
    }
    pub fn set_timeout(&self, secs: u64) {
        *self.timeout_secs.write().unwrap() = secs.max(60);
    }
    pub fn timeout(&self) -> u64 {
        *self.timeout_secs.read().unwrap()
    }
    pub fn create(&self, user_id: &str, username: &str, role: Role) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        self.inner.lock().unwrap().insert(
            token.clone(),
            Session {
                token: token.clone(),
                user_id: user_id.into(),
                username: username.into(),
                role,
                created_at: std::time::Instant::now(),
            },
        );
        token
    }
    pub fn get(&self, token: &str) -> Option<Session> {
        let map = self.inner.lock().unwrap();
        let s = map.get(token)?.clone();
        if s.created_at.elapsed().as_secs() > self.timeout() {
            return None;
        }
        Some(s)
    }
    pub fn invalidate(&self, token: &str) {
        self.inner.lock().unwrap().remove(token);
    }
    /// Rate-limit login sederhana: hitung kegagalan per username dalam memori.
    pub fn check_permission(&self, token: &str, perm: &str) -> AppResult<Session> {
        let s = self
            .get(token)
            .ok_or_else(|| AppError::Auth("sesi tidak valid/kedaluwarsa".into()))?;
        let perms = permissions(&s.role);
        let ok = perms.contains(perm)
            || perms
                .iter()
                .any(|p| p.ends_with(".*") && perm.starts_with(&p[..p.len() - 2]));
        if !ok {
            return Err(AppError::Forbidden(format!(
                "role {} tidak punya {perm}",
                s.role.as_str()
            )));
        }
        Ok(s)
    }
}

pub fn login(
    repo: &Repo,
    sessions: &SessionStore,
    username: &str,
    password: &str,
) -> AppResult<(String, String, String)> {
    if is_locked_out(username) {
        return Err(AppError::Auth(
            "terlalu banyak percobaan, coba lagi 5 menit".into(),
        ));
    }
    let row: Option<(String, String, String, i64)> = repo.query_one(
        "SELECT id, username, role, is_active FROM users WHERE username=?1",
        &[&username],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let Some((uid, uname, role_s, active)) = row else {
        mark_fail(username);
        return Err(AppError::Auth("username/password salah".into()));
    };
    if active == 0 {
        return Err(AppError::Auth("akun nonaktif".into()));
    }
    let hash: Option<String> = repo.query_one(
        "SELECT password_hash FROM users WHERE id=?1",
        &[&uid],
        |r| r.get(0),
    )?;
    if hash
        .as_deref()
        .map(|h| verify_password(h, password))
        .unwrap_or(false)
    {
        clear_fail(username);
        let role = Role::from_code(&role_s).unwrap_or(Role::Viewer);
        let token = sessions.create(&uid, &uname, role);
        let now = crate::timex::now_wib_string();
        repo.exec(
            "UPDATE users SET last_login_at=?1 WHERE id=?2",
            &[&now, &uid],
        )?;
        log::info!("login ok: {username}");
        Ok((token, uname, role_s))
    } else {
        mark_fail(username);
        log::warn!("login gagal: {username}");
        Err(AppError::Auth("username/password salah".into()))
    }
}

fn fails_map() -> &'static Mutex<HashMap<String, (u32, std::time::Instant)>> {
    use std::sync::OnceLock;
    static FAILS: OnceLock<Mutex<HashMap<String, (u32, std::time::Instant)>>> = OnceLock::new();
    FAILS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_locked_out(username: &str) -> bool {
    let m = fails_map().lock().unwrap();
    matches!(m.get(username), Some((n, t)) if *n >= 5 && t.elapsed().as_secs() < 300)
}

fn mark_fail(username: &str) {
    let mut m = fails_map().lock().unwrap();
    let e = m
        .entry(username.to_string())
        .or_insert((0, std::time::Instant::now()));
    e.0 += 1;
    e.1 = std::time::Instant::now();
}

fn clear_fail(username: &str) {
    fails_map().lock().unwrap().remove(username);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_verify_roundtrip() {
        let h = hash_password("Rahasia123").unwrap();
        assert!(verify_password(&h, "Rahasia123"));
        assert!(!verify_password(&h, "salah"));
        assert!(!h.contains("Rahasia123"));
    }
    #[test]
    fn viewer_cannot_manage_users() {
        let store = SessionStore::new(3600);
        let t = store.create("u1", "view", Role::Viewer);
        assert!(store.check_permission(&t, "user.create").is_err());
        assert!(store.check_permission(&t, "report.view").is_ok());
    }
    #[test]
    fn super_admin_wildcard() {
        let store = SessionStore::new(3600);
        let t = store.create("u1", "sa", Role::SuperAdmin);
        assert!(store.check_permission(&t, "device.sync").is_ok());
    }
    #[test]
    fn weak_password_rejected() {
        assert!(hash_password("abc").is_err());
    }
}
