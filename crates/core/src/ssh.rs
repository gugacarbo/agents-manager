use crate::{
    domain::{SshConnection, SshEnvironment, SshEnvironmentDraft, SshErrorKind, SshInventory},
    error::CoreError,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
fn error(kind: SshErrorKind, message: &str) -> CoreError {
    CoreError::SshError {
        kind,
        message: message.into(),
    }
}
fn valid(value: &str, host: bool) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('-')
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(b, b'.' | b'_' | b'-')
                || (host && matches!(b, b':' | b'[' | b']' | b'%'))
        })
}
pub fn validate(draft: &SshEnvironmentDraft) -> Result<(), CoreError> {
    if draft.name.trim().is_empty()
        || draft.name.len() > 255
        || draft.name.chars().any(char::is_control)
    {
        return Err(error(
            SshErrorKind::InvalidInput,
            "Enter a valid environment name.",
        ));
    }
    let valid_connection = match &draft.connection {
        SshConnection::Alias { alias } => valid(alias, false),
        SshConnection::Manual { host, user, port } => {
            valid(host, true) && valid(user, false) && *port > 0
        }
    };
    if !valid_connection {
        return Err(error(
            SshErrorKind::InvalidInput,
            "Enter a valid SSH destination, user and port.",
        ));
    }
    Ok(())
}
pub fn list_environments(registry: &Path) -> Result<Vec<SshEnvironment>, CoreError> {
    match fs::read(registry) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.into()),
    }
}
fn store(registry: &Path, items: &[SshEnvironment]) -> Result<(), CoreError> {
    let parent = registry
        .parent()
        .ok_or_else(|| error(SshErrorKind::InvalidInput, "Invalid registry path."))?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&serde_json::to_vec_pretty(items)?)?;
    temp.as_file().sync_all()?;
    temp.persist(registry)
        .map_err(|e| CoreError::from(e.error))?;
    Ok(())
}
pub fn save_environment(
    registry: &Path,
    id: Option<&str>,
    draft: &SshEnvironmentDraft,
) -> Result<SshEnvironment, CoreError> {
    validate(draft)?;
    let mut items = list_environments(registry)?;
    let id = if let Some(id) = id {
        if !items.iter().any(|e| e.id == id) {
            return Err(CoreError::NotFound { target: id.into() });
        }
        id.to_owned()
    } else {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| error(SshErrorKind::Protocol, "System clock unavailable."))?
            .as_nanos();
        format!(
            "{:x}",
            Sha256::digest(format!("{nanos}-{}", draft.name).as_bytes())
        )
    };
    let env = SshEnvironment {
        id,
        name: draft.name.clone(),
        connection: draft.connection.clone(),
    };
    items.retain(|item| item.id != env.id);
    items.push(env.clone());
    store(registry, &items)?;
    Ok(env)
}
pub fn remove_environment(registry: &Path, id: &str) -> Result<(), CoreError> {
    let mut items = list_environments(registry)?;
    let before = items.len();
    items.retain(|item| item.id != id);
    if before == items.len() {
        return Err(CoreError::NotFound { target: id.into() });
    }
    store(registry, &items)
}
pub(crate) fn terminate(child: &mut std::process::Child) {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(child.id()) {
        let _ = nix::sys::signal::killpg(
            nix::unistd::Pid::from_raw(pid),
            nix::sys::signal::Signal::SIGKILL,
        );
    }
    let _ = child.kill();
    let _ = child.wait();
}
#[derive(Debug, Clone)]
pub struct SshTransport {
    program: PathBuf,
    config: Option<PathBuf>,
    timeout: Duration,
}
impl Default for SshTransport {
    fn default() -> Self {
        Self {
            program: PathBuf::from("ssh"),
            config: None,
            timeout: Duration::from_secs(15),
        }
    }
}
impl SshTransport {
    pub fn new(program: PathBuf, config: Option<PathBuf>, timeout: Duration) -> Self {
        Self {
            program,
            config,
            timeout,
        }
    }
    fn run(&self, draft: &SshEnvironmentDraft, config_only: bool) -> Result<String, CoreError> {
        validate(draft)?;
        let mut command = Command::new(&self.program);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(config) = &self.config {
            command.arg("-F").arg(config);
        }
        if config_only {
            command.arg("-G");
        } else {
            command.args([
                "-T",
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                "-o",
                "ForwardAgent=no",
                "-o",
                "ClearAllForwardings=yes",
                "-o",
                "PermitLocalCommand=no",
                "-o",
                "ConnectTimeout=10",
            ]);
        }
        match &draft.connection {
            SshConnection::Alias { alias } => {
                command.arg(alias);
            }
            SshConnection::Manual { host, user, port } => {
                command
                    .arg("-l")
                    .arg(user)
                    .arg("-p")
                    .arg(port.to_string())
                    .arg(host);
            }
        }
        if !config_only {
            command.arg(
                r#"am_has_agent() {
    command -v "$1" >/dev/null 2>&1 ||
        { test -n "${HOME:-}" && test -f "$HOME/.local/bin/$1" && test -x "$HOME/.local/bin/$1"; }
}
printf 'AM_SSH_INVENTORY_V1\n'
printf 'host='; hostname
printf 'user='; id -un
if am_has_agent claude; then printf 'claude=1\n'; else printf 'claude=0\n'; fi
if am_has_agent codex; then printf 'codex=1\n'; else printf 'codex=0\n'; fi
printf 'AM_SSH_END_V1\n'"#,
            );
        }
        let mut child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                error(
                    SshErrorKind::ClientUnavailable,
                    "OpenSSH client is unavailable.",
                )
            } else {
                error(SshErrorKind::Connection, "Could not start the SSH client.")
            }
        })?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| error(SshErrorKind::Protocol, "SSH output unavailable."))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| error(SshErrorKind::Protocol, "SSH error output unavailable."))?;
        let out = thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.take(65537).read_to_end(&mut bytes);
            bytes
        });
        let err = thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.take(65537).read_to_end(&mut bytes);
            bytes
        });
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if start.elapsed() < self.timeout => {
                    thread::sleep(Duration::from_millis(10))
                }
                Ok(None) => {
                    terminate(&mut child);
                    drop(out);
                    drop(err);
                    return Err(error(SshErrorKind::Timeout, "SSH connection timed out."));
                }
                Err(_) => {
                    terminate(&mut child);
                    drop(out);
                    drop(err);
                    return Err(error(SshErrorKind::Connection, "SSH process failed."));
                }
            }
        };
        let output = out
            .join()
            .map_err(|_| error(SshErrorKind::Protocol, "SSH output unavailable."))?;
        let stderr = err
            .join()
            .map_err(|_| error(SshErrorKind::Protocol, "SSH error output unavailable."))?;
        if !status.success() {
            let msg = String::from_utf8_lossy(&stderr).to_lowercase();
            return Err(
                if msg.contains("host key")
                    || msg.contains("remote host identification")
                    || msg.contains("no matching host key")
                {
                    error(
                        SshErrorKind::HostKey,
                        "Host key verification failed. Verify the host identity with OpenSSH outside the app.",
                    )
                } else if msg.contains("permission denied") || msg.contains("authentication failed")
                {
                    error(
                        SshErrorKind::Authentication,
                        "SSH authentication failed. Configure non-interactive OpenSSH authentication outside the app.",
                    )
                } else if msg.contains("timed out") {
                    error(SshErrorKind::Timeout, "SSH connection timed out.")
                } else {
                    error(
                        SshErrorKind::Connection,
                        "SSH connection failed. Check the destination and OpenSSH configuration.",
                    )
                },
            );
        }
        if output.len() > 65536 {
            return Err(error(
                SshErrorKind::Protocol,
                "SSH output exceeded the limit.",
            ));
        }
        String::from_utf8(output)
            .map_err(|_| error(SshErrorKind::Protocol, "SSH output was not valid UTF-8."))
    }
    pub(crate) fn resolved_port(&self, draft: &SshEnvironmentDraft) -> Result<u16, CoreError> {
        let resolved = self.run(draft, true)?;
        if resolved.lines().any(|line| {
            line.strip_prefix("proxycommand ")
                .or_else(|| line.strip_prefix("proxyjump "))
                .is_some_and(|value| !value.eq_ignore_ascii_case("none"))
        }) {
            return Err(error(
                SshErrorKind::Protocol,
                "Remote editing requires a directly authenticated SSH connection.",
            ));
        }
        resolved
            .lines()
            .find_map(|line| line.strip_prefix("port "))
            .and_then(|port| port.parse::<u16>().ok())
            .filter(|port| *port > 0)
            .ok_or_else(|| error(SshErrorKind::Protocol, "Could not resolve the SSH port."))
    }
    pub(crate) fn remote_command(
        &self,
        draft: &SshEnvironmentDraft,
        remote: &str,
    ) -> Result<Command, CoreError> {
        validate(draft)?;
        let mut command = Command::new(&self.program);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        if let Some(config) = &self.config {
            command.arg("-F").arg(config);
        }
        command.args([
            "-v",
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ForwardAgent=no",
            "-o",
            "ClearAllForwardings=yes",
            "-o",
            "PermitLocalCommand=no",
            "-o",
            "ConnectTimeout=10",
            "-o",
            "ProxyJump=none",
            "-o",
            "ProxyCommand=none",
            "-o",
            "ControlMaster=no",
            "-o",
            "ControlPath=none",
        ]);
        match &draft.connection {
            SshConnection::Alias { alias } => {
                command.arg(alias);
            }
            SshConnection::Manual { host, user, port } => {
                command
                    .arg("-l")
                    .arg(user)
                    .arg("-p")
                    .arg(port.to_string())
                    .arg(host);
            }
        }
        command
            .arg(remote)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Ok(command)
    }
    pub(crate) fn operation_timeout(&self) -> Duration {
        self.timeout
    }
    pub fn inventory(&self, draft: &SshEnvironmentDraft) -> Result<SshInventory, CoreError> {
        let config = self.run(draft, true)?;
        let port = config
            .lines()
            .find_map(|l| l.strip_prefix("port "))
            .and_then(|p| p.parse::<u16>().ok())
            .filter(|p| *p > 0)
            .ok_or_else(|| error(SshErrorKind::Protocol, "Could not resolve the SSH port."))?;
        let output = self.run(draft, false)?;
        let mut lines = output.lines();
        if lines.next() != Some("AM_SSH_INVENTORY_V1") {
            return Err(error(
                SshErrorKind::Protocol,
                "Unexpected SSH inventory output.",
            ));
        }
        let host = lines
            .next()
            .and_then(|s| s.strip_prefix("host="))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| error(SshErrorKind::Protocol, "Remote host identity unavailable."))?;
        let user = lines
            .next()
            .and_then(|s| s.strip_prefix("user="))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                error(
                    SshErrorKind::Protocol,
                    "Remote account identity unavailable.",
                )
            })?;
        let claude = match lines.next() {
            Some("claude=1") => true,
            Some("claude=0") => false,
            _ => return Err(error(SshErrorKind::Protocol, "Invalid SSH inventory.")),
        };
        let codex = match lines.next() {
            Some("codex=1") => true,
            Some("codex=0") => false,
            _ => return Err(error(SshErrorKind::Protocol, "Invalid SSH inventory.")),
        };
        if lines.next() != Some("AM_SSH_END_V1") || lines.next().is_some() {
            return Err(error(SshErrorKind::Protocol, "Invalid SSH inventory."));
        }
        Ok(SshInventory {
            host: host.into(),
            user: user.into(),
            port,
            claude,
            codex,
        })
    }
}
pub fn inventory(draft: &SshEnvironmentDraft) -> Result<SshInventory, CoreError> {
    SshTransport::default().inventory(draft)
}
