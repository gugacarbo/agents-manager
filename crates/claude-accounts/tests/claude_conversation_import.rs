#![allow(clippy::unwrap_used)]

use agents_manager_claude_accounts::conversation_import::{
    ConversationAvailability, ConversationProfile, ConversationSourceKind, RemoteConversationRef,
    RemoteConversationStatus, RemoteConversationValidator, import_selected, list_sessions,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const MANIFEST: &str = ".claude-conversation-imports.json";

struct Fixture {
    _temp: tempfile::TempDir,
    source: ConversationProfile,
    destination: ConversationProfile,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = profile(temp.path(), "source", "account-a", "org-a");
        let destination = profile(temp.path(), "destination", "account-b", "org-b");
        Self {
            _temp: temp,
            source,
            destination,
        }
    }

    fn add_local(&self, id: &str, cwd: &str, extra: Value) {
        let entry = json!({
            "cliSessionId": id,
            "sessionId": format!("local_{id}"),
            "title": format!("Conversation {id}"),
            "titleSource": "user",
            "createdAt": 1795773600000_i64,
            "lastActivityAt": 1795777200000_i64,
            "cwd": cwd,
            "projectPath": cwd,
            "branch": "feature/session",
            "worktreePath": "/repo/.worktrees/session",
            "worktreeName": "session",
            "folderPlace": {"cwd": "/repo/.worktrees/session", "realPath": "/repo/.worktrees/session", "origin": {"cwd": "/repo", "realPath": "/repo", "credential": "must-not-cross-index"}, "unknown": "must-not-cross-index"},
            "isActive": false,
            "secretToken": "must-not-cross-index",
            "nestedUnknown": {"credential": "must-not-cross-index"},
            "extra": extra,
        });
        fs::create_dir_all(index_dir(&self.source)).unwrap();
        fs::write(
            index_dir(&self.source).join(format!("local_{id}.json")),
            entry.to_string(),
        )
        .unwrap();
        let transcript = transcript_path(&self.source, cwd, id);
        fs::create_dir_all(transcript.parent().unwrap()).unwrap();
        fs::write(&transcript, format!("transcript-{id}")).unwrap();
        age(&transcript, Duration::from_secs(600));
    }
}

/// Moves a file's mtime into the past so it does not count as an active session.
fn age(path: &Path, ago: Duration) {
    let file = fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - ago).unwrap();
}

fn profile(root: &Path, id: &str, account_id: &str, organization_id: &str) -> ConversationProfile {
    ConversationProfile {
        id: id.into(),
        config_dir: root.join(id).join("desktop-config"),
        cli_config_dir: root.join(id).join("cli-config"),
        account_id: account_id.into(),
        organization_id: organization_id.into(),
    }
}

fn index_dir(profile: &ConversationProfile) -> PathBuf {
    profile
        .config_dir
        .join("claude-code-sessions")
        .join(&profile.account_id)
        .join(&profile.organization_id)
}

fn transcript_path(profile: &ConversationProfile, cwd: &str, id: &str) -> PathBuf {
    // Same encoding as Claude Code: '/' and '.' become '-'.
    let project = cwd.replace(['/', '.'], "-");
    profile
        .cli_config_dir
        .join("projects")
        .join(project)
        .join(format!("{id}.jsonl"))
}

fn add_ssh(fixture: &Fixture, id: &str) {
    fs::create_dir_all(index_dir(&fixture.source)).unwrap();
    fs::write(
        index_dir(&fixture.source).join(format!("ssh_{id}.json")),
        json!({
            "cliSessionId": id, "name": id, "cwd": "/srv/project", "branch": "main",
            "sshConfig": {"sshHost": "registered-host", "sshUser": "dev", "sshPort": 2222},
            "sshRemoteTranscriptPath": format!("/home/dev/.claude/projects/-srv-project/{id}.jsonl"),
            "isActive": false
        })
        .to_string(),
    )
    .unwrap();
}

