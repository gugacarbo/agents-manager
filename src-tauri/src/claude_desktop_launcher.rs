use agents_manager_claude_accounts::desktop_quotas::{DesktopQuotaError, source::ensure_closed};
use std::{
    fs::{File, OpenOptions, TryLockError},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::Path,
    process::{Command, Stdio},
};

const XDG_DIRS: [(&str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config"),
    ("XDG_DATA_HOME", "data"),
    ("XDG_CACHE_HOME", "cache"),
    ("XDG_STATE_HOME", "state"),
];
const CLAUDE_SCHEME: &str = "x-scheme-handler/claude";

fn callback_entry(xdg: &Path) -> String {
    // Portal app choosers need a visible handler for selecting the isolated profile.
    let mut entry = format!(
        "[Desktop Entry]\nType=Application\nName=Claude Desktop sign-in (Agents Manager)\nNoDisplay=false\nTryExec=/proc/{}/exe\nMimeType=x-scheme-handler/claude;\nExec=/usr/bin/env -u CLAUDE_USER_DATA_DIR",
        std::process::id(),
    );
    for (var, dir) in XDG_DIRS {
        let value = xdg.join(dir);
        let mut escaped = String::new();
        for char in value.to_string_lossy().chars() {
            match char {
                '\\' => escaped.push_str(r"\\\\"),
                '"' | '$' | '`' => {
                    escaped.push_str(r"\\");
                    escaped.push(char);
                }
                '%' => escaped.push_str("%%"),
                '\n' => escaped.push_str(r"\n"),
                '\r' => escaped.push_str(r"\r"),
                _ => escaped.push(char),
            }
        }
        entry.push_str(&format!(" \"{var}={escaped}\""));
    }
    entry.push_str(" claude-desktop %u\n");
    entry
}

fn default_callback() -> Result<String, DesktopQuotaError> {
    let output = Command::new("xdg-mime")
        .args(["query", "default", CLAUDE_SCHEME])
        .output()
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || id.is_empty() {
        return Err(DesktopQuotaError::SourceUnavailable);
    }
    Ok(id)
}

fn set_callback(id: &str) -> Result<(), DesktopQuotaError> {
    let status = Command::new("xdg-mime")
        .args(["default", id, CLAUDE_SCHEME])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
    status
        .success()
        .then_some(())
        .ok_or(DesktopQuotaError::SourceUnavailable)
}

struct LoginCallback {
    _entry: tempfile::NamedTempFile,
    id: String,
    previous: String,
}

impl LoginCallback {
    fn register(xdg: &Path) -> Result<Self, DesktopQuotaError> {
        let previous = default_callback()?;
        let applications = directories::BaseDirs::new()
            .ok_or(DesktopQuotaError::StorageUnavailable)?
            .data_dir()
            .join("applications");
        std::fs::create_dir_all(&applications)
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        let mut entry = tempfile::Builder::new()
            .prefix("agents-manager-claude-login-")
            .suffix(".desktop")
            .tempfile_in(applications)
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        entry
            .write_all(callback_entry(xdg).as_bytes())
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        let id = entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(DesktopQuotaError::StorageUnavailable)?
            .to_owned();
        let callback = Self {
            _entry: entry,
            id,
            previous,
        };
        set_callback(&callback.id)?;
        if default_callback()? != callback.id {
            return Err(DesktopQuotaError::SourceUnavailable);
        }
        Ok(callback)
    }
}

impl Drop for LoginCallback {
    fn drop(&mut self) {
        if default_callback().as_deref() == Ok(self.id.as_str()) {
            let _ = set_callback(&self.previous);
        }
    }
}

pub(crate) fn login_lock_at(path: &Path) -> Result<File, DesktopQuotaError> {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    lock.try_lock().map_err(|error| match error {
        TryLockError::WouldBlock => DesktopQuotaError::DesktopRunning,
        TryLockError::Error(_) => DesktopQuotaError::StorageUnavailable,
    })?;
    Ok(lock)
}

fn path_metadata(
    path: &Path,
    follow_symlinks: bool,
) -> Result<std::fs::Metadata, DesktopQuotaError> {
    let result = if follow_symlinks {
        std::fs::metadata(path)
    } else {
        std::fs::symlink_metadata(path)
    };
    result.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            DesktopQuotaError::Unauthenticated
        } else if error.kind() == std::io::ErrorKind::PermissionDenied {
            DesktopQuotaError::StorageUnavailable
        } else {
            DesktopQuotaError::SourceUnavailable
        }
    })
}

