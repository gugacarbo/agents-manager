use agents_manager_core::{
    domain::{RemoteConfigDocument, RemoteConfigId},
    error::CoreError,
};
pub fn validate_draft(doc: &RemoteConfigDocument, content: &str) -> Result<(), CoreError> {
    match doc.entry.format {
        agents_manager_core::domain::ConfigFormat::Json => {
            let value: serde_json::Value =
                serde_json::from_str(content).map_err(|e| CoreError::SyntaxError {
                    message: e.to_string(),
                })?;
            if matches!(
                doc.entry.id,
                RemoteConfigId::ClaudeHome | RemoteConfigId::ClaudeNested
            ) {
                let old: serde_json::Value =
                    serde_json::from_str(&doc.content).map_err(|e| CoreError::SyntaxError {
                        message: e.to_string(),
                    })?;
                if old.get("projects") != value.get("projects") {
                    return Err(CoreError::ProtectedField);
                }
            }
        }
        agents_manager_core::domain::ConfigFormat::Toml => {
            content
                .parse::<toml::Table>()
                .map_err(|e| CoreError::SyntaxError {
                    message: e.to_string(),
                })?;
        }
        _ => {
            return Err(CoreError::PathNotAllowed {
                path: doc.path.clone(),
            });
        }
    }
    Ok(())
}

