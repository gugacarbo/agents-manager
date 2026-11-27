use super::{AccountError, AccountState, ClaudePrograms, ProfilePaths};
use nix::{
    fcntl::{FcntlArg, OFlag, fcntl},
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use serde::Deserialize;
use std::{
    ffi::OsString,
    fs,
    io::{self, Read},
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const STATUS_LIMIT: usize = 16 * 1024;
const LOGIN_COMMAND: &str = r#"
completed=$1
shift
"$@" auth login && : > "$completed"
"#;

// Only the opener gets the original desktop HOME/XDG paths. Claude Code
// invokes BROWSER with its URL; forward it without reading or logging it.
const BROWSER_OPENER: &str = r#"#!/bin/sh
export HOME="$DESKTOP_HOME"
export XDG_CONFIG_HOME="${DESKTOP_XDG_CONFIG_HOME:-$HOME/.config}"
export XDG_DATA_HOME="${DESKTOP_XDG_DATA_HOME:-$HOME/.local/share}"
export XDG_CONFIG_DIRS="${DESKTOP_XDG_CONFIG_DIRS:-/etc/xdg}"
export XDG_DATA_DIRS="${DESKTOP_XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
unset BROWSER CLAUDE_CONFIG_DIR DESKTOP_HOME DESKTOP_XDG_CONFIG_HOME DESKTOP_XDG_CONFIG_DIRS DESKTOP_XDG_DATA_HOME DESKTOP_XDG_DATA_DIRS
exec /usr/bin/xdg-open "$@"
"#;

fn assignment(mut key: OsString, value: &std::ffi::OsStr) -> OsString {
    key.push("=");
    key.push(value);
    key
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CliStatus {
    config_directory: PathBuf,
    logged_in: bool,
    auth_method: AuthMethod,
}

#[derive(Deserialize)]
enum AuthMethod {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "claude.ai")]
    StoredAccount,
}

impl ClaudePrograms {
    pub(super) fn status(
        &self,
        cli: &Path,
        paths: &ProfilePaths,
    ) -> Result<AccountState, AccountError> {
        let mut child = Command::new(cli)
            .args(["auth", "status"])
            .current_dir("/")
            .env_clear()
            .envs(self.cli_env(paths))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map_err(|_| AccountError::CliUnavailable)?;
        let (bytes, exit) = match bounded_status(&mut child, self.timeout) {
            Ok(result) => result,
            Err(error) => {
                if let Ok(pid) = i32::try_from(child.id()) {
                    let _ = killpg(Pid::from_raw(pid), Signal::SIGKILL);
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let status: CliStatus =
            serde_json::from_slice(&bytes).map_err(|_| AccountError::StatusInvalid)?;
        // Compare the exact reported directory, not just a symlink alias to it.
        if status.config_directory.as_os_str() != paths.config.as_os_str() {
            return Err(AccountError::StatusInvalid);
        }
        match (exit, status.logged_in, status.auth_method) {
            (Some(0), true, AuthMethod::StoredAccount) => Ok(AccountState::Connected),
            (Some(1), false, AuthMethod::None) => Ok(AccountState::Disconnected),
            _ => Err(AccountError::StatusInvalid),
        }
    }

    pub(super) fn login(&self, cli: &Path, paths: &ProfilePaths) -> Result<bool, AccountError> {
        // Ghostty waits for the CLI, but returns its own exit status. This private
        // marker contains no CLI output and is written only on a successful login.
        let launch = tempfile::tempdir_in(&paths.home).map_err(super::storage)?;
        let completed = launch.path().join("completed");
        let mut assignments: Vec<_> = self
            .cli_env(paths)
            .map(|(key, value)| assignment(key.into(), value))
            .collect();
        if let Some(home) = &self.terminal_home {
            let opener = launch.path().join("open-browser");
            fs::write(&opener, BROWSER_OPENER).map_err(super::storage)?;
            fs::set_permissions(&opener, fs::Permissions::from_mode(0o700))
                .map_err(super::storage)?;
            assignments.push(assignment("BROWSER".into(), opener.as_os_str()));
            assignments.push(assignment("DESKTOP_HOME".into(), home.as_os_str()));
            for (key, value) in self.browser_xdg_env() {
                let mut name = OsString::from("DESKTOP_");
                name.push(key);
                assignments.push(assignment(name, value));
            }
        }
        for (terminal, args) in &self.terminals {
            let mut command = Command::new(terminal);
            command
                .args(*args)
                .args(["/usr/bin/env", "-i"])
                .args(&assignments)
                .args(["/bin/sh", "-c", LOGIN_COMMAND, "claude-account-login"])
                .arg(&completed)
                .arg(cli)
                .current_dir("/")
                .env_clear()
                .env("PATH", super::SYSTEM_PATH)
                .envs(self.gui_env())
                .envs(self.browser_xdg_env())
                // Local emulator diagnostics aren't the terminal's interactive streams.
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if let Some(home) = &self.terminal_home {
                command.env("HOME", home);
            }
            match command.spawn() {
                // Never try another emulator after one has started: its window may
                // already have launched the CLI, even if the emulator exits nonzero.
                Ok(mut child) => {
                    child
                        .wait()
                        .map_err(|_| AccountError::TerminalUnavailable)?;
                    return Ok(completed.is_file());
                }
                Err(_) => continue,
            }
        }
        Err(AccountError::TerminalUnavailable)
    }
}

fn bounded_status(
    child: &mut Child,
    timeout: Duration,
) -> Result<(Vec<u8>, Option<i32>), AccountError> {
    let invalid = |_| AccountError::StatusInvalid;
    let mut stdout = child.stdout.take().ok_or(AccountError::StatusInvalid)?;
    fcntl(&stdout, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).map_err(invalid)?;
    let start = Instant::now();
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        if start.elapsed() >= timeout {
            return Err(AccountError::StatusTimeout);
        }
        match stdout.read(&mut buffer) {
            Ok(0) => {
                if let Some(status) = child.try_wait().map_err(|_| AccountError::StatusInvalid)? {
                    return Ok((bytes, status.code()));
                }
            }
            Ok(size) if bytes.len() + size <= STATUS_LIMIT => {
                bytes.extend_from_slice(&buffer[..size]);
                continue;
            }
            Ok(_) => return Err(AccountError::StatusInvalid),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => (),
            Err(_) => return Err(AccountError::StatusInvalid),
        }
        thread::sleep(Duration::from_millis(10));
    }
}