/// Denies every SSH reference, as when the host is unreachable or the destination cannot access
/// the working directory.
#[derive(Default)]
struct RemoteDown;
impl RemoteConversationValidator for RemoteDown {
    fn validate_sessions(&self, refs: &[RemoteConversationRef]) -> Vec<RemoteConversationStatus> {
        refs.iter()
            .map(|reference| RemoteConversationStatus {
                availability: ConversationAvailability::Unavailable,
                registered_host_name: Some(reference.ssh_host.clone()),
            })
            .collect()
    }
}

/// `can_write` fails, as when the Desktop was reopened between listing and writing.
struct RemoteDesktopReopened;
impl RemoteConversationValidator for RemoteDesktopReopened {
    fn validate_sessions(&self, refs: &[RemoteConversationRef]) -> Vec<RemoteConversationStatus> {
        Remote.validate_sessions(refs)
    }
    fn can_write(&self) -> bool {
        false
    }
}

fn destination_is_pristine(fixture: &Fixture) {
    assert!(
        !fixture.destination.config_dir.exists(),
        "destination Desktop config was created"
    );
    assert!(
        !fixture.destination.cli_config_dir.exists(),
        "destination CLI dir was created"
    );
}

fn manifest(fixture: &Fixture) -> Value {
    serde_json::from_slice(&fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap())
        .unwrap()
}

#[derive(Default)]
struct Remote;
impl RemoteConversationValidator for Remote {
    fn validate_sessions(&self, refs: &[RemoteConversationRef]) -> Vec<RemoteConversationStatus> {
        refs.iter()
            .map(|reference| RemoteConversationStatus {
                availability: if reference.session_id == "remote-active" {
                    ConversationAvailability::Active
                } else if reference.session_id == "remote-unavailable" {
                    ConversationAvailability::Unavailable
                } else {
                    ConversationAvailability::Available
                },
                registered_host_name: Some("build-host".into()),
            })
            .collect()
    }
}

#[test]
fn listing_projects_safe_fields_and_keeps_worktree_metadata() {
    let fixture = Fixture::new();
    fixture.add_local("local-1", "/repo", json!(null));

    let sessions = list_sessions(&fixture.source, &fixture.destination, &Remote).unwrap();
    assert_eq!(sessions.len(), 1);
    let session = &sessions[0];
    assert_eq!(session.source_kind, ConversationSourceKind::Local);
    assert_eq!(session.availability, ConversationAvailability::Available);
    assert_eq!(session.metadata["worktreePath"], "/repo/.worktrees/session");
    assert_eq!(session.metadata["worktreeName"], "session");
    assert_eq!(
        session.metadata["folderPlace"]["origin"]["realPath"],
        "/repo"
    );
    assert!(
        session.metadata["folderPlace"]["origin"]
            .get("credential")
            .is_none()
    );
    assert!(session.metadata.get("secretToken").is_none());
    assert!(session.metadata.get("nestedUnknown").is_none());
}