use crate::ssh::{self, SshTransport};
use agents_manager_core::{
    config::detect_secrets,
    domain::{
        AgentKind, ConfigFormat, RemoteConfigEntry, RemoteSaveResult, SshEnvironmentDraft,
        SshErrorKind,
    },
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin},
    sync::mpsc::{self, Receiver},
    thread,
    time::Instant,
};
const SCRIPT: &str = include_str!("remote_config.py");
const MAX_OUTPUT: u64 = 8 * 1024 * 1024;
fn err(message: &str) -> CoreError {
    CoreError::SshError {
        kind: SshErrorKind::Protocol,
        message: message.into(),
    }
}
fn entry(id: RemoteConfigId) -> RemoteConfigEntry {
    let (agent, label, format) = match id {
        RemoteConfigId::ClaudeSettings => {
            (AgentKind::ClaudeCode, "Claude settings", ConfigFormat::Json)
        }
        RemoteConfigId::ClaudeHome => (
            AgentKind::ClaudeCode,
            "Claude configuration",
            ConfigFormat::Json,
        ),
        RemoteConfigId::ClaudeNested => (
            AgentKind::ClaudeCode,
            "Claude nested configuration",
            ConfigFormat::Json,
        ),
        RemoteConfigId::CodexConfig => {
            (AgentKind::Codex, "Codex configuration", ConfigFormat::Toml)
        }
    };
    RemoteConfigEntry {
        id,
        agent,
        label: label.into(),
        format,
    }
}
fn key(id: RemoteConfigId) -> &'static str {
    match id {
        RemoteConfigId::ClaudeSettings => "claude-settings",
        RemoteConfigId::ClaudeHome => "claude-home",
        RemoteConfigId::ClaudeNested => "claude-nested",
        RemoteConfigId::CodexConfig => "codex-config",
    }
}
fn relative(id: RemoteConfigId) -> &'static str {
    match id {
        RemoteConfigId::ClaudeSettings => ".claude/settings.json",
        RemoteConfigId::ClaudeHome => ".claude.json",
        RemoteConfigId::ClaudeNested => ".claude/.claude.json",
        RemoteConfigId::CodexConfig => ".codex/config.toml",
    }
}
#[derive(Deserialize)]
struct Hello {
    host: String,
    user: String,
    home: String,
}
#[derive(Deserialize)]
struct RemoteFile {
    path: String,
    content: String,
    sha256: String,
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    metadata_hash: String,
}
struct Session {
    child: Child,
    input: ChildStdin,
    output: Receiver<Vec<u8>>,
    deadline: Instant,
    host_key: String,
    hello: Hello,
    port: u16,
}
impl Drop for Session {
    fn drop(&mut self) {
        ssh::terminate(&mut self.child);
    }
}
impl Session {
    fn start(transport: &SshTransport, draft: &SshEnvironmentDraft) -> Result<Self, CoreError> {
        let port = transport.resolved_port(draft)?;
        let quoted = format!("'{}'", SCRIPT.replace('\'', "'\\''"));
        let remote = format!("/usr/bin/python3 -u -c {quoted}");
        let mut command = transport.remote_command(draft, &remote)?;
        let mut child = command.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CoreError::SshError {
                    kind: SshErrorKind::ClientUnavailable,
                    message: "OpenSSH client is unavailable.".into(),
                }
            } else {
                err("Could not start SSH.")
            }
        })?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| err("SSH input unavailable."))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| err("SSH output unavailable."))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| err("SSH identity output unavailable."))?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(std::io::Read::take(stdout, MAX_OUTPUT));
            loop {
                let mut line = Vec::new();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let (key_tx, key_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(std::io::Read::take(stderr, MAX_OUTPUT));
            let mut connected_port = None;
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if line.contains("Connecting to ")
                            && let Some((_, suffix)) = line.rsplit_once(" port ")
                        {
                            connected_port =
                                suffix.trim().trim_end_matches('.').parse::<u16>().ok();
                        }
                        if line.contains("Server host key:")
                            && let Some(k) =
                                line.split_whitespace().find(|s| s.starts_with("SHA256:"))
                        {
                            let _ = key_tx.send((k.to_owned(), connected_port));
                        }
                    }
                }
            }
        });
        let deadline = Instant::now() + transport.operation_timeout();
        let (host_key, connected_port) =
            match key_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(key) => key,
                Err(_) => {
                    ssh::terminate(&mut child);
                    return Err(err("Unable to verify authenticated SSH host identity."));
                }
            };
        if connected_port != Some(port) {
            ssh::terminate(&mut child);
            return Err(CoreError::RemoteDestinationChanged);
        }
        let line = match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(line) => line,
            Err(_) => {
                ssh::terminate(&mut child);
                return Err(err("Remote SSH session did not start."));
            }
        };
        let hello: Hello = match serde_json::from_slice(&line) {
            Ok(hello) => hello,
            Err(_) => {
                ssh::terminate(&mut child);
                return Err(err("Invalid remote identity response."));
            }
        };
        if hello.user.is_empty() || hello.host.is_empty() || !Path::new(&hello.home).is_absolute() {
            ssh::terminate(&mut child);
            return Err(err("Invalid authenticated remote identity."));
        }
        Ok(Self {
            child,
            input,
            output: rx,
            deadline,
            host_key,
            hello,
            port,
        })
    }
    fn request(
        &mut self,
        request: &serde_json::Value,
        write: bool,
    ) -> Result<serde_json::Value, CoreError> {
        let mut bytes = serde_json::to_vec(request)?;
        bytes.push(b'\n');
        if self
            .input
            .write_all(&bytes)
            .and_then(|_| self.input.flush())
            .is_err()
        {
            return Err(if write {
                CoreError::RemoteUnconfirmed
            } else {
                err("Remote SSH request failed.")
            });
        }
        let line = self
            .output
            .recv_timeout(self.deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| {
                if write {
                    CoreError::RemoteUnconfirmed
                } else {
                    err("Remote SSH response unavailable.")
                }
            })?;
        let value: serde_json::Value = serde_json::from_slice(&line).map_err(|_| {
            if write {
                CoreError::RemoteUnconfirmed
            } else {
                err("Invalid remote SSH response.")
            }
        })?;
        if let Some(code) = value.get("error").and_then(serde_json::Value::as_str) {
            return Err(match code {
                "missing" if write => CoreError::ExternalConflict,
                "missing" => CoreError::NotFound {
                    target: "Remote configuration".into(),
                },
                "permission" => CoreError::PermissionDenied {
                    message: "Remote configuration is not accessible.".into(),
                },
                "disallowed" => CoreError::PathNotAllowed {
                    path: "Remote configuration".into(),
                },
                "conflict" => CoreError::ExternalConflict,
                "unconfirmed" => CoreError::RemoteUnconfirmed,
                "encoding" => CoreError::IoError {
                    message: "Remote configuration is not UTF-8.".into(),
                },
                _ => CoreError::IoError {
                    message: "Remote operation failed safely.".into(),
                },
            });
        }
        Ok(value)
    }
    fn read(&mut self, id: RemoteConfigId) -> Result<RemoteFile, CoreError> {
        let value = self.request(&serde_json::json!({"op":"read","id":key(id)}), false)?;
        let f: RemoteFile =
            serde_json::from_value(value).map_err(|_| err("Invalid remote file response."))?;
        let expected = Path::new(&self.hello.home).join(relative(id));
        if Path::new(&f.path) != expected {
            return Err(CoreError::RemoteDestinationChanged);
        }
        Ok(f)
    }
}
fn hash(s: &[u8]) -> String {
    format!("{:x}", Sha256::digest(s))
}
fn document(
    session: &Session,
    id: RemoteConfigId,
    file: RemoteFile,
) -> Result<RemoteConfigDocument, CoreError> {
    if hash(file.content.as_bytes()) != file.sha256 {
        return Err(err("Remote content hash did not match."));
    }
    let secret_spans = detect_secrets(&file.content);
    Ok(RemoteConfigDocument {
        entry: entry(id),
        host: session.hello.host.clone(),
        user: session.hello.user.clone(),
        port: session.port,
        path: file.path,
        content: file.content,
        sha256: file.sha256,
        secret_spans,
        host_key: session.host_key.clone(),
        device: file.device,
        inode: file.inode,
        mode: file.mode,
        uid: file.uid,
        gid: file.gid,
        metadata_hash: file.metadata_hash,
    })
}
pub fn list_files(
    transport: &SshTransport,
    draft: &SshEnvironmentDraft,
) -> Result<Vec<RemoteConfigEntry>, CoreError> {
    let mut session = Session::start(transport, draft)?;
    let mut entries = Vec::new();
    for id in [
        RemoteConfigId::ClaudeSettings,
        RemoteConfigId::ClaudeHome,
        RemoteConfigId::ClaudeNested,
        RemoteConfigId::CodexConfig,
    ] {
        match session.read(id) {
            Ok(_) | Err(CoreError::PermissionDenied { .. }) => entries.push(entry(id)),
            Err(CoreError::NotFound { .. }) => (),
            Err(error) => return Err(error),
        }
    }
    Ok(entries)
}
pub fn read_config(
    transport: &SshTransport,
    draft: &SshEnvironmentDraft,
    id: RemoteConfigId,
) -> Result<RemoteConfigDocument, CoreError> {
    let mut session = Session::start(transport, draft)?;
    let file = session.read(id)?;
    document(&session, id, file)
}
fn same_file(doc: &RemoteConfigDocument, f: &RemoteFile) -> bool {
    doc.path == f.path
        && doc.sha256 == f.sha256
        && doc.device == f.device
        && doc.inode == f.inode
        && doc.mode == f.mode
        && doc.uid == f.uid
        && doc.gid == f.gid
        && doc.metadata_hash == f.metadata_hash
        && doc.content == f.content
}
fn snapshot(backups: &Path, doc: &RemoteConfigDocument) -> Result<PathBuf, CoreError> {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(backups)?;
    #[cfg(unix)]
    fs::set_permissions(backups, fs::Permissions::from_mode(0o700))?;
    let prefix = hash(
        format!(
            "{}\0{}\0{}\0{}\0{}",
            doc.host_key, doc.host, doc.user, doc.port, doc.path
        )
        .as_bytes(),
    );
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| CoreError::IoError {
            message: e.to_string(),
        })?
        .as_nanos();
    let mut file = tempfile::Builder::new()
        .prefix(&format!("remote-{prefix}-{nanos:039}-"))
        .tempfile_in(backups)?;
    file.write_all(doc.content.as_bytes())?;
    file.as_file().sync_all()?;
    let (_, path) = file.keep().map_err(|e| CoreError::from(e.error))?;
    let mut snapshots = fs::read_dir(backups)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&format!("remote-{prefix}-"))
        })
        .map(|e| e.path())
        .collect::<Vec<_>>();
    snapshots.sort();
    let excess = snapshots.len().saturating_sub(20);
    for old in snapshots.into_iter().take(excess) {
        fs::remove_file(old)?;
    }
    fs::File::open(backups)?.sync_all()?;
    Ok(path)
}
pub fn save_config(
    transport: &SshTransport,
    draft: &SshEnvironmentDraft,
    doc: &RemoteConfigDocument,
    content: &str,
    backups: &Path,
) -> Result<RemoteSaveResult, CoreError> {
    validate_draft(doc, content)?;
    if doc.entry != entry(doc.entry.id) || hash(doc.content.as_bytes()) != doc.sha256 {
        return Err(CoreError::RemoteDestinationChanged);
    }
    let mut session = Session::start(transport, draft)?;
    if session.host_key != doc.host_key
        || session.hello.host != doc.host
        || session.hello.user != doc.user
        || session.port != doc.port
        || Path::new(&session.hello.home).join(relative(doc.entry.id)) != Path::new(&doc.path)
    {
        return Err(CoreError::RemoteDestinationChanged);
    }
    let current = match session.read(doc.entry.id) {
        Err(CoreError::NotFound { .. }) => return Err(CoreError::ExternalConflict),
        other => other?,
    };
    if !same_file(doc, &current) {
        return Err(CoreError::ExternalConflict);
    }
    let backup = snapshot(backups, doc)?;
    let expected = serde_json::json!({"path":doc.path,"sha256":doc.sha256,"device":doc.device,"inode":doc.inode,"mode":doc.mode,"uid":doc.uid,"gid":doc.gid,"metadata_hash":doc.metadata_hash});
    let result = session.request(
        &serde_json::json!({"op":"save","id":key(doc.entry.id),"expected":expected,"content":content}),
        true,
    )?;
    let sha = result
        .get("sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or(CoreError::RemoteUnconfirmed)?;
    if sha != hash(content.as_bytes()) {
        return Err(CoreError::RemoteUnconfirmed);
    }
    Ok(RemoteSaveResult {
        sha256: sha.into(),
        backup_path: backup.to_string_lossy().into_owned(),
    })
}