fn validate_profile_config(isolated_xdg: Option<&Path>) -> Result<(), DesktopQuotaError> {
    let config = if let Some(xdg) = isolated_xdg {
        let config_dir = xdg.join("config");
        let claude_dir = config_dir.join("Claude");
        for directory in [xdg, config_dir.as_path(), claude_dir.as_path()] {
            let metadata = path_metadata(directory, false)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(DesktopQuotaError::SourceUnavailable);
            }
        }
        let config = claude_dir.join("config.json");
        let metadata = path_metadata(&config, false)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(DesktopQuotaError::SourceUnavailable);
        }
        return Ok(());
    } else {
        let base = directories::BaseDirs::new().ok_or(DesktopQuotaError::StorageUnavailable)?;
        base.config_dir().join("Claude/config.json")
    };
    let metadata = path_metadata(&config, true)?;
    metadata
        .is_file()
        .then_some(())
        .ok_or(DesktopQuotaError::SourceUnavailable)
}

fn launch_command(program: impl AsRef<std::ffi::OsStr>, xdg: Option<&Path>) -> Command {
    let mut command = Command::new(program);
    command.env_remove("CLAUDE_USER_DATA_DIR");
    if let Some(xdg) = xdg {
        for (var, name) in XDG_DIRS {
            command.env(var, xdg.join(name));
        }
    }
    command
}

fn launch_profile(
    command: &mut Command,
    lock: File,
    callback: Option<LoginCallback>,
) -> Result<(), DesktopQuotaError> {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
        drop(callback);
        drop(lock);
    });
    Ok(())
}

/// Initializes an isolated XDG profile directory and spawns `claude-desktop` for sign-in.
pub(crate) fn launch_login(store_dir: &Path, profile_dir: &Path) -> Result<(), DesktopQuotaError> {
    let lock = login_lock_at(&store_dir.join("login.lock"))?;
    ensure_closed()?;
    let xdg = profile_dir.join("xdg");
    for (_, name) in XDG_DIRS {
        let path = xdg.join(name);
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&path)
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    }
    // The browser starts a separate process for claude:// and does not inherit this environment.
    let callback = LoginCallback::register(&xdg)?;
    let mut command = launch_command("claude-desktop", Some(&xdg));
    launch_profile(&mut command, lock, Some(callback))
}

