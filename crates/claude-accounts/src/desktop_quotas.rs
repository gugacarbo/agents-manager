//! Explicit, account-bound Claude Desktop quota collection.
#[cfg(target_os = "linux")]
pub mod source;

use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    fs::{self, File},
    path::PathBuf,
    sync::Arc,
};

pub trait DesktopQuotaSource: Send + Sync {
    fn open(&self) -> Result<Box<dyn DesktopQuotaSession + '_>, DesktopQuotaError>;
}
pub trait DesktopQuotaSession {
    fn identity(&self) -> Result<DesktopAccountIdentity, DesktopQuotaError>;
    fn usage(&self) -> Result<DesktopQuotaWindows, DesktopQuotaError>;
}
#[derive(Clone, Debug)]
pub struct DesktopAccountIdentity {
    pub id: String,
    pub display_name: String,
}
#[derive(Clone, Debug)]
pub struct DesktopQuotaWindows {
    pub five_hour: Option<DesktopQuotaWindow>,
    pub seven_day: Option<DesktopQuotaWindow>,
}
#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DesktopQuotaWindow {
    pub utilization: Option<f64>,
    pub resets_at: Option<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopQuotaError {
    DesktopRunning,
    Unauthenticated,
    SourceUnavailable,
    SourceIncompatible,
    IdentityMismatch,
    RateLimited,
    NotLinked,
    StorageUnavailable,
    KeyringUnavailable,
    DuplicateAccount,
    ProfileNotFound,
}
#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DesktopQuotaAccount {
    pub display_name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DesktopQuotaReading {
    pub collected_at: String,
    pub five_hour: Option<DesktopQuotaWindow>,
    pub seven_day: Option<DesktopQuotaWindow>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, Type, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DesktopQuotaState {
    pub account: Option<DesktopQuotaAccount>,
    pub last_attempt_at: Option<String>,
    pub last_success: Option<DesktopQuotaReading>,
    pub error: Option<DesktopQuotaError>,
}
#[derive(Clone, Copy, Debug, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopQuotaRefreshStatus {
    Updated,
    Failed,
    InProgress,
    Wait,
}
#[derive(Clone, Debug, Serialize, Type)]
pub struct DesktopQuotaRefresh {
    pub status: DesktopQuotaRefreshStatus,
    pub state: DesktopQuotaState,
    pub retry_after_seconds: Option<u32>,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    identity: Option<String>,
    state: DesktopQuotaState,
    last_attempt_epoch_ms: Option<i64>,
}
pub struct DesktopQuotaService {
    dir: PathBuf,
    source: Arc<dyn DesktopQuotaSource>,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}
