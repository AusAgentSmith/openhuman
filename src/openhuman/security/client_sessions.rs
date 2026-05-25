//! Device-scoped client sessions for desktop/web/mobile RPC clients.
//!
//! The bootstrap `core.token` remains available for local admin setup, but
//! mobile clients should use named session tokens from this store so one lost
//! device can be revoked without rotating the whole server bearer.

use std::fs::OpenOptions;
use std::io::Write as _;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SESSION_FILE: &str = "client-sessions.json";
const TOKEN_PREFIX: &str = "ohs_";

static GLOBAL_STORE: OnceLock<ClientSessionStore> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSession {
    pub id: String,
    pub label: String,
    pub token_prefix: String,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedClientSession {
    pub session: ClientSession,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSessionSummary {
    pub initialized: bool,
    pub active_count: usize,
    pub revoked_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredClientSession {
    id: String,
    label: String,
    token_hash: String,
    token_prefix: String,
    created_at: String,
    last_seen_at: Option<String>,
    revoked_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientSessionFile {
    #[serde(default)]
    sessions: Vec<StoredClientSession>,
}

pub struct ClientSessionStore {
    path: PathBuf,
    inner: Mutex<ClientSessionFile>,
}

impl ClientSessionStore {
    pub fn open(workspace_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(workspace_dir).with_context(|| {
            format!(
                "failed to create client session workspace {}",
                workspace_dir.display()
            )
        })?;
        let path = workspace_dir.join(SESSION_FILE);
        let file = if path.exists() {
            let raw = std::fs::read_to_string(&path).with_context(|| {
                format!("failed to read client session file {}", path.display())
            })?;
            serde_json::from_str::<ClientSessionFile>(&raw).with_context(|| {
                format!("failed to parse client session file {}", path.display())
            })?
        } else {
            ClientSessionFile::default()
        };
        Ok(Self {
            path,
            inner: Mutex::new(file),
        })
    }

    pub fn issue(&self, label: Option<&str>) -> Result<CreatedClientSession> {
        let id = uuid::Uuid::new_v4().to_string();
        let token = generate_token(&id);
        let now = Utc::now().to_rfc3339();
        let stored = StoredClientSession {
            id,
            label: normalize_label(label),
            token_hash: hash_token(&token),
            token_prefix: public_token_prefix(&token),
            created_at: now,
            last_seen_at: None,
            revoked_at: None,
        };

        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.sessions.push(stored.clone());
        self.persist_locked(&guard)?;

        Ok(CreatedClientSession {
            session: stored.into_public(),
            token,
        })
    }

    pub fn list(&self) -> Vec<ClientSession> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard
            .sessions
            .iter()
            .cloned()
            .map(StoredClientSession::into_public)
            .collect()
    }

    pub fn revoke(&self, session_id: &str) -> Result<Option<ClientSession>> {
        let trimmed = session_id.trim();
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(idx) = guard.sessions.iter().position(|entry| entry.id == trimmed) else {
            return Ok(None);
        };
        if guard.sessions[idx].revoked_at.is_none() {
            guard.sessions[idx].revoked_at = Some(Utc::now().to_rfc3339());
            self.persist_locked(&guard)?;
        }
        Ok(Some(guard.sessions[idx].clone().into_public()))
    }

    pub fn authenticate(&self, token: &str) -> bool {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return false;
        }
        let hashed = hash_token(trimmed);
        let now = Utc::now().to_rfc3339();
        let mut matched = false;
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for session in &mut guard.sessions {
            if session.revoked_at.is_none() && constant_time_eq(&session.token_hash, &hashed) {
                session.last_seen_at = Some(now);
                matched = true;
                break;
            }
        }
        if matched {
            if let Err(err) = self.persist_locked(&guard) {
                tracing::warn!(error = %err, "[security:sessions] failed to persist last_seen_at");
            }
        }
        matched
    }

    pub fn summary(&self) -> ClientSessionSummary {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        summarize(true, &guard.sessions)
    }

    fn persist_locked(&self, file: &ClientSessionFile) -> Result<()> {
        let serialized =
            serde_json::to_string_pretty(file).context("failed to serialize client sessions")?;
        let temp_path = self
            .path
            .with_file_name(format!(".{SESSION_FILE}.tmp-{}", uuid::Uuid::new_v4()));

        #[cfg(unix)]
        {
            let mut temp = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temp_path)
                .with_context(|| {
                    format!(
                        "failed to open temporary client session file {}",
                        temp_path.display()
                    )
                })?;
            temp.write_all(serialized.as_bytes())?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&temp_path, serialized.as_bytes()).with_context(|| {
                format!(
                    "failed to write temporary client session file {}",
                    temp_path.display()
                )
            })?;
        }

        std::fs::rename(&temp_path, &self.path).with_context(|| {
            let _ = std::fs::remove_file(&temp_path);
            format!(
                "failed to replace client session file {}",
                self.path.display()
            )
        })?;
        Ok(())
    }
}

