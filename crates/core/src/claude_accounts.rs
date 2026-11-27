//! Local Claude account metadata and isolated official CLI authentication.
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

pub const STORE_NAME: &str = "claude-accounts";

/// Protect both normal paths and aliases from the editor and sync.
pub fn is_private_path(store: &Path, path: &Path) -> bool {
    path.starts_with(store)
        || path.canonicalize().is_ok_and(|path| {
            path.starts_with(store)
                || store
                    .canonicalize()
                    .is_ok_and(|root| path.starts_with(root))
        })
}

#[derive(Clone, Debug, Deserialize, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeAccount {
    pub id: String,
    pub display_name: String,
}

#[derive(Clone, Copy, Debug, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AccountState {
    Pending,
    Connected,
    Disconnected,
    VerificationFailed,
}

#[derive(Clone, Copy, Debug, Serialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AccountError {
    InvalidDisplayName,
    ProfileNotFound,
    StorageUnavailable,
    CliUnavailable,
    StatusTimeout,
    StatusInvalid,
    TerminalUnavailable,
    LoginFailed,
}

#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    pub profile_id: String,
    pub state: AccountState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<AccountError>,
}

impl AccountStatus {
    fn state(id: &str, state: AccountState) -> Self {
        Self {
            profile_id: id.into(),
            state,
            error_code: None,
        }
    }

    fn checked(id: &str, outcome: Result<AccountState, AccountError>) -> Self {
        match outcome {
            Ok(state) => Self::state(id, state),
            Err(error) => Self {
                profile_id: id.into(),
                state: AccountState::VerificationFailed,
                error_code: Some(error),
            },
        }
    }
}

// Session variables only; no shell, credential, proxy, or Claude overrides.
const GUI_ENV: &[&str] = &[
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "XAUTHORITY",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_RUNTIME_DIR",
    "XDG_CURRENT_DESKTOP",
    "XDG_SESSION_TYPE",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
];
const SYSTEM_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

pub struct ClaudePrograms {
    cli: Option<PathBuf>,
    terminal: PathBuf,
    timeout: Duration,
    environment: Vec<(OsString, OsString)>,
    terminal_home: Option<PathBuf>,
}

impl ClaudePrograms {
    pub fn local(home: PathBuf) -> Self {
        Self {
            cli: None,
            terminal: "/usr/bin/gnome-terminal".into(),
            timeout: Duration::from_secs(5),
            environment: GUI_ENV
                .iter()
                .filter_map(|&key| std::env::var_os(key).map(|value| (key.into(), value)))
                .collect(),
            terminal_home: Some(home),
        }
    }

    /// Inject executables and environment for process-boundary tests.
    pub fn new(
        cli: PathBuf,
        terminal: PathBuf,
        timeout: Duration,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Self {
        Self {
            cli: Some(cli),
            terminal,
            timeout,
            environment: environment.into_iter().collect(),
            terminal_home: None,
        }
    }

    fn cli(&self) -> Result<PathBuf, AccountError> {
        use std::os::unix::fs::PermissionsExt;
        let executable = |path: &Path| {
            path.is_absolute()
                && fs::metadata(path)
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        };
        if let Some(path) = &self.cli {
            return executable(path)
                .then(|| path.clone())
                .ok_or(AccountError::CliUnavailable);
        }
        let path = std::env::var_os("PATH").unwrap_or_else(|| SYSTEM_PATH.into());
        std::env::split_paths(&path)
            .chain(
                self.terminal_home
                    .iter()
                    .map(|home| home.join(".local/bin")),
            )
            .map(|dir| dir.join("claude"))
            .find(|path| executable(path))
            .ok_or(AccountError::CliUnavailable)
    }

    fn gui_env(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        self.environment
            .iter()
            .filter(|(key, _)| GUI_ENV.iter().any(|allowed| key == *allowed))
            .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
    }

    fn cli_env<'a>(
        &'a self,
        profile: &'a ProfilePaths,
    ) -> impl Iterator<Item = (&'a OsStr, &'a OsStr)> {
        [
            (OsStr::new("HOME"), profile.home.as_os_str()),
            (OsStr::new("CLAUDE_CONFIG_DIR"), profile.config.as_os_str()),
            (OsStr::new("PATH"), OsStr::new(SYSTEM_PATH)),
            (OsStr::new("TERM"), OsStr::new("xterm-256color")),
        ]
        .into_iter()
        .chain(self.gui_env())
    }
}

