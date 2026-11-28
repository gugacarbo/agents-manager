#![allow(clippy::unwrap_used)]

use agents_manager_core::domain::{SshConnection, SshEnvironmentDraft};
use agents_manager_remote::{
    ssh::SshTransport,
    ssh_sessions::{SshSessionReference, probe_sessions},
};
use std::{io::Write, os::unix::fs::OpenOptionsExt, time::Duration};
use tempfile::tempdir;

/// Writes an executable script with its final mode in a single `open`, so no process can observe
/// a writable-or-unprivileged intermediate state between creation and the first spawn.
fn write_executable(path: &std::path::Path, contents: &str) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o700)
        .open(path)
        .unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    file.sync_all().unwrap();
    drop(file);
}

fn draft() -> SshEnvironmentDraft {
    SshEnvironmentDraft {
        name: "workstation".into(),
        connection: SshConnection::Alias {
            alias: "work".into(),
        },
    }
}

fn simulated_ssh(tmp: &tempfile::TempDir, reply: &str) -> SshTransport {
    let program = tmp.path().join("ssh-sim");
    let output = tmp.path().join("request.json");
    let script = format!(
        "#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = -G ]; then printf 'hostname remote\\nuser alice\\nport 22\\n'; exit 0; fi; done\ncat > '{}'\necho 'Connecting to host [127.0.0.1] port 22.' >&2\necho 'Server host key: ssh-ed25519 SHA256:simulated' >&2\nprintf '%s\\n' '{}'\nprintf '%s\\n' '{}'\n",
        output.display(),
        r#"{"host":"remote","user":"alice","home":"/home/alice"}"#,
        reply
    );
    write_executable(&program, &script);
    SshTransport::new(program, None, Duration::from_secs(1))
}

#[test]
fn probes_known_references_over_one_simulated_read_only_ssh_operation() {
    let tmp = tempdir().unwrap();
    let response = r#"{"sessions":[{"sessionId":"a1b2c3d4-1234-1234-1234-123456789abc","cwd":"/home/alice/project","transcriptPath":"/home/alice/.claude/projects/-home-alice-project/a1b2c3d4-1234-1234-1234-123456789abc.jsonl","available":true,"active":false,"reason":null}]}"#;
    let transport = simulated_ssh(&tmp, response);
    let refs = [SshSessionReference {
        session_id: "a1b2c3d4-1234-1234-1234-123456789abc".into(),
        cwd: "/home/alice/project".into(),
        transcript_path: "/home/alice/.claude/projects/-home-alice-project/a1b2c3d4-1234-1234-1234-123456789abc.jsonl".into(),
    }];

    let result = probe_sessions(&transport, &draft(), &refs).unwrap();

    assert_eq!(result.len(), 1);
    assert!(result[0].available);
    assert!(!result[0].active);
    assert_eq!(result[0].host, "remote");
    assert_eq!(result[0].user, "alice");
    assert_eq!(result[0].session_id, refs[0].session_id);
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("request.json")).unwrap()).unwrap();
    assert_eq!(request["op"], "probe_sessions");
    assert_eq!(request["references"][0]["sessionId"], refs[0].session_id);
    assert!(!request.to_string().contains("transcript content"));
}

