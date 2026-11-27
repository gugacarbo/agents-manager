use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

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
    pub environment: agents_manager_core::domain::SshEnvironment,
    pub revision: u64,
    pub document: agents_manager_core::domain::RemoteConfigDocument,
    pub requires_verification: bool,
}

#[derive(Default)]
pub struct RemoteState {
    pub sessions: std::sync::Mutex<HashMap<String, RemoteSession>>,
    pub reviews: std::sync::Mutex<ReviewVault<String>>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
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
}