pub struct ClaudeAccounts {
    store: PathBuf,
    programs: ClaudePrograms,
    registry: Mutex<()>,
    busy: Mutex<HashSet<String>>,
}

struct Exclusive<'a> {
    busy: &'a Mutex<HashSet<String>>,
    id: &'a str,
}
impl Drop for Exclusive<'_> {
    fn drop(&mut self) {
        if let Ok(mut busy) = self.busy.lock() {
            busy.remove(self.id);
        }
    }
}

struct ProfilePaths {
    home: PathBuf,
    config: PathBuf,
}

impl ClaudeAccounts {
    pub fn new(store: PathBuf, programs: ClaudePrograms) -> Self {
        Self {
            store,
            programs,
            registry: Mutex::new(()),
            busy: Mutex::new(HashSet::new()),
        }
    }

    fn registry(&self) -> Result<MutexGuard<'_, ()>, AccountError> {
        self.registry
            .lock()
            .map_err(|_| AccountError::StorageUnavailable)
    }

    pub fn list(&self) -> Result<Vec<ClaudeAccount>, AccountError> {
        let _lock = self.registry()?;
        if !store_exists(&self.store)? {
            return Ok(Vec::new());
        }
        let mut accounts = Vec::new();
        for entry in fs::read_dir(&self.store).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            if let Some(id) = entry.file_name().to_str().filter(|id| valid_id(id)) {
                match self.read_profile(id) {
                    Ok(account) => accounts.push(account),
                    // An interrupted create with no metadata is not a registered account.
                    Err(AccountError::ProfileNotFound) => (),
                    Err(error) => return Err(error),
                }
            }
        }
        accounts.sort_by(|a, b| a.display_name.cmp(&b.display_name).then(a.id.cmp(&b.id)));
        Ok(accounts)
    }

    pub fn create(&self, display_name: &str) -> Result<ClaudeAccount, AccountError> {
        use std::{io::Write, os::unix::fs::PermissionsExt};
        let display_name = display_name.trim();
        if !valid_name(display_name) {
            return Err(AccountError::InvalidDisplayName);
        }
        let _lock = self.registry()?;
        if !store_exists(&self.store)? {
            let parent = self
                .store
                .parent()
                .ok_or(AccountError::StorageUnavailable)?;
            create_ancestors(parent)?;
            create_private_dir(&self.store)?;
        }
        let dir = tempfile::Builder::new()
            .prefix("profile-")
            .rand_bytes(20)
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir_in(&self.store)
            .map_err(storage)?;
        let id = dir
            .path()
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(AccountError::StorageUnavailable)?;
        let account = ClaudeAccount {
            id: id.into(),
            display_name: display_name.into(),
        };
        create_private_dir(&dir.path().join("config"))?;
        create_private_dir(&dir.path().join("home"))?;
        let mut metadata = tempfile::NamedTempFile::new_in(dir.path()).map_err(storage)?;
        serde_json::to_writer(&mut metadata, &account).map_err(storage)?;
        metadata.flush().map_err(storage)?;
        metadata.as_file().sync_all().map_err(storage)?;
        metadata
            .persist_noclobber(dir.path().join("profile.json"))
            .map_err(storage)?;
        let _ = dir.keep();
        Ok(account)
    }

    fn read_profile(&self, id: &str) -> Result<ClaudeAccount, AccountError> {
        if !valid_id(id) || !store_exists(&self.store)? {
            return Err(AccountError::ProfileNotFound);
        }
        let dir = self.store.join(id);
        match fs::symlink_metadata(&dir) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AccountError::ProfileNotFound);
            }
            _ => check_private_dir(&dir)?,
        }
        let path = dir.join("profile.json");
        let metadata = match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AccountError::ProfileNotFound);
            }
            result => result.map_err(storage)?,
        };
        if !single_file(&metadata) || metadata.len() > 2048 {
            return Err(AccountError::StorageUnavailable);
        }
        let account: ClaudeAccount =
            serde_json::from_slice(&fs::read(path).map_err(storage)?).map_err(storage)?;
        if account.id != id || !valid_name(&account.display_name) {
            return Err(AccountError::StorageUnavailable);
        }
        Ok(account)
    }

    fn profile_paths(&self, id: &str) -> Result<ProfilePaths, AccountError> {
        let profile = self.store.join(id);
        check_private_dir(&self.store)?;
        check_private_dir(&profile)?;
        let paths = ProfilePaths {
            home: profile.join("home"),
            config: profile.join("config"),
        };
        for dir in [&paths.home, &paths.config] {
            check_private_dir(dir)?;
            check_tree(dir)?;
        }
        Ok(paths)
    }

    fn begin<'a>(&'a self, id: &'a str) -> Result<Option<Exclusive<'a>>, AccountError> {
        // Both status and login serialize per profile; the guard spans final confirmation.
        let mut busy = self.busy.lock().map_err(storage)?;
        if !busy.insert(id.into()) {
            return Ok(None);
        }
        Ok(Some(Exclusive {
            busy: &self.busy,
            id,
        }))
    }

    pub fn status(&self, id: &str) -> Result<AccountStatus, AccountError> {
        let Some(_guard) = self.enter(id)? else {
            return Ok(AccountStatus::state(id, AccountState::Pending));
        };
        let result = self.profile_paths(id).and_then(|paths| {
            self.programs
                .cli()
                .and_then(|cli| self.programs.status(&cli, &paths))
        });
        Ok(AccountStatus::checked(id, result))
    }

    pub fn start_login(&self, id: &str) -> Result<AccountStatus, AccountError> {
        let Some(_guard) = self.enter(id)? else {
            return Ok(AccountStatus::state(id, AccountState::Pending));
        };
        let result = (|| {
            let paths = self.profile_paths(id)?;
            let cli = self.programs.cli()?;
            // An older CLI might ignore CLAUDE_CONFIG_DIR. Require positive isolation
            // evidence before launching even if the profile has never logged in.
            self.programs.status(&cli, &paths)?;
            let clean = self.programs.login(&cli, &self.profile_paths(id)?)?;
            let state = self.programs.status(&cli, &self.profile_paths(id)?)?;
            if !clean && state == AccountState::Connected {
                return Err(AccountError::LoginFailed);
            }
            Ok(state)
        })();
        Ok(AccountStatus::checked(id, result))
    }

    fn enter<'a>(&'a self, id: &'a str) -> Result<Option<Exclusive<'a>>, AccountError> {
        let _lock = self.registry()?;
        self.read_profile(id)?;
        self.begin(id)
    }
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= 100 && !name.chars().any(char::is_control)
}

