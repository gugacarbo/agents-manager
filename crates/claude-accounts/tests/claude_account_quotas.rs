#![allow(clippy::unwrap_used, clippy::expect_used)]
use agents_manager_claude_accounts::desktop_quotas::*;
use std::sync::Arc;

struct Source;
impl DesktopQuotaSource for Source {
    fn open(&self) -> Result<Box<dyn DesktopQuotaSession + '_>, DesktopQuotaError> {
        Ok(Box::new(Session))
    }
}
struct Session;
impl DesktopQuotaSession for Session {
    fn identity(&self) -> Result<DesktopAccountIdentity, DesktopQuotaError> {
        Ok(DesktopAccountIdentity {
            id: "a".into(),
            display_name: "Alice".into(),
        })
    }
    fn usage(&self) -> Result<DesktopQuotaWindows, DesktopQuotaError> {
        Ok(DesktopQuotaWindows {
            five_hour: None,
            seven_day: None,
        })
    }
}
#[test]
fn linking_does_not_collect_usage_and_refresh_obeys_interval() {
    let dir = tempfile::tempdir().unwrap();
    let service = DesktopQuotaService::new(
        dir.path().to_owned(),
        Arc::new(Source),
        Arc::new(|| 1_000_000),
    );
    let linked = service.link().unwrap();
    assert!(linked.last_success.is_none());
    assert_eq!(
        service.refresh().unwrap().status,
        DesktopQuotaRefreshStatus::Updated
    );
    assert_eq!(
        service.refresh().unwrap().status,
        DesktopQuotaRefreshStatus::Wait
    );
}

use std::sync::{
    Condvar, Mutex,
    atomic::{AtomicI64, AtomicUsize, Ordering},
};
struct MutableSource {
    id: Mutex<String>,
    fail: Mutex<Option<DesktopQuotaError>>,
    usage_calls: AtomicUsize,
    gate: Option<Arc<(Mutex<bool>, Condvar)>>,
}
impl DesktopQuotaSource for MutableSource {
    fn open(&self) -> Result<Box<dyn DesktopQuotaSession + '_>, DesktopQuotaError> {
        Ok(Box::new(MutableSession(self)))
    }
}
struct MutableSession<'a>(&'a MutableSource);
impl DesktopQuotaSession for MutableSession<'_> {
    fn identity(&self) -> Result<DesktopAccountIdentity, DesktopQuotaError> {
        Ok(DesktopAccountIdentity {
            id: self.0.id.lock().unwrap().clone(),
            display_name: "Alice".into(),
        })
    }
    fn usage(&self) -> Result<DesktopQuotaWindows, DesktopQuotaError> {
        self.0.usage_calls.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.0.gate {
            let (lock, cond) = &**gate;
            let mut open = lock.lock().unwrap();
            while !*open {
                open = cond.wait(open).unwrap();
            }
        }
        if let Some(error) = *self.0.fail.lock().unwrap() {
            return Err(error);
        }
        Ok(DesktopQuotaWindows {
            five_hour: Some(DesktopQuotaWindow {
                utilization: Some(25.0),
                resets_at: None,
            }),
            seven_day: None,
        })
    }
}
fn mutable_source() -> Arc<MutableSource> {
    Arc::new(MutableSource {
        id: Mutex::new("a".into()),
        fail: Mutex::new(None),
        usage_calls: AtomicUsize::new(0),
        gate: None,
    })
}
#[test]
fn mismatch_preserves_history_and_never_queries_usage() {
    let dir = tempfile::tempdir().unwrap();
    let source = mutable_source();
    let time = Arc::new(AtomicI64::new(1_000_000));
    let clock = {
        let time = time.clone();
        Arc::new(move || time.load(Ordering::SeqCst))
    };
    let service = DesktopQuotaService::new(dir.path().to_owned(), source.clone(), clock);
    service.link().unwrap();
    assert_eq!(source.usage_calls.load(Ordering::SeqCst), 0);
    let first = service.refresh().unwrap();
    assert_eq!(
        first
            .state
            .last_success
            .as_ref()
            .unwrap()
            .five_hour
            .as_ref()
            .unwrap()
            .utilization,
        Some(25.0)
    );
    time.store(1_060_000, Ordering::SeqCst);
    *source.id.lock().unwrap() = "b".into();
    let failed = service.refresh().unwrap();
    assert_eq!(failed.status, DesktopQuotaRefreshStatus::Failed);
    assert_eq!(
        failed.state.error,
        Some(DesktopQuotaError::IdentityMismatch)
    );
    assert_eq!(failed.state.last_success, first.state.last_success);
    assert_eq!(source.usage_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        service.link().unwrap_err(),
        DesktopQuotaError::IdentityMismatch
    );
    assert_eq!(
        service.state().unwrap().last_success,
        first.state.last_success
    );
}
#[test]
fn failure_rate_limit_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let source = mutable_source();
    *source.fail.lock().unwrap() = Some(DesktopQuotaError::RateLimited);
    let service = DesktopQuotaService::new(
        dir.path().to_owned(),
        source.clone(),
        Arc::new(|| 1_000_000),
    );
    service.link().unwrap();
    assert_eq!(
        service.refresh().unwrap().state.error,
        Some(DesktopQuotaError::RateLimited)
    );
    let restarted = DesktopQuotaService::new(
        dir.path().to_owned(),
        source.clone(),
        Arc::new(|| 1_059_999),
    );
    let wait = restarted.refresh().unwrap();
    assert_eq!(wait.status, DesktopQuotaRefreshStatus::Wait);
    assert_eq!(wait.retry_after_seconds, Some(1));
    assert_eq!(source.usage_calls.load(Ordering::SeqCst), 1);
}
#[test]
fn in_flight_request_remains_visible_after_interval() {
    let dir = tempfile::tempdir().unwrap();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let source = Arc::new(MutableSource {
        id: Mutex::new("a".into()),
        fail: Mutex::new(None),
        usage_calls: AtomicUsize::new(0),
        gate: Some(gate.clone()),
    });
    let time = Arc::new(AtomicI64::new(1_000_000));
    let clock = {
        let time = time.clone();
        Arc::new(move || time.load(Ordering::SeqCst))
    };
    let service = Arc::new(DesktopQuotaService::new(
        dir.path().to_owned(),
        source.clone(),
        clock,
    ));
    service.link().unwrap();
    let worker = {
        let service = service.clone();
        std::thread::spawn(move || service.refresh().unwrap())
    };
    while source.usage_calls.load(Ordering::SeqCst) == 0 {
        std::thread::yield_now();
    }
    time.store(1_100_000, Ordering::SeqCst);
    let in_this_instance = service.refresh().unwrap().status;
    let another_instance = DesktopQuotaService::new(
        dir.path().to_owned(),
        Arc::new(Source),
        Arc::new(|| 1_100_000),
    );
    let in_another_instance = another_instance.refresh().unwrap().status;
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    assert_eq!(
        worker.join().unwrap().status,
        DesktopQuotaRefreshStatus::Updated
    );
    assert_eq!(in_this_instance, DesktopQuotaRefreshStatus::InProgress);
    assert_eq!(in_another_instance, DesktopQuotaRefreshStatus::InProgress);
}