fn timestamp(epoch_ms: i64) -> Result<String, DesktopQuotaError> {
    chrono::DateTime::from_timestamp_millis(epoch_ms)
        .map(|t| t.to_rfc3339())
        .ok_or(DesktopQuotaError::StorageUnavailable)
}
impl DesktopQuotaService {
    pub fn new(
        dir: PathBuf,
        source: Arc<dyn DesktopQuotaSource>,
        clock_ms: Arc<dyn Fn() -> i64 + Send + Sync>,
    ) -> Self {
        Self {
            dir,
            source,
            clock: clock_ms,
        }
    }
    fn try_begin(&self) -> Result<Option<File>, DesktopQuotaError> {
        let mut dir = fs::DirBuilder::new();
        dir.recursive(true);
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
            dir.mode(0o700);
            options.mode(0o600);
        }
        dir.create(&self.dir)
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        let file = options
            .open(self.dir.join("operation.lock"))
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(fs::TryLockError::WouldBlock) => Ok(None),
            Err(fs::TryLockError::Error(_)) => Err(DesktopQuotaError::StorageUnavailable),
        }
    }
    fn load(&self) -> Result<Stored, DesktopQuotaError> {
        match fs::read(self.dir.join("state.json")) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|_| DesktopQuotaError::StorageUnavailable)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Stored::default()),
            Err(_) => Err(DesktopQuotaError::StorageUnavailable),
        }
    }
    fn save(&self, stored: &Stored) -> Result<(), DesktopQuotaError> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new_in(&self.dir)
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        serde_json::to_writer(&mut tmp, stored)
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        tmp.flush()
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        tmp.as_file()
            .sync_all()
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        tmp.persist(self.dir.join("state.json"))
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        Ok(())
    }
    pub fn state(&self) -> Result<DesktopQuotaState, DesktopQuotaError> {
        Ok(self.load()?.state)
    }
    pub fn link(&self) -> Result<DesktopQuotaState, DesktopQuotaError> {
        let _operation = self
            .try_begin()?
            .ok_or(DesktopQuotaError::SourceUnavailable)?;
        let mut stored = self.load()?;
        let session = self.source.open()?;
        let identity = session.identity()?;
        if identity.id.trim().is_empty() || identity.display_name.trim().is_empty() {
            return Err(DesktopQuotaError::SourceIncompatible);
        }
        if stored
            .identity
            .as_ref()
            .is_some_and(|id| id != &identity.id)
        {
            return Err(DesktopQuotaError::IdentityMismatch);
        }
        stored.identity = Some(identity.id);
        stored.state.account = Some(DesktopQuotaAccount {
            display_name: identity.display_name,
        });
        stored.state.error = None;
        self.save(&stored)?;
        Ok(stored.state)
    }
    pub fn refresh(&self) -> Result<DesktopQuotaRefresh, DesktopQuotaError> {
        let Some(_operation) = self.try_begin()? else {
            return Ok(DesktopQuotaRefresh {
                status: DesktopQuotaRefreshStatus::InProgress,
                state: self.state()?,
                retry_after_seconds: None,
            });
        };
        let mut stored = self.load()?;
        let id = stored
            .identity
            .clone()
            .ok_or(DesktopQuotaError::NotLinked)?;
        let now = (self.clock)();
        if let Some(last) = stored.last_attempt_epoch_ms {
            let remaining_ms = 60_000_i64.saturating_sub(now.saturating_sub(last));
            if remaining_ms > 0 {
                let remaining_seconds = (remaining_ms as u64).div_ceil(1000);
                return Ok(DesktopQuotaRefresh {
                    status: DesktopQuotaRefreshStatus::Wait,
                    state: stored.state,
                    retry_after_seconds: Some(u32::try_from(remaining_seconds).unwrap_or(u32::MAX)),
                });
            }
        }
        stored.last_attempt_epoch_ms = Some(now);
        stored.state.last_attempt_at = Some(timestamp(now)?);
        self.save(&stored)?;
        let result = (|| {
            let session = self.source.open()?;
            let identity = session.identity()?;
            if identity.id.is_empty() || identity.id != id {
                return Err(DesktopQuotaError::IdentityMismatch);
            }
            session.usage()
        })();
        let status = match result {
            Ok(windows) => {
                stored.state.last_success = Some(DesktopQuotaReading {
                    collected_at: timestamp((self.clock)())?,
                    five_hour: windows.five_hour,
                    seven_day: windows.seven_day,
                });
                stored.state.error = None;
                DesktopQuotaRefreshStatus::Updated
            }
            Err(error) => {
                stored.state.error = Some(error);
                DesktopQuotaRefreshStatus::Failed
            }
        };
        self.save(&stored)?;
        Ok(DesktopQuotaRefresh {
            status,
            state: stored.state,
            retry_after_seconds: None,
        })
    }
}

/// Persisted profile registry. The default slot keeps the original state location.
#[derive(Clone, Debug, Serialize, Type)]
pub struct DesktopQuotaProfile {
    pub id: String,
    pub is_default: bool,
    pub shares_default_quota: bool,
    pub state: DesktopQuotaState,
}

type SourceFactory = dyn Fn(&str) -> Arc<dyn DesktopQuotaSource> + Send + Sync;

pub struct DesktopQuotaProfiles {
    dir: PathBuf,
    source: Arc<SourceFactory>,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}