fn valid_id(id: &str) -> bool {
    id.strip_prefix("profile-").is_some_and(|suffix| {
        suffix.len() == 20 && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}

fn storage<T>(_: T) -> AccountError {
    AccountError::StorageUnavailable
}

fn store_exists(path: &Path) -> Result<bool, AccountError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        _ => check_private_dir(path).map(|()| true),
    }
}

fn check_private_dir(path: &Path) -> Result<(), AccountError> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let meta = fs::symlink_metadata(path).map_err(storage)?;
    if !meta.is_dir()
        || meta.uid() != nix::unistd::geteuid().as_raw()
        || meta.permissions().mode() & 0o7777 != 0o700
        || path.canonicalize().map_err(storage)? != path
    {
        return Err(AccountError::StorageUnavailable);
    }
    Ok(())
}

fn create_private_dir(path: &Path) -> Result<(), AccountError> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(storage)?;
    check_private_dir(path)
}

fn create_ancestors(path: &Path) -> Result<(), AccountError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && path.canonicalize().map_err(storage)? == path => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            create_ancestors(path.parent().ok_or(AccountError::StorageUnavailable)?)?;
            create_private_dir(path)
        }
        _ => Err(AccountError::StorageUnavailable),
    }
}

fn single_file(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.is_file() && metadata.nlink() == 1
}

// Never read CLI files. Reject links and special files so the CLI cannot follow
// them into the default user's configuration or another profile.
fn check_tree(root: &Path) -> Result<(), AccountError> {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir).map_err(storage)? {
            let path = entry.map_err(storage)?.path();
            let meta = fs::symlink_metadata(&path).map_err(storage)?;
            if meta.is_dir() {
                dirs.push(path);
            } else if !single_file(&meta) {
                return Err(AccountError::StorageUnavailable);
            }
        }
    }
    Ok(())
}

mod process;