#[test]
fn desktop_profiles_persist_drafts_and_keep_default_history() {
    let dir = tempfile::tempdir().unwrap();
    let source = Arc::new(Source);
    let clock = Arc::new(|| 1_700_000_000_000);
    let old = DesktopQuotaService::new(dir.path().to_owned(), source.clone(), clock.clone());
    old.link().unwrap();
    old.refresh().unwrap();
    let profiles = DesktopQuotaProfiles::new(
        dir.path().to_owned(),
        Arc::new(move |_| source.clone()),
        clock,
    );
    let draft = profiles.create().unwrap();
    let listed = profiles.list().unwrap();
    assert_eq!(listed.len(), 2);
    assert!(listed[0].is_default);
    assert!(listed[0].state.last_success.is_some());
    assert_eq!(listed[1].id, draft.id);
    assert!(listed[1].state.account.is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = profiles.profile_dir(&draft.id).unwrap();
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    assert!(matches!(
        profiles.refresh("../state"),
        Err(DesktopQuotaError::ProfileNotFound)
    ));
}

#[test]
fn opening_profile_requires_persisted_identity_and_propagates_unavailable_storage() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = DesktopQuotaProfiles::new(
        dir.path().to_owned(),
        Arc::new(|_| Arc::new(Source)),
        Arc::new(|| 1_000_000),
    );
    let draft = profiles.create().unwrap();
    assert_eq!(
        profiles.ensure_linked(&draft.id),
        Err(DesktopQuotaError::Unauthenticated)
    );
    assert_eq!(
        profiles.ensure_linked("unknown"),
        Err(DesktopQuotaError::ProfileNotFound)
    );
    profiles.link(&draft.id).unwrap();
    assert_eq!(profiles.ensure_linked(&draft.id), Ok(()));

    std::fs::write(
        profiles.profile_dir(&draft.id).unwrap().join("state.json"),
        "unreadable state",
    )
    .unwrap();
    assert_eq!(
        profiles.ensure_linked(&draft.id),
        Err(DesktopQuotaError::StorageUnavailable)
    );
}

