use crate::{ssh, ssh::SshTransport};
use agents_manager_core::{
    domain::{SshEnvironmentDraft, SshErrorKind},
    error::CoreError,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::Stdio,
    thread,
    time::{Duration, Instant},
};

const SCRIPT: &str = include_str!("ssh_sessions.py");
const MAX_OUTPUT: usize = 4 * 1024 * 1024;
const MAX_REQUEST: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshSessionReference {
    pub session_id: String,
    pub cwd: String,
    pub transcript_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshSessionProbe {
    pub host: String,
    pub user: String,
    pub port: u16,
    pub session_id: String,
    pub cwd: String,
    pub transcript_path: String,
    pub available: bool,
    pub active: bool,
    pub reason: Option<String>,
}

#[derive(Serialize)]
struct Request<'a> {
    op: &'static str,
    references: &'a [SshSessionReference],
}

#[derive(Deserialize)]
struct Hello {
    host: String,
    user: String,
    home: String,
}

#[derive(Deserialize)]
struct Response {
    sessions: Vec<RemoteProbe>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteProbe {
    session_id: String,
    cwd: String,
    transcript_path: String,
    available: bool,
    active: bool,
    reason: Option<String>,
}

fn protocol_error(message: &str) -> CoreError {
    CoreError::SshError {
        kind: SshErrorKind::Protocol,
        message: message.into(),
    }
}

fn read_line(reader: &mut impl BufRead) -> Result<Vec<u8>, CoreError> {
    let mut bytes = Vec::new();
    reader
        .take((MAX_OUTPUT + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .map_err(|_| protocol_error("SSH response could not be read."))?;
    if bytes.len() > MAX_OUTPUT {
        return Err(protocol_error("SSH response exceeded the limit."));
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    Ok(bytes)
}

/// Probes only the supplied SSH session references. The fixed remote operation returns metadata,
/// never transcript contents, and performs no writes on the host.
pub fn probe_sessions(
    transport: &SshTransport,
    draft: &SshEnvironmentDraft,
    references: &[SshSessionReference],
) -> Result<Vec<SshSessionProbe>, CoreError> {
    if references.is_empty() {
        return Ok(Vec::new());
    }
    if references.len() > 2048 {
        return Err(protocol_error("Too many SSH sessions to probe."));
    }
    let port = transport.resolved_port(draft)?;
    let quoted = format!("'{}'", SCRIPT.replace('\'', "'\\''"));
    let remote = format!("/usr/bin/python3 -u -c {quoted}");
    let mut command = transport.remote_command(draft, &remote)?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = ssh::spawn_retrying_txtbusy(&mut command).map_err(|e| CoreError::SshError {
        kind: if e.kind() == std::io::ErrorKind::NotFound {
            SshErrorKind::ClientUnavailable
        } else {
            SshErrorKind::Connection
        },
        message: if e.kind() == std::io::ErrorKind::NotFound {
            "Could not start SSH session discovery.".into()
        } else {
            format!("Could not start SSH session discovery: {e}.")
        },
    })?;
    let input = child
        .stdin
        .take()
        .ok_or_else(|| protocol_error("SSH input is unavailable."))?;
    let output = child
        .stdout
        .take()
        .ok_or_else(|| protocol_error("SSH output is unavailable."))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| protocol_error("SSH error output is unavailable."))?;
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.take(65537).read_to_end(&mut bytes);
        bytes
    });
    let output_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = output.take((MAX_OUTPUT + 1) as u64).read_to_end(&mut bytes);
        bytes
    });
    let request = serde_json::to_vec(&Request {
        op: "probe_sessions",
        references,
    })?;
    if request.len() > MAX_REQUEST {
        ssh::terminate(&mut child);
        return Err(protocol_error(
            "SSH session probe request exceeded the limit.",
        ));
    }
    let started = Instant::now();
    let timeout = transport.operation_timeout();
    let (write_tx, write_rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut input = input;
        let result = input
            .write_all(&request)
            .and_then(|_| input.write_all(b"\n"));
        let _ = write_tx.send(result.map_err(|_| ()));
    });
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                ssh::terminate(&mut child);
                return Err(CoreError::SshError {
                    kind: SshErrorKind::Timeout,
                    message: "SSH session probe timed out.".into(),
                });
            }
            Err(_) => {
                ssh::terminate(&mut child);
                return Err(CoreError::SshError {
                    kind: SshErrorKind::Connection,
                    message: "SSH session probe failed.".into(),
                });
            }
        }
    };
    let stderr = stderr_reader.join().unwrap_or_default();
    let output = output_reader.join().unwrap_or_default();
    if output.len() > MAX_OUTPUT {
        return Err(protocol_error("SSH response exceeded the limit."));
    }
    if let Ok(Err(())) = write_rx.try_recv() {
        return Err(protocol_error("SSH request could not be sent."));
    }
    drop(stderr);
    if !status.success() {
        return Err(CoreError::SshError {
            kind: SshErrorKind::Connection,
            message: "SSH session probe failed.".into(),
        });
    }
    let mut output = BufReader::new(output.as_slice());
    let hello: Hello = serde_json::from_slice(&read_line(&mut output)?)
        .map_err(|_| protocol_error("Invalid SSH session probe identity."))?;
    if hello.host.is_empty() || hello.user.is_empty() || !Path::new(&hello.home).is_absolute() {
        return Err(protocol_error("Invalid SSH session probe identity."));
    }
    let response: Response = serde_json::from_slice(&read_line(&mut output)?)
        .map_err(|_| protocol_error("Invalid SSH session probe response."))?;
    if output.read(&mut [0_u8; 1])? != 0 || response.sessions.len() != references.len() {
        return Err(protocol_error(
            "SSH session probe response did not match the request.",
        ));
    }
    response
        .sessions
        .into_iter()
        .zip(references)
        .map(|(probe, reference)| {
            if probe.session_id != reference.session_id
                || probe.cwd != reference.cwd
                || probe.transcript_path != reference.transcript_path
                || (probe.available && probe.active && probe.reason.as_deref() != Some("active"))
            {
                return Err(protocol_error(
                    "SSH session probe response did not match the request.",
                ));
            }
            Ok(SshSessionProbe {
                host: hello.host.clone(),
                user: hello.user.clone(),
                port,
                session_id: probe.session_id,
                cwd: probe.cwd,
                transcript_path: probe.transcript_path,
                available: probe.available,
                active: probe.active,
                reason: probe.reason,
            })
        })
        .collect()
}
