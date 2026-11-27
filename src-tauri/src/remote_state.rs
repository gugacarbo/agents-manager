use agents_manager_core::{
    domain::{RemoteConfigDocument, SshEnvironment},
    error::CoreError,
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

/// Maximum number of active remote editing sessions retained in memory.
pub const MAX_REMOTE_SESSIONS: usize = 32;

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
pub fn next_token() -> String {
    format!("remote-{}", NEXT_TOKEN.fetch_add(1, Ordering::Relaxed))
}

pub struct ReviewVault<T> {
    reviews: HashMap<String, (String, T)>,
}
impl<T> Default for ReviewVault<T> {
    fn default() -> Self {
        Self {
            reviews: HashMap::new(),
        }
    }
}
impl<T> ReviewVault<T> {
    pub fn issue(&mut self, session: String, draft: T) -> String {
        let token = next_token();
        self.reviews.insert(token.clone(), (session, draft));
        token
    }
    pub fn invalidate(&mut self, session: &str) {
        self.reviews.retain(|_, (owner, _)| owner != session);
    }

    pub fn consume(&mut self, token: &str) -> Option<(String, T)> {
        let review = self.reviews.remove(token)?;
        self.reviews.retain(|_, (session, _)| *session != review.0);
        Some(review)
    }
}

#[derive(Clone)]
pub struct RemoteSession {
    pub environment: SshEnvironment,
    pub revision: u64,
    pub document: RemoteConfigDocument,
    pub requires_verification: bool,
}

/// Bounded FIFO store of active remote editing sessions.
#[derive(Default)]
pub struct SessionStore {
    sessions: HashMap<String, RemoteSession>,
    order: VecDeque<String>,
}

impl SessionStore {
    /// Looks up a session by handle.
    pub fn get(&self, handle: &str) -> Option<&RemoteSession> {
        self.sessions.get(handle)
    }

    /// Looks up a mutable reference to a session by handle.
    pub fn get_mut(&mut self, handle: &str) -> Option<&mut RemoteSession> {
        self.sessions.get_mut(handle)
    }

    /// Inserts a session, evicting the oldest entries if already at [`MAX_REMOTE_SESSIONS`].
    ///
    /// Returns the handles of any evicted sessions so callers can invalidate dependent state.
    pub fn insert(&mut self, handle: String, session: RemoteSession) -> Vec<String> {
        let mut evicted = Vec::new();
        if self.sessions.contains_key(&handle) {
            self.order.retain(|existing| existing != &handle);
        } else {
            while self.sessions.len() >= MAX_REMOTE_SESSIONS {
                let Some(oldest) = self.order.pop_front() else {
                    break;
                };
                if self.sessions.remove(&oldest).is_some() {
                    evicted.push(oldest);
                }
            }
        }
        self.order.push_back(handle.clone());
        self.sessions.insert(handle, session);
        evicted
    }

    /// Removes and returns a session by handle.
    pub fn remove(&mut self, handle: &str) -> Option<RemoteSession> {
        let removed = self.sessions.remove(handle)?;
        self.order.retain(|existing| existing != handle);
        Some(removed)
    }
}

fn state_error() -> CoreError {
    CoreError::IoError {
        message: "Remote editor state unavailable.".into(),
    }
}

#[derive(Default)]
pub struct RemoteState {
    sessions: Mutex<SessionStore>,
    reviews: Mutex<ReviewVault<String>>,
}

impl RemoteState {
    /// Returns a cloned session for `handle`, or `CoreError::RemoteDestinationChanged` if missing.
    pub fn get_session(&self, handle: &str) -> Result<RemoteSession, CoreError> {
        self.sessions
            .lock()
            .map_err(|_| state_error())?
            .get(handle)
            .cloned()
            .ok_or(CoreError::RemoteDestinationChanged)
    }

    /// Stores a new session under `handle`, evicting the oldest session(s) beyond capacity
    /// and invalidating any pending reviews bound to evicted handles.
    pub fn insert_session(&self, handle: String, session: RemoteSession) -> Result<(), CoreError> {
        let mut sessions = self.sessions.lock().map_err(|_| state_error())?;
        let mut reviews = self.reviews.lock().map_err(|_| state_error())?;
        for evicted in sessions.insert(handle, session) {
            reviews.invalidate(&evicted);
        }
        Ok(())
    }

    /// Issues a review token bound to an active, unflagged session handle.
    pub fn issue_review(&self, handle: &str, content: String) -> Result<String, CoreError> {
        let sessions = self.sessions.lock().map_err(|_| state_error())?;
        let session = sessions
            .get(handle)
            .ok_or(CoreError::RemoteDestinationChanged)?;
        if session.requires_verification {
            return Err(CoreError::RemoteDestinationChanged);
        }
        let mut reviews = self.reviews.lock().map_err(|_| state_error())?;
        Ok(reviews.issue(handle.to_owned(), content))
    }

    /// Atomically consumes a review token and marks its owning session as requiring verification.
    pub fn begin_save(&self, review_id: &str) -> Result<(RemoteSession, String), CoreError> {
        let mut sessions = self.sessions.lock().map_err(|_| state_error())?;
        let mut reviews = self.reviews.lock().map_err(|_| state_error())?;
        let (handle, content) = reviews
            .consume(review_id)
            .ok_or(CoreError::RemoteDestinationChanged)?;
        let session = sessions
            .get_mut(&handle)
            .ok_or(CoreError::RemoteDestinationChanged)?;
        if session.requires_verification {
            return Err(CoreError::RemoteDestinationChanged);
        }
        session.requires_verification = true;
        Ok((session.clone(), content))
    }

    /// Removes a session and invalidates any pending reviews bound to it.
    pub fn remove_session(&self, handle: &str) -> Result<(), CoreError> {
        let mut sessions = self.sessions.lock().map_err(|_| state_error())?;
        let mut reviews = self.reviews.lock().map_err(|_| state_error())?;
        sessions.remove(handle);
        reviews.invalidate(handle);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use agents_manager_core::domain::{
        AgentKind, ConfigFormat, RemoteConfigEntry, RemoteConfigId, SshConnection,
    };

    fn sample_session(index: usize) -> RemoteSession {
        RemoteSession {
            environment: SshEnvironment {
                id: format!("env-{index}"),
                name: format!("Env {index}"),
                connection: SshConnection::Alias {
                    alias: format!("host-{index}"),
                },
            },
            revision: index as u64,
            document: RemoteConfigDocument {
                entry: RemoteConfigEntry {
                    id: RemoteConfigId::CodexConfig,
                    agent: AgentKind::Codex,
                    label: "config".into(),
                    format: ConfigFormat::Toml,
                },
                host: format!("host-{index}"),
                user: "user".into(),
                port: 22,
                path: format!("/home/user/.codex/config-{index}.toml"),
                content: format!("content-{index}"),
                sha256: format!("sha-{index}"),
                secret_spans: vec![],
                host_key: "key".into(),
                device: 1,
                inode: index as u64,
                mode: 0o600,
                uid: 1,
                gid: 1,
                metadata_hash: format!("meta-{index}"),
            },
            requires_verification: false,
        }
    }

    #[test]
    fn confirmation_is_bound_to_content_and_consumed_once() {
        let mut vault = ReviewVault::default();
        let token = vault.issue("session".into(), "draft".to_owned());
        assert_eq!(
            vault.consume(&token),
            Some(("session".into(), "draft".to_owned()))
        );
        assert!(vault.consume(&token).is_none());
    }
    #[test]
    fn consuming_a_review_invalidates_other_reviews_for_same_session() {
        let mut vault = ReviewVault::default();
        let first = vault.issue("session".into(), "one".to_owned());
        let second = vault.issue("session".into(), "two".to_owned());
        assert!(vault.consume(&first).is_some());
        assert!(vault.consume(&second).is_none());
    }

    #[test]
    fn session_store_evicts_oldest_when_exceeding_max_capacity() {
        let mut store = SessionStore::default();
        for index in 0..MAX_REMOTE_SESSIONS {
            let evicted = store.insert(format!("handle-{index}"), sample_session(index));
            assert!(evicted.is_empty());
        }
        assert_eq!(store.sessions.len(), MAX_REMOTE_SESSIONS);

        let evicted = store.insert(
            format!("handle-{MAX_REMOTE_SESSIONS}"),
            sample_session(MAX_REMOTE_SESSIONS),
        );
        assert_eq!(evicted, vec!["handle-0".to_owned()]);
        assert_eq!(store.sessions.len(), MAX_REMOTE_SESSIONS);
        assert!(store.get("handle-0").is_none());
        assert!(store.get("handle-1").is_some());
        assert!(
            store
                .get(&format!("handle-{MAX_REMOTE_SESSIONS}"))
                .is_some()
        );

        // Removing a session frees capacity so the next insertion does not evict.
        assert!(store.remove("handle-1").is_some());
        let next = MAX_REMOTE_SESSIONS + 1;
        assert!(
            store
                .insert(format!("handle-{next}"), sample_session(next))
                .is_empty()
        );
        assert_eq!(store.sessions.len(), MAX_REMOTE_SESSIONS);
        assert!(store.get("handle-2").is_some());
    }

    #[test]
    fn evicting_session_invalidates_its_pending_reviews() {
        let state = RemoteState::default();
        state
            .insert_session("s-0".into(), sample_session(0))
            .unwrap();
        let evicted_review = state.issue_review("s-0", "draft-0".into()).unwrap();

        state
            .insert_session("s-1".into(), sample_session(1))
            .unwrap();
        let retained_review = state.issue_review("s-1", "draft-1".into()).unwrap();

        for index in 2..=MAX_REMOTE_SESSIONS {
            state
                .insert_session(format!("s-{index}"), sample_session(index))
                .unwrap();
        }

        assert!(matches!(
            state.get_session("s-0"),
            Err(CoreError::RemoteDestinationChanged)
        ));
        assert!(
            state
                .reviews
                .lock()
                .unwrap()
                .consume(&evicted_review)
                .is_none()
        );
        assert!(matches!(
            state.begin_save(&evicted_review),
            Err(CoreError::RemoteDestinationChanged)
        ));

        assert!(state.get_session("s-1").is_ok());
        let (session, content) = state.begin_save(&retained_review).unwrap();
        assert_eq!(content, "draft-1");
        assert!(session.requires_verification);
    }
}