impl StoredClientSession {
    fn into_public(self) -> ClientSession {
        ClientSession {
            id: self.id,
            label: self.label,
            token_prefix: self.token_prefix,
            created_at: self.created_at,
            last_seen_at: self.last_seen_at,
            revoked_at: self.revoked_at,
        }
    }
}

pub fn init_global(workspace_dir: &Path) -> Result<()> {
    if GLOBAL_STORE.get().is_some() {
        return Ok(());
    }
    let store = ClientSessionStore::open(workspace_dir)?;
    let _ = GLOBAL_STORE.set(store);
    Ok(())
}

pub fn global_summary() -> ClientSessionSummary {
    match GLOBAL_STORE.get() {
        Some(store) => store.summary(),
        None => ClientSessionSummary {
            initialized: false,
            active_count: 0,
            revoked_count: 0,
            total_count: 0,
        },
    }
}

pub fn issue_global(label: Option<&str>) -> Result<CreatedClientSession> {
    global_store()?.issue(label)
}

pub fn list_global() -> Result<Vec<ClientSession>> {
    Ok(global_store()?.list())
}

pub fn revoke_global(session_id: &str) -> Result<Option<ClientSession>> {
    global_store()?.revoke(session_id)
}

pub fn authenticate_global(token: &str) -> bool {
    GLOBAL_STORE
        .get()
        .map(|store| store.authenticate(token))
        .unwrap_or(false)
}

fn global_store() -> Result<&'static ClientSessionStore> {
    GLOBAL_STORE
        .get()
        .context("client session store is not initialized")
}

fn summarize(initialized: bool, sessions: &[StoredClientSession]) -> ClientSessionSummary {
    let revoked_count = sessions
        .iter()
        .filter(|session| session.revoked_at.is_some())
        .count();
    ClientSessionSummary {
        initialized,
        active_count: sessions.len().saturating_sub(revoked_count),
        revoked_count,
        total_count: sessions.len(),
    }
}

fn normalize_label(label: Option<&str>) -> String {
    let trimmed = label.map(str::trim).unwrap_or("");
    if trimmed.is_empty() {
        "Unnamed client".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn generate_token(session_id: &str) -> String {
    use rand::RngExt as _;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    format!("{TOKEN_PREFIX}{session_id}_{}", hex::encode(bytes))
}

fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

fn public_token_prefix(token: &str) -> String {
    token.chars().take(18).collect()
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let len_diff = a.len() ^ b.len();
    let max_len = a.len().max(b.len());
    let mut diff = 0u8;
    for i in 0..max_len {
        let x = *a.get(i).unwrap_or(&0);
        let y = *b.get(i).unwrap_or(&0);
        diff |= x ^ y;
    }
    (len_diff == 0) & (diff == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_lists_authenticates_and_revokes_session_tokens() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = ClientSessionStore::open(dir.path()).unwrap();

        let created = store.issue(Some("phone")).unwrap();

        assert_eq!(created.session.label, "phone");
        assert!(created.token.starts_with(TOKEN_PREFIX));
        assert!(store.authenticate(&created.token));
        let listed = store.list();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].last_seen_at.is_some());

        let revoked = store.revoke(&created.session.id).unwrap().unwrap();
        assert!(revoked.revoked_at.is_some());
        assert!(!store.authenticate(&created.token));
    }

    #[test]
    fn persisted_sessions_reload_without_plaintext_token() {
        let dir = tempfile::TempDir::new().unwrap();
        let token = {
            let store = ClientSessionStore::open(dir.path()).unwrap();
            store.issue(Some("tablet")).unwrap().token
        };

        let raw = std::fs::read_to_string(dir.path().join(SESSION_FILE)).unwrap();
        assert!(!raw.contains(&token));

        let store = ClientSessionStore::open(dir.path()).unwrap();
        assert!(store.authenticate(&token));
        assert_eq!(store.summary().active_count, 1);
    }

    #[test]
    fn empty_label_gets_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = ClientSessionStore::open(dir.path()).unwrap();
        let created = store.issue(Some("  ")).unwrap();

        assert_eq!(created.session.label, "Unnamed client");
    }
}