#[test]
fn importing_selected_local_session_copies_only_its_transcript_when_cli_roots_are_separate() {
    let fixture = Fixture::new();
    fixture.add_local("local-2", "/repo", json!(null));
    let sessions = list_sessions(&fixture.source, &fixture.destination, &Remote).unwrap();
    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &[sessions[0].id.clone()],
        &Remote,
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].availability, ConversationAvailability::Imported);
    assert_eq!(
        fs::read_to_string(transcript_path(&fixture.destination, "/repo", "local-2")).unwrap(),
        "transcript-local-2"
    );
    let imported: Value = serde_json::from_slice(
        &fs::read(index_dir(&fixture.destination).join("local_local-2.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(imported["worktreePath"], "/repo/.worktrees/session");
    assert!(imported.get("secretToken").is_none());
}

/// EC1 — ADR-0011: "reaproveitando a transcrição local quando acessível". When both profiles
/// share the same `cli_config_dir`, the destination only receives the index entry: the
/// transcript is never copied (no new file under `projects/`), the existing file is reused in
/// place, and a second import is `Ignored` because the destination already has the entry.
#[test]
fn ec1_shared_cli_root_reuses_the_transcript_in_place_and_writes_only_the_index_entry() {
    let fixture = Fixture::new();
    let mut destination = fixture.destination.clone();
    destination.cli_config_dir = fixture.source.cli_config_dir.clone();
    fixture.add_local("shared", "/repo", json!(null));

    let transcript = transcript_path(&fixture.source, "/repo", "shared");
    let before = fs::read(&transcript).unwrap();
    let projects = fixture.source.cli_config_dir.join("projects");
    let files_before = list_files(&projects);
    assert_eq!(files_before.len(), 1, "fixture starts with one transcript");

    let results = import_selected(
        &fixture.source,
        &destination,
        &["local:shared".to_string()],
        &Remote,
    );
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].availability, ConversationAvailability::Imported);

    let entry = index_dir(&destination).join("local_shared.json");
    assert!(
        entry.is_file(),
        "the index entry must be written in the destination Desktop config"
    );
    assert_ne!(
        entry, transcript,
        "index entry lives under config_dir, never under projects/"
    );
    let files_after = list_files(&projects);
    assert_eq!(
        files_after, files_before,
        "no transcript may be copied under a shared projects/"
    );
    assert_eq!(
        fs::read(&transcript).unwrap(),
        before,
        "the existing transcript is reused in place, untouched"
    );
    let written: Value =
        serde_json::from_slice(&fs::read(destination.config_dir.join(MANIFEST)).unwrap()).unwrap();
    assert_eq!(written["imported"].as_array().map(Vec::len), Some(1));

    let again = import_selected(
        &fixture.source,
        &destination,
        &["local:shared".to_string()],
        &Remote,
    );
    assert_eq!(again[0].availability, ConversationAvailability::Ignored);
    assert_eq!(list_files(&projects), files_before, "still nothing copied");
}

fn list_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(list_files(&path));
        } else {
            files.push(path.strip_prefix(dir).unwrap().to_path_buf());
        }
    }
    files.sort();
    files
}

#[test]
fn repeat_import_preserves_destination_edits_and_deletions_and_keeps_input_order() {
    let fixture = Fixture::new();
    fixture.add_local("first", "/repo", json!(null));
    fixture.add_local("second", "/repo", json!(null));
    let sessions = list_sessions(&fixture.source, &fixture.destination, &Remote).unwrap();
    let ids: Vec<_> = sessions.iter().map(|session| session.id.clone()).collect();
    let first = import_selected(&fixture.source, &fixture.destination, &ids, &Remote);
    assert!(
        first
            .iter()
            .all(|result| result.availability == ConversationAvailability::Imported)
    );

    let first_index = index_dir(&fixture.destination).join("local_first.json");
    let mut edited: Value = serde_json::from_slice(&fs::read(&first_index).unwrap()).unwrap();
    edited["name"] = json!("edited in destination");
    fs::write(&first_index, edited.to_string()).unwrap();
    fs::remove_file(index_dir(&fixture.destination).join("local_second.json")).unwrap();

    let repeated = import_selected(
        &fixture.source,
        &fixture.destination,
        &[ids[1].clone(), ids[0].clone()],
        &Remote,
    );
    assert_eq!(
        repeated
            .iter()
            .map(|result| result.session_id.as_str())
            .collect::<Vec<_>>(),
        vec![ids[1].as_str(), ids[0].as_str()]
    );
    assert!(
        repeated
            .iter()
            .all(|result| result.availability == ConversationAvailability::Ignored)
    );
    let saved: Value = serde_json::from_slice(&fs::read(first_index).unwrap()).unwrap();
    assert_eq!(saved["name"], "edited in destination");
    assert!(
        !index_dir(&fixture.destination)
            .join("local_second.json")
            .exists()
    );
}