#[test]
fn exposes_resolved_direct_ssh_conversation_route() {
    let tmp = tempdir().unwrap();
    let transport = simulated_ssh(&tmp, r#"{"sessions":[]}"#);
    assert_eq!(
        transport.conversation_route(&draft()).unwrap(),
        ("remote".into(), "alice".into(), 22)
    );
}

#[test]
fn resolves_reference_route_without_forcing_a_port() {
    let tmp = tempdir().unwrap();
    let transport = simulated_ssh(&tmp, r#"{"sessions":[]}"#);
    assert_eq!(
        transport
            .conversation_reference_route("alice@remote", None)
            .unwrap(),
        ("remote".into(), "alice".into(), 22)
    );
}

/// Simulated `ssh` that fails before the remote operation can answer, as an unreachable host does.
fn unreachable_ssh(tmp: &tempfile::TempDir) -> SshTransport {
    let program = tmp.path().join("ssh-sim");
    let script = "#!/bin/sh\nfor arg in \"$@\"; do if [ \"$arg\" = -G ]; then printf 'hostname remote\\nuser alice\\nport 22\\n'; exit 0; fi; done\nprintf 'ssh: connect to host remote port 22: No route to host\\n' >&2\nexit 255\n";
    write_executable(&program, script);
    SshTransport::new(program, None, Duration::from_secs(1))
}

const READY: &str = "a1b2c3d4-1234-1234-1234-123456789abc";
const RUNNING: &str = "b1b2c3d4-1234-1234-1234-123456789abc";
const GONE: &str = "c1b2c3d4-1234-1234-1234-123456789abc";

fn reference(id: &str) -> SshSessionReference {
    SshSessionReference {
        session_id: id.into(),
        cwd: "/home/alice/project".into(),
        transcript_path: format!("/home/alice/.claude/projects/-home-alice-project/{id}.jsonl"),
    }
}

// Edge case 3: the host does not answer. The probe reports that origin as an error instead of
// returning partial references, and a working host keeps working.
#[test]
fn ec3_unreachable_host_is_an_error_for_that_origin_and_leaves_no_partial_references() {
    let refs = [reference(READY), reference(RUNNING), reference(GONE)];
    let down_dir = tempdir().unwrap();
    let down = unreachable_ssh(&down_dir);
    let error = probe_sessions(&down, &draft(), &refs).unwrap_err();
    assert!(
        matches!(
            error,
            agents_manager_core::error::CoreError::SshError { ref kind, .. }
                if *kind == agents_manager_core::domain::SshErrorKind::Connection
        ),
        "unexpected error: {error:?}"
    );

    // Another host, probed with its own transport, still answers in the same call pattern.
    let ok_dir = tempdir().unwrap();
    let ok = simulated_ssh(
        &ok_dir,
        r#"{"sessions":[{"sessionId":"a1b2c3d4-1234-1234-1234-123456789abc","cwd":"/home/alice/project","transcriptPath":"/home/alice/.claude/projects/-home-alice-project/a1b2c3d4-1234-1234-1234-123456789abc.jsonl","available":true,"active":false,"reason":null},{"sessionId":"b1b2c3d4-1234-1234-1234-123456789abc","cwd":"/home/alice/project","transcriptPath":"/home/alice/.claude/projects/-home-alice-project/b1b2c3d4-1234-1234-1234-123456789abc.jsonl","available":false,"active":false,"reason":null},{"sessionId":"c1b2c3d4-1234-1234-1234-123456789abc","cwd":"/home/alice/project","transcriptPath":"/home/alice/.claude/projects/-home-alice-project/c1b2c3d4-1234-1234-1234-123456789abc.jsonl","available":false,"active":false,"reason":null}]}"#,
    );
    assert_eq!(probe_sessions(&ok, &draft(), &refs).unwrap().len(), 3);
}

// Edge case 5: an active session and an unreadable transcript come back from the probe as
// unavailable with the matching reason, and never as available.
#[test]
fn ec5_active_session_and_unreadable_transcript_are_unavailable_with_their_reason() {
    let tmp = tempdir().unwrap();
    let response = format!(
        r#"{{"sessions":[{{"sessionId":"{RUNNING}","cwd":"/home/alice/project","transcriptPath":"/home/alice/.claude/projects/-home-alice-project/{RUNNING}.jsonl","available":true,"active":true,"reason":"active"}},{{"sessionId":"{GONE}","cwd":"/home/alice/project","transcriptPath":"/home/alice/.claude/projects/-home-alice-project/{GONE}.jsonl","available":false,"active":false,"reason":"inaccessibleWorkingDirectory"}}]}}"#
    );
    let transport = simulated_ssh(&tmp, &response);
    let refs = [reference(RUNNING), reference(GONE)];

    let probes = probe_sessions(&transport, &draft(), &refs).unwrap();

    assert!(probes[0].active);
    assert_eq!(probes[0].reason.as_deref(), Some("active"));
    assert!(!probes[1].available);
    assert!(!probes[1].active);
    assert_eq!(
        probes[1].reason.as_deref(),
        Some("inaccessibleWorkingDirectory")
    );
}