/// Validates and spawns `claude-desktop` for a linked quota profile (or default profile when `xdg` is `None`).
pub(crate) fn launch_authenticated_profile(
    store_dir: &Path,
    xdg: Option<&Path>,
) -> Result<(), DesktopQuotaError> {
    validate_profile_config(xdg)?;
    let lock = login_lock_at(&store_dir.join("login.lock"))?;
    ensure_closed()?;
    let callback = xdg.map(LoginCallback::register).transpose()?;
    let mut command = launch_command("claude-desktop", xdg);
    launch_profile(&mut command, lock, callback)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn callback_uses_the_same_xdg_profile_as_the_login() {
        let path = std::path::Path::new("/tmp/managed profile %u/xdg");
        let entry = callback_entry(path);
        assert!(entry.contains("NoDisplay=false\n"));
        assert!(entry.contains(
            "Exec=/usr/bin/env -u CLAUDE_USER_DATA_DIR \"XDG_CONFIG_HOME=/tmp/managed profile %%u/xdg/config\""
        ));
        for directory in ["data", "cache", "state"] {
            assert!(entry.contains(&format!("%%u/xdg/{directory}\"")));
        }
        assert!(entry.ends_with(" claude-desktop %u\n"));
    }

    #[test]
    fn missing_or_redirected_private_config_is_not_opened_as_a_new_profile() {
        let temp = tempfile::tempdir().expect("temporary test directory");
        let profile = temp.path().join("profile");
        let xdg = profile.join("xdg");
        let xdg_config = xdg.join("config");
        std::fs::create_dir_all(xdg_config.join("Claude")).expect("create private config root");
        assert_eq!(
            validate_profile_config(Some(&xdg)),
            Err(DesktopQuotaError::Unauthenticated)
        );

        std::fs::write(xdg_config.join("Claude/config.json"), "not parsed here")
            .expect("create session config");
        assert_eq!(validate_profile_config(Some(&xdg)), Ok(()));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let external = temp.path().join("external");
            std::fs::create_dir_all(external.join("Claude")).expect("create external config");
            std::fs::write(external.join("Claude/config.json"), "opaque session bytes")
                .expect("create external session config");
            std::fs::remove_dir_all(profile.join("xdg/config")).expect("remove private config");
            symlink(&external, profile.join("xdg/config")).expect("redirect private config");
            assert_eq!(
                validate_profile_config(Some(&xdg)),
                Err(DesktopQuotaError::SourceUnavailable)
            );
        }
    }

    #[test]
    fn profile_launch_uses_private_xdg_roots_and_default_inherits_defaults() {
        let xdg = Path::new("/tmp/managed-profile/xdg");
        let isolated = launch_command("claude-desktop", Some(xdg));
        for (var, directory) in XDG_DIRS {
            let value = isolated
                .get_envs()
                .find(|(name, _)| *name == var)
                .and_then(|(_, value)| value)
                .expect("isolated profile must set every XDG directory");
            assert_eq!(value, xdg.join(directory).as_os_str());
        }
        assert!(
            isolated
                .get_envs()
                .any(|(name, value)| { name == "CLAUDE_USER_DATA_DIR" && value.is_none() })
        );

        let default = launch_command("claude-desktop", None);
        for (var, _) in XDG_DIRS {
            assert!(!default.get_envs().any(|(name, _)| name == var));
        }
        assert!(
            default
                .get_envs()
                .any(|(name, value)| { name == "CLAUDE_USER_DATA_DIR" && value.is_none() })
        );
    }

    #[test]
    fn profile_launch_lock_blocks_concurrent_open_and_releases_for_retry() {
        let temp = tempfile::tempdir().expect("temporary test directory");
        let lock_path = temp.path().join("login.lock");
        let held = login_lock_at(&lock_path).expect("first launch acquires lock");
        assert_eq!(
            login_lock_at(&lock_path).err(),
            Some(DesktopQuotaError::DesktopRunning)
        );
        drop(held);

        let missing = temp.path().join("missing-desktop");
        let lock = login_lock_at(&lock_path).expect("retry acquires released lock");
        assert_eq!(
            launch_profile(&mut Command::new(missing), lock, None),
            Err(DesktopQuotaError::SourceUnavailable)
        );
        let lock = login_lock_at(&lock_path).expect("failed launch releases lock");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let program = temp.path().join("desktop-stand-in");
            std::fs::write(&program, "#!/bin/sh\nexit 0\n").expect("write stand-in");
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
                .expect("make stand-in executable");
            let mut command = Command::new(program);
            launch_profile(&mut command, lock, None).expect("stand-in starts");
            let mut released = false;
            for _ in 0..100 {
                if let Ok(lock) = login_lock_at(&lock_path) {
                    drop(lock);
                    released = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(released, "child exit must release the launch lock");
        }
    }
}