#[test]
fn remote_sessions_are_batch_validated_and_active_or_unreachable_are_not_imported() {
    let fixture = Fixture::new();
    fs::create_dir_all(index_dir(&fixture.source)).unwrap();
    for id in ["remote-ok", "remote-active", "remote-unavailable"] {
        fs::write(
            index_dir(&fixture.source).join(format!("ssh_{id}.json")),
            json!({
                "cliSessionId": id, "name": id, "cwd": "/srv/project", "branch": "main",
                "sshConfig": {"sshHost": "registered-host"},
                "sshRemoteTranscriptPath": format!("/home/dev/.claude/projects/{id}.jsonl"),
                "isActive": false
            })
            .to_string(),
        )
        .unwrap();
    }
    let sessions = list_sessions(&fixture.source, &fixture.destination, &Remote).unwrap();
    assert_eq!(sessions.len(), 3);
    assert!(
        sessions
            .iter()
            .all(|session| session.source_kind == ConversationSourceKind::Ssh)
    );
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.id == "ssh:remote-active")
            .unwrap()
            .availability,
        ConversationAvailability::Active
    );
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.id == "ssh:remote-unavailable")
            .unwrap()
            .availability,
        ConversationAvailability::Unavailable
    );

    let requested: Vec<_> = sessions.iter().map(|session| session.id.clone()).collect();
    let results = import_selected(&fixture.source, &fixture.destination, &requested, &Remote);
    assert_eq!(
        results
            .iter()
            .map(|result| result.availability)
            .collect::<Vec<_>>(),
        vec![
            ConversationAvailability::Active,
            ConversationAvailability::Imported,
            ConversationAvailability::Unavailable
        ]
    );
    assert!(!fixture.destination.cli_config_dir.join("projects").exists());
}

// --- Edge case 6: import disabled or cancelled -----------------------------------------------------

#[test]
fn ec6_empty_selection_creates_or_changes_nothing_in_the_destination() {
    let fixture = Fixture::new();
    fixture.add_local("kept", "/repo", json!(null));
    // A previous import leaves an index entry and a manifest behind.
    let first = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:kept".to_string()],
        &Remote,
    );
    assert_eq!(first[0].availability, ConversationAvailability::Imported);
    let index = index_dir(&fixture.destination).join("local_kept.json");
    let index_before = fs::read(&index).unwrap();
    let manifest_before = fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap();

    let results = import_selected(&fixture.source, &fixture.destination, &[], &Remote);

    assert!(results.is_empty());
    assert_eq!(fs::read(&index).unwrap(), index_before);
    assert_eq!(
        fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap(),
        manifest_before
    );
    // And with no prior import, an empty selection leaves no trace at all.
    let fresh = Fixture::new();
    fresh.add_local("kept", "/repo", json!(null));
    import_selected(&fresh.source, &fresh.destination, &[], &Remote);
    destination_is_pristine(&fresh);
}

// --- Edge case 3: host does not answer, or destination cannot access the directory -------------------

#[test]
fn ec3_ssh_host_unavailable_skips_only_that_session_and_keeps_local_ones_and_existing_entries() {
    let fixture = Fixture::new();
    fixture.add_local("local-ok", "/repo", json!(null));
    add_ssh(&fixture, "remote-down");
    let sessions = list_sessions(&fixture.source, &fixture.destination, &RemoteDown).unwrap();
    let ssh = sessions
        .iter()
        .find(|session| session.id == "ssh:remote-down")
        .unwrap();
    assert_eq!(ssh.availability, ConversationAvailability::Unavailable);
    assert_eq!(ssh.registered_host_name.as_deref(), Some("registered-host"));
    // A previous, unrelated entry in the destination must survive untouched.
    fixture.add_local("previous", "/repo", json!(null));
    let previous = index_dir(&fixture.destination).join("local_previous.json");
    fs::create_dir_all(previous.parent().unwrap()).unwrap();
    fs::write(
        &previous,
        json!({"cliSessionId": "previous", "name": "kept"}).to_string(),
    )
    .unwrap();
    let previous_before = fs::read(&previous).unwrap();

    let ids: Vec<_> = sessions.iter().map(|session| session.id.clone()).collect();
    let results = import_selected(&fixture.source, &fixture.destination, &ids, &RemoteDown);

    let by_id = |id: &str| {
        results
            .iter()
            .find(|result| result.session_id == id)
            .unwrap()
    };
    assert_eq!(
        by_id("local:local-ok").availability,
        ConversationAvailability::Imported
    );
    let remote = by_id("ssh:remote-down");
    assert_eq!(remote.availability, ConversationAvailability::Unavailable);
    assert!(
        remote
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("unavailable")
    );
    assert!(
        !index_dir(&fixture.destination)
            .join("ssh_remote-down.json")
            .exists()
    );
    assert_eq!(fs::read(&previous).unwrap(), previous_before);
    assert_eq!(
        fs::read_to_string(transcript_path(&fixture.destination, "/repo", "local-ok")).unwrap(),
        "transcript-local-ok"
    );
}