impl DesktopQuotaProfiles {
    pub fn new(
        dir: PathBuf,
        source: Arc<SourceFactory>,
        clock: Arc<dyn Fn() -> i64 + Send + Sync>,
    ) -> Self {
        Self { dir, source, clock }
    }
    fn registry_lock(&self) -> Result<File, DesktopQuotaError> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(self.dir.join("profiles"))
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(self.dir.join("profiles.lock"))
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        file.lock()
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        Ok(file)
    }
    fn valid_id(id: &str) -> bool {
        id.strip_prefix("p-")
            .is_some_and(|s| s.len() == 16 && s.bytes().all(|c| c.is_ascii_alphanumeric()))
    }
    fn dir_for(&self, id: &str) -> Result<PathBuf, DesktopQuotaError> {
        if id == "default" {
            return Ok(self.dir.clone());
        }
        if !Self::valid_id(id) {
            return Err(DesktopQuotaError::ProfileNotFound);
        }
        let dir = self.dir.join("profiles").join(id);
        match fs::symlink_metadata(&dir) {
            Ok(metadata) if metadata.is_dir() => Ok(dir),
            Ok(_) => Err(DesktopQuotaError::ProfileNotFound),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(DesktopQuotaError::ProfileNotFound)
            }
            Err(_) => Err(DesktopQuotaError::StorageUnavailable),
        }
    }
    pub fn profile_dir(&self, id: &str) -> Result<PathBuf, DesktopQuotaError> {
        self.dir_for(id)
    }
    /// Confirms that the profile has a persisted account identity before launching the Desktop.
    pub fn ensure_linked(&self, id: &str) -> Result<(), DesktopQuotaError> {
        let service = self.service(id)?;
        if service.load()?.identity.is_some() {
            Ok(())
        } else {
            Err(DesktopQuotaError::Unauthenticated)
        }
    }
    /// Account identity persisted for a linked profile, so callers can bind data to it.
    pub fn linked_identity(&self, id: &str) -> Result<DesktopAccountIdentity, DesktopQuotaError> {
        let stored = self.service(id)?.load()?;
        let identity = stored
            .identity
            .filter(|value| !value.trim().is_empty())
            .ok_or(DesktopQuotaError::Unauthenticated)?;
        Ok(DesktopAccountIdentity {
            display_name: stored
                .state
                .account
                .map(|account| account.display_name)
                .unwrap_or_else(|| identity.clone()),
            id: identity,
        })
    }
    fn service(&self, id: &str) -> Result<DesktopQuotaService, DesktopQuotaError> {
        Ok(DesktopQuotaService::new(
            self.dir_for(id)?,
            (self.source)(id),
            self.clock.clone(),
        ))
    }
    fn ids(&self) -> Result<Vec<String>, DesktopQuotaError> {
        let mut ids = vec!["default".to_owned()];
        let mut others = Vec::new();
        for entry in fs::read_dir(self.dir.join("profiles"))
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?
        {
            let entry = entry.map_err(|_| DesktopQuotaError::StorageUnavailable)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if Self::valid_id(&name)
                && entry
                    .file_type()
                    .map_err(|_| DesktopQuotaError::StorageUnavailable)?
                    .is_dir()
            {
                others.push(name);
            }
        }
        others.sort();
        ids.extend(others);
        Ok(ids)
    }
    pub fn list(&self) -> Result<Vec<DesktopQuotaProfile>, DesktopQuotaError> {
        let _guard = self.registry_lock()?;
        let default_identity = self
            .service("default")?
            .load()
            .ok()
            .and_then(|stored| stored.identity);
        Ok(self
            .ids()?
            .into_iter()
            .map(|id| {
                let shares_default_quota;
                let state = match self.service(&id).and_then(|service| service.load()) {
                    Ok(stored) => {
                        shares_default_quota = id != "default"
                            && default_identity.is_some()
                            && stored.identity == default_identity;
                        stored.state
                    }
                    Err(error) => {
                        shares_default_quota = false;
                        DesktopQuotaState {
                            error: Some(error),
                            ..DesktopQuotaState::default()
                        }
                    }
                };
                DesktopQuotaProfile {
                    is_default: id == "default",
                    shares_default_quota,
                    id,
                    state,
                }
            })
            .collect())
    }
    pub fn create(&self) -> Result<DesktopQuotaProfile, DesktopQuotaError> {
        let _guard = self.registry_lock()?;
        let mut builder = tempfile::Builder::new();
        builder.prefix("p-").rand_bytes(16);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(fs::Permissions::from_mode(0o700));
        }
        let dir = builder
            .tempdir_in(self.dir.join("profiles"))
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?
            .keep();
        let id = dir
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(DesktopQuotaError::StorageUnavailable)?
            .to_owned();
        Ok(DesktopQuotaProfile {
            id,
            is_default: false,
            shares_default_quota: false,
            state: DesktopQuotaState::default(),
        })
    }
    pub fn link(&self, id: &str) -> Result<DesktopQuotaProfile, DesktopQuotaError> {
        let service = self.service(id)?;
        let _operation = service
            .try_begin()?
            .ok_or(DesktopQuotaError::SourceUnavailable)?;
        let mut stored = service.load()?;
        let session = service.source.open()?;
        let identity = session.identity()?;
        if identity.id.trim().is_empty() || identity.display_name.trim().is_empty() {
            return Err(DesktopQuotaError::SourceIncompatible);
        }
        if stored
            .identity
            .as_ref()
            .is_some_and(|previous| previous != &identity.id)
        {
            return Err(DesktopQuotaError::IdentityMismatch);
        }
        let _guard = self.registry_lock()?;
        let mut shares_default_quota = false;
        for other in self.ids()? {
            if other != id && self.service(&other)?.load()?.identity.as_ref() == Some(&identity.id)
            {
                if id != "default" && other != "default" {
                    return Err(DesktopQuotaError::DuplicateAccount);
                }
                shares_default_quota = other == "default";
            }
        }
        stored.identity = Some(identity.id);
        stored.state.account = Some(DesktopQuotaAccount {
            display_name: identity.display_name,
        });
        stored.state.error = None;
        service.save(&stored)?;
        Ok(DesktopQuotaProfile {
            id: id.to_owned(),
            is_default: id == "default",
            shares_default_quota,
            state: stored.state,
        })
    }
    pub fn refresh(&self, id: &str) -> Result<DesktopQuotaRefresh, DesktopQuotaError> {
        let service = self.service(id)?;
        if id != "default" {
            let identity = service.load()?.identity;
            let default = self.service("default")?;
            if identity.is_some()
                && default.load().ok().and_then(|stored| stored.identity) == identity
            {
                return default.refresh();
            }
        }
        service.refresh()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

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
                id: "acct-1".into(),
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

    fn profiles() -> DesktopQuotaProfiles {
        DesktopQuotaProfiles::new(
            tempfile::tempdir().unwrap().keep(),
            Arc::new(|_: &str| Arc::new(Source)),
            Arc::new(|| 1_000_000),
        )
    }

    #[test]
    fn linked_identity_returns_the_persisted_identity_of_a_linked_profile() {
        let profiles = profiles();
        let draft = profiles.create().unwrap();
        profiles.link(&draft.id).unwrap();

        let identity = profiles.linked_identity(&draft.id).unwrap();
        assert_eq!(identity.id, "acct-1");
        assert_eq!(identity.display_name, "Alice");
    }

    #[test]
    fn linked_identity_is_unauthenticated_for_a_profile_without_identity() {
        let profiles = profiles();
        let draft = profiles.create().unwrap();

        assert!(matches!(
            profiles.linked_identity(&draft.id),
            Err(DesktopQuotaError::Unauthenticated)
        ));
        // Default slot with no state yet behaves the same.
        assert!(matches!(
            profiles.linked_identity("default"),
            Err(DesktopQuotaError::Unauthenticated)
        ));
        // A blank identity on disk is treated as absent.
        let dir = profiles.profile_dir(&draft.id).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("state.json"),
            r#"{"identity":"   ","state":{"account":null,"last_attempt_at":null,"last_success":null,"error":null},"last_attempt_epoch_ms":null}"#,
        )
        .unwrap();
        assert!(matches!(
            profiles.linked_identity(&draft.id),
            Err(DesktopQuotaError::Unauthenticated)
        ));
    }

    #[test]
    fn linked_identity_reports_a_missing_profile_without_panicking() {
        let profiles = profiles();
        assert!(matches!(
            profiles.linked_identity("p-doesnotexist"),
            Err(DesktopQuotaError::ProfileNotFound)
        ));
        assert!(matches!(
            profiles.linked_identity("../state"),
            Err(DesktopQuotaError::ProfileNotFound)
        ));
    }
}