#[test]
fn unreadable_profile_state_does_not_hide_other_accounts() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = DesktopQuotaProfiles::new(
        dir.path().to_owned(),
        Arc::new(|_| Arc::new(Source)),
        Arc::new(|| 1_000_000),
    );
    profiles.link("default").unwrap();
    let draft = profiles.create().unwrap();
    let path = profiles.profile_dir(&draft.id).unwrap().join("state.json");
    std::fs::write(path, "invalid state").unwrap();

    let listed = profiles.list().unwrap();
    assert!(listed[0].state.account.is_some());
    assert_eq!(
        listed[1].state.error,
        Some(DesktopQuotaError::StorageUnavailable)
    );
    assert_eq!(
        profiles.refresh("default").unwrap().status,
        DesktopQuotaRefreshStatus::Updated
    );
}

#[test]
fn different_accounts_preserve_independent_results() {
    let dir = tempfile::tempdir().unwrap();
    let a = mutable_source();
    let b = mutable_source();
    let sources = Arc::new(Mutex::new(std::collections::HashMap::<
        String,
        Arc<MutableSource>,
    >::new()));
    sources.lock().unwrap().insert("default".into(), a.clone());
    let factory = {
        let sources = sources.clone();
        Arc::new(move |id: &str| -> Arc<dyn DesktopQuotaSource> {
            sources.lock().unwrap().get(id).unwrap().clone()
        })
    };
    let profiles =
        DesktopQuotaProfiles::new(dir.path().to_owned(), factory, Arc::new(|| 1_000_000));
    let draft = profiles.create().unwrap();
    sources.lock().unwrap().insert(draft.id.clone(), b.clone());
    profiles.link("default").unwrap();
    *b.id.lock().unwrap() = "b".into();
    profiles.link(&draft.id).unwrap();
    let good = profiles.refresh("default").unwrap();
    *b.fail.lock().unwrap() = Some(DesktopQuotaError::RateLimited);
    let failed = profiles.refresh(&draft.id).unwrap();
    assert_eq!(good.status, DesktopQuotaRefreshStatus::Updated);
    assert_eq!(failed.state.error, Some(DesktopQuotaError::RateLimited));
    assert!(
        profiles
            .link("default")
            .unwrap()
            .state
            .last_success
            .is_some()
    );
}

#[test]
fn permits_one_isolated_session_for_the_default_account_and_shares_the_quota() {
    let dir = tempfile::tempdir().unwrap();
    let source = Arc::new(Source);
    let factory = Arc::new(move |_: &str| -> Arc<dyn DesktopQuotaSource> { source.clone() });
    let profiles =
        DesktopQuotaProfiles::new(dir.path().to_owned(), factory, Arc::new(|| 1_000_000));
    let isolated = profiles.create().unwrap();
    profiles.link("default").unwrap();
    assert!(profiles.link(&isolated.id).unwrap().shares_default_quota);
    assert!(
        profiles
            .list()
            .unwrap()
            .into_iter()
            .any(|item| item.id == isolated.id && item.shares_default_quota)
    );

    assert_eq!(
        profiles.refresh("default").unwrap().status,
        DesktopQuotaRefreshStatus::Updated
    );
    assert_eq!(
        profiles.refresh(&isolated.id).unwrap().status,
        DesktopQuotaRefreshStatus::Wait
    );
    let duplicate = profiles.create().unwrap();
    assert_eq!(
        profiles.link(&duplicate.id).unwrap_err(),
        DesktopQuotaError::DuplicateAccount
    );
}

#[test]
fn simultaneous_links_cannot_claim_the_same_identity() {
    let dir = tempfile::tempdir().unwrap();
    let source = Arc::new(Source);
    let factory = Arc::new(move |_: &str| -> Arc<dyn DesktopQuotaSource> { source.clone() });
    let profiles = Arc::new(DesktopQuotaProfiles::new(
        dir.path().to_owned(),
        factory,
        Arc::new(|| 1_000_000),
    ));
    let first = profiles.create().unwrap();
    let second = profiles.create().unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let handles: Vec<_> = [first.id, second.id]
        .into_iter()
        .map(|id| {
            let profiles = profiles.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                profiles.link(&id)
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(DesktopQuotaError::DuplicateAccount)))
            .count(),
        1
    );
}