#[test]
fn ec3_unreadable_local_source_is_unavailable_and_nothing_is_copied() {
    let fixture = Fixture::new();
    fixture.add_local("gone", "/repo", json!(null));
    fixture.add_local("fine", "/repo", json!(null));
    fs::remove_file(transcript_path(&fixture.source, "/repo", "gone")).unwrap();

    let sessions = list_sessions(&fixture.source, &fixture.destination, &Remote).unwrap();
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.id == "local:gone")
            .unwrap()
            .availability,
        ConversationAvailability::Unavailable
    );
    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:gone".to_string(), "local:fine".to_string()],
        &Remote,
    );
    assert_eq!(
        results[0].availability,
        ConversationAvailability::Unavailable
    );
    assert!(
        !index_dir(&fixture.destination)
            .join("local_gone.json")
            .exists()
    );
    assert!(
        !transcript_path(&fixture.destination, "/repo", "gone").exists(),
        "transcript was copied for an unavailable session"
    );
    assert_eq!(results[1].availability, ConversationAvailability::Imported);
}

#[test]
fn ec3_desktop_reopened_before_write_fails_every_session_without_touching_the_destination() {
    let fixture = Fixture::new();
    fixture.add_local("one", "/repo", json!(null));
    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:one".to_string()],
        &RemoteDesktopReopened,
    );
    assert_eq!(results[0].availability, ConversationAvailability::Failed);
    assert!(
        results[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("written")
    );
    destination_is_pristine(&fixture);
}

// --- Edge case 5: active session or failed write --------------------------------------------------------

#[test]
fn ec5_local_transcript_modified_within_the_active_window_is_unavailable_and_not_written() {
    let fixture = Fixture::new();
    fixture.add_local("running", "/repo", json!(null));
    let transcript = transcript_path(&fixture.source, "/repo", "running");
    // The fixture aged the file by 10 minutes; bring it back inside the 15 s window.
    fs::OpenOptions::new()
        .write(true)
        .open(&transcript)
        .unwrap()
        .set_modified(SystemTime::now())
        .unwrap();

    let sessions = list_sessions(&fixture.source, &fixture.destination, &Remote).unwrap();
    assert_eq!(sessions[0].availability, ConversationAvailability::Active);
    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:running".to_string()],
        &Remote,
    );
    assert_eq!(results[0].availability, ConversationAvailability::Active);
    assert!(
        results[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("running")
    );
    destination_is_pristine(&fixture);
}

#[test]
fn ec5_index_entry_marked_active_is_unavailable_and_not_written() {
    let fixture = Fixture::new();
    fixture.add_local("busy", "/repo", json!(null));
    let entry = index_dir(&fixture.source).join("local_busy.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&entry).unwrap()).unwrap();
    value["isActive"] = json!(true);
    fs::write(&entry, value.to_string()).unwrap();

    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:busy".to_string()],
        &Remote,
    );
    assert_eq!(results[0].availability, ConversationAvailability::Active);
    destination_is_pristine(&fixture);
}

#[test]
fn ec5_transcript_path_precreated_as_directory_fails_only_that_session_and_leaves_no_partial_file()
{
    let fixture = Fixture::new();
    fixture.add_local("good", "/repo", json!(null));
    fixture.add_local("blocked", "/repo", json!(null));
    // First import so the destination has an index and a manifest to protect.
    import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:good".to_string()],
        &Remote,
    );
    let manifest_before = fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap();
    let good_before = fs::read(index_dir(&fixture.destination).join("local_good.json")).unwrap();
    // Block the transcript destination: the JSONL path already exists as a directory.
    let blocked = transcript_path(&fixture.destination, "/repo", "blocked");
    fs::create_dir_all(&blocked).unwrap();

    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:good".to_string(), "local:blocked".to_string()],
        &Remote,
    );

    assert_eq!(results[0].availability, ConversationAvailability::Ignored);
    assert_eq!(results[1].availability, ConversationAvailability::Failed);
    assert!(
        !index_dir(&fixture.destination)
            .join("local_blocked.json")
            .exists(),
        "index entry written for a failed session"
    );
    assert_eq!(
        fs::read(index_dir(&fixture.destination).join("local_good.json")).unwrap(),
        good_before
    );
    assert_eq!(
        fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap(),
        manifest_before
    );
    // No partial transcript: the blocking directory is still empty and nothing else was left
    // behind (`blocked.jsonl` is the directory this test pre-created).
    assert_eq!(fs::read_dir(&blocked).unwrap().count(), 0);
    let project = blocked.parent().unwrap();
    let leftovers: Vec<_> = fs::read_dir(project)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "good.jsonl" && name != "blocked.jsonl")
        .collect();
    assert!(
        leftovers.is_empty(),
        "partial files left behind: {leftovers:?}"
    );
}

#[test]
fn ec5_manifest_write_failure_rolls_back_the_index_entry_and_reports_failure() {
    let fixture = Fixture::new();
    fixture.add_local("one", "/repo", json!(null));
    // `.claude-conversation-imports.json` pre-created as a directory: the atomic write cannot
    // persist over it.
    fs::create_dir_all(fixture.destination.config_dir.join(MANIFEST)).unwrap();

    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:one".to_string()],
        &Remote,
    );

    assert_eq!(results[0].availability, ConversationAvailability::Failed);
    assert!(
        !index_dir(&fixture.destination)
            .join("local_one.json")
            .exists(),
        "index entry must be rolled back when the manifest cannot be written"
    );
    assert!(
        !transcript_path(&fixture.destination, "/repo", "one").exists(),
        "transcript copy must be rolled back when the manifest cannot be written"
    );
}

#[test]
fn ec5_read_only_index_directory_fails_only_that_session_and_keeps_previous_entries() {
    let fixture = Fixture::new();
    fixture.add_local("one", "/repo", json!(null));
    fixture.add_local("two", "/repo", json!(null));
    import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:one".to_string()],
        &Remote,
    );
    let one_before = fs::read(index_dir(&fixture.destination).join("local_one.json")).unwrap();
    let manifest_before = fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap();
    let index = index_dir(&fixture.destination);
    let mut permissions = fs::metadata(&index).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o500);
    }
    fs::set_permissions(&index, permissions.clone()).unwrap();
    let restore = || {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&index, fs::Permissions::from_mode(0o700));
        }
    };

    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:one".to_string(), "local:two".to_string()],
        &Remote,
    );
    restore();

    assert_eq!(results[0].availability, ConversationAvailability::Ignored);
    assert_eq!(results[1].availability, ConversationAvailability::Failed);
    assert_eq!(
        fs::read(index_dir(&fixture.destination).join("local_one.json")).unwrap(),
        one_before
    );
    assert_eq!(
        fs::read(fixture.destination.config_dir.join(MANIFEST)).unwrap(),
        manifest_before
    );
    assert!(
        !index_dir(&fixture.destination)
            .join("local_two.json")
            .exists()
    );
    assert!(
        !transcript_path(&fixture.destination, "/repo", "two").exists(),
        "transcript copy must be rolled back when the index write fails"
    );
}

// --- Edge case 2 (SSH) -----------------------------------------------------------------------------------

#[test]
fn ec2_ssh_entry_preserves_host_cwd_transcript_path_and_session_id_field_by_field() {
    let fixture = Fixture::new();
    add_ssh(&fixture, "remote-ok");
    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["ssh:remote-ok".to_string()],
        &Remote,
    );
    assert_eq!(results[0].availability, ConversationAvailability::Imported);

    let entry: Value = serde_json::from_slice(
        &fs::read(index_dir(&fixture.destination).join("ssh_remote-ok.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(entry["cliSessionId"], "remote-ok");
    assert_eq!(entry["cwd"], "/srv/project");
    assert_eq!(
        entry["sshRemoteTranscriptPath"],
        "/home/dev/.claude/projects/-srv-project/remote-ok.jsonl"
    );
    assert_eq!(entry["sshConfig"]["sshHost"], "registered-host");
    assert_eq!(entry["sshConfig"]["sshUser"], "dev");
    assert_eq!(entry["sshConfig"]["sshPort"], 2222);
    // No transcript is copied for SSH sessions.
    assert!(!fixture.destination.cli_config_dir.exists());
    assert_eq!(
        manifest(&fixture)["imported"][0],
        "source:ssh:registered-host:remote-ok"
    );
}

// --- Project directory encoding ------------------------------------------------------------------------

#[test]
fn local_copy_preserves_the_source_relative_project_directory_when_cwd_contains_dots() {
    let fixture = Fixture::new();
    let cwd = "/home/dev/app.v2/.worktrees";
    fixture.add_local("dotted", cwd, json!(null));
    // The transcript really lives under Claude Code's encoding (dots become dashes).
    let encoded = cwd.replace(['/', '.'], "-");
    assert_eq!(
        transcript_path(&fixture.source, cwd, "dotted")
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap(),
        encoded
    );

    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:dotted".to_string()],
        &Remote,
    );

    assert_eq!(results[0].availability, ConversationAvailability::Imported);
    let copied = fixture
        .destination
        .cli_config_dir
        .join("projects")
        .join(encoded)
        .join("dotted.jsonl");
    assert_eq!(fs::read_to_string(&copied).unwrap(), "transcript-dotted");
    let entry: Value = serde_json::from_slice(
        &fs::read(index_dir(&fixture.destination).join("local_dotted.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(entry["cwd"], cwd);
}

#[test]
fn local_copy_keeps_the_real_source_project_directory_when_its_encoding_differs() {
    let fixture = Fixture::new();
    let cwd = "/home/dev/project";
    fixture.add_local("moved", cwd, json!(null));
    // Move the transcript to a differently encoded directory, as an older CLI version would.
    let real = fixture
        .source
        .cli_config_dir
        .join("projects")
        .join("-home-dev-project ");
    fs::create_dir_all(&real).unwrap();
    fs::rename(
        transcript_path(&fixture.source, cwd, "moved"),
        real.join("moved.jsonl"),
    )
    .unwrap();

    let results = import_selected(
        &fixture.source,
        &fixture.destination,
        &["local:moved".to_string()],
        &Remote,
    );

    assert_eq!(results[0].availability, ConversationAvailability::Imported);
    assert!(
        fixture
            .destination
            .cli_config_dir
            .join("projects")
            .join("-home-dev-project ")
            .join("moved.jsonl")
            .exists()
    );
}

// --- Edge case 4 (deletion / newer destination version) ---------------------------------------------------

#[test]
fn ec4_destination_deletion_and_edits_are_preserved_with_a_reason() {
    let fixture = Fixture::new();
    fixture.add_local("first", "/repo", json!(null));
    let ids = ["local:first".to_string()];
    import_selected(&fixture.source, &fixture.destination, &ids, &Remote);
    let index = index_dir(&fixture.destination).join("local_first.json");
    let mut edited: Value = serde_json::from_slice(&fs::read(&index).unwrap()).unwrap();
    edited["name"] = json!("edited in destination");
    fs::write(&index, edited.to_string()).unwrap();

    let repeated = import_selected(&fixture.source, &fixture.destination, &ids, &Remote);
    assert_eq!(repeated[0].availability, ConversationAvailability::Ignored);
    assert_eq!(
        repeated[0].message.as_deref(),
        Some("The destination already has this conversation.")
    );

    fs::remove_file(&index).unwrap();
    let after_deletion = import_selected(&fixture.source, &fixture.destination, &ids, &Remote);
    assert_eq!(
        after_deletion[0].availability,
        ConversationAvailability::Ignored
    );
    assert!(
        after_deletion[0]
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("deletion")
    );
    assert!(!index.exists(), "deletion in the destination was undone");
}
