//! Selective import of Claude Desktop conversation index entries.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

const MAX_INDEX_ENTRY_BYTES: u64 = 1024 * 1024;
const IMPORT_MANIFEST: &str = ".claude-conversation-imports.json";
const SAFE_FIELDS: &[&str] = &[
    "cliSessionId",
    "sessionId",
    "uuid",
    "name",
    "title",
    "summary",
    "createdAt",
    "updatedAt",
    "lastActivityAt",
    "titleSource",
    "cwd",
    "projectPath",
    "branch",
    "gitBranch",
    "worktreePath",
    "worktreeName",
    "isActive",
    "isArchived",
    "isPinned",
    "sshRemoteTranscriptPath",
    "remoteSessionId",
    "model",
];

#[derive(Clone, Debug)]
pub struct ConversationProfile {
    pub id: String,
    /// Claude Desktop's configuration directory, containing `claude-code-sessions`.
    pub config_dir: PathBuf,
    /// Claude Code's configuration directory, containing `projects` transcripts.
    pub cli_config_dir: PathBuf,
    pub account_id: String,
    pub organization_id: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationAvailability {
    Available,
    Active,
    Unavailable,
    Imported,
    Ignored,
    Failed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationSourceKind {
    Local,
    Ssh,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConversationSession {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub cwd: String,
    pub source_kind: ConversationSourceKind,
    pub source_profile_id: String,
    pub registered_host_name: Option<String>,
    pub availability: ConversationAvailability,
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteConversationRef {
    pub session_id: String,
    pub ssh_host: String,
    pub cwd: String,
    pub transcript_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteConversationStatus {
    pub availability: ConversationAvailability,
    pub registered_host_name: Option<String>,
}

pub trait RemoteConversationValidator {
    /// Validates references in one or more bounded batches. Results correspond to input order.
    fn validate_sessions(&self, refs: &[RemoteConversationRef]) -> Vec<RemoteConversationStatus>;

    /// Rechecked immediately before each write, after potentially slow SSH validation.
    fn can_write(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConversationImportResult {
    /// Opaque ID supplied by the caller, including for unknown IDs.
    pub session_id: String,
    pub availability: ConversationAvailability,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationImportError {
    StorageUnavailable,
    ProfileUnavailable,
    InvalidProfile,
    InvalidIndex,
}

/// A local session counts as active while its transcript was written within this window, matching
/// the SSH probe.
const ACTIVE_WINDOW: std::time::Duration = std::time::Duration::from_secs(15);

fn transcript_recently_modified(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_or(true, |modified| {
            SystemTime::now()
                .duration_since(modified)
                .is_ok_and(|since| since < ACTIVE_WINDOW)
        })
}

#[derive(Clone, Debug)]
struct Record {
    session: ConversationSession,
    relative_index_path: PathBuf,
    /// Directory of the source transcript relative to `projects/`, preserved in the destination.
    project_directory: String,
    identity: String,
    remote: Option<RemoteConversationRef>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportManifest {
    imported: Vec<String>,
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        && value != "."
        && value != ".."
}

fn validate_profile(profile: &ConversationProfile) -> Result<(), ConversationImportError> {
    if !valid_component(&profile.id)
        || !valid_component(&profile.account_id)
        || !valid_component(&profile.organization_id)
    {
        return Err(ConversationImportError::InvalidProfile);
    }
    Ok(())
}

fn index_dir(profile: &ConversationProfile) -> PathBuf {
    profile
        .config_dir
        .join("claude-code-sessions")
        .join(&profile.account_id)
        .join(&profile.organization_id)
}

fn ensure_real_dir(path: &Path) -> Result<(), ConversationImportError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(ConversationImportError::StorageUnavailable),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|_| ConversationImportError::StorageUnavailable)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                    .map_err(|_| ConversationImportError::StorageUnavailable)?;
            }
            Ok(())
        }
        Err(_) => Err(ConversationImportError::StorageUnavailable),
    }
}

fn ensure_dir_chain(root: &Path, path: &Path) -> Result<(), ConversationImportError> {
    ensure_path(root)?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| ConversationImportError::InvalidProfile)?;
    let mut current = root.to_owned();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(ConversationImportError::InvalidProfile);
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(ConversationImportError::StorageUnavailable),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                ensure_real_dir(&current)?
            }
            Err(_) => return Err(ConversationImportError::StorageUnavailable),
        }
    }
    Ok(())
}

fn ensure_path(path: &Path) -> Result<(), ConversationImportError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(ConversationImportError::StorageUnavailable),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = path
                .parent()
                .ok_or(ConversationImportError::InvalidProfile)?;
            ensure_path(parent)?;
            ensure_real_dir(path)
        }
        Err(_) => Err(ConversationImportError::StorageUnavailable),
    }
}

fn read_json_file(path: &Path) -> Result<Value, ConversationImportError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| ConversationImportError::StorageUnavailable)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_INDEX_ENTRY_BYTES
    {
        return Err(ConversationImportError::InvalidIndex);
    }
    let file = File::open(path).map_err(|_| ConversationImportError::StorageUnavailable)?;
    let mut bytes = Vec::new();
    file.take(MAX_INDEX_ENTRY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ConversationImportError::StorageUnavailable)?;
    serde_json::from_slice(&bytes).map_err(|_| ConversationImportError::InvalidIndex)
}

fn projected_metadata(value: &Value, source_kind: ConversationSourceKind) -> Value {
    let mut output = Map::new();
    if let Some(object) = value.as_object() {
        for key in SAFE_FIELDS {
            if let Some(value) = object.get(*key)
                && !value.is_null()
                && (value.is_string() || value.is_boolean() || value.is_number())
            {
                output.insert((*key).into(), value.clone());
            }
        }
        if let Some(place) = object.get("folderPlace").and_then(Value::as_object) {
            let mut projected = Map::new();
            for key in ["cwd", "realPath"] {
                if let Some(value) = place.get(key).and_then(Value::as_str) {
                    projected.insert(key.into(), Value::String(value.into()));
                }
            }
            if let Some(origin) = place.get("origin").and_then(Value::as_object) {
                let mut safe_origin = Map::new();
                for key in ["cwd", "realPath"] {
                    if let Some(value) = origin.get(key).and_then(Value::as_str) {
                        safe_origin.insert(key.into(), Value::String(value.into()));
                    }
                }
                if !safe_origin.is_empty() {
                    projected.insert("origin".into(), Value::Object(safe_origin));
                }
            }
            if !projected.is_empty() {
                output.insert("folderPlace".into(), Value::Object(projected));
            }
        }
        if source_kind == ConversationSourceKind::Ssh
            && let Some(config) = object.get("sshConfig").and_then(Value::as_object)
        {
            let mut safe_config = Map::new();
            for key in ["sshHost", "sshUser", "sshPort"] {
                if let Some(value) = config.get(key)
                    && (value.is_string() || value.is_number())
                {
                    safe_config.insert(key.into(), value.clone());
                }
            }
            if !safe_config.is_empty() {
                output.insert("sshConfig".into(), Value::Object(safe_config));
            }
        }
    }
    Value::Object(output)
}

fn record_from_file(
    profile: &ConversationProfile,
    file: PathBuf,
    name: &str,
) -> Result<Option<Record>, ConversationImportError> {
    let value = read_json_file(&file)?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let Some(session_id) = object
        .get("cliSessionId")
        .or_else(|| object.get("sessionId"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    if !valid_component(session_id) {
        return Ok(None);
    }
    let remote = name.starts_with("ssh_");
    let source_kind = if remote {
        ConversationSourceKind::Ssh
    } else {
        ConversationSourceKind::Local
    };
    let cwd = object
        .get("cwd")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let title = object
        .get("name")
        .or_else(|| object.get("title"))
        .and_then(Value::as_str)
        .unwrap_or(session_id)
        .to_owned();
    let host = if remote {
        object
            .get("sshConfig")
            .and_then(|config| config.get("sshHost"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    } else {
        None
    };
    let remote_ref = if remote {
        let host = host.clone().unwrap_or_default();
        let transcript_path = object
            .get("sshRemoteTranscriptPath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Some(RemoteConversationRef {
            session_id: session_id.into(),
            ssh_host: host,
            cwd: cwd.clone(),
            transcript_path,
        })
    } else {
        None
    };
    let active = object
        .get("isActive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let opaque_id = if remote {
        format!("ssh:{session_id}")
    } else {
        format!("local:{session_id}")
    };
    let metadata = projected_metadata(&value, source_kind);
    let identity = match source_kind {
        ConversationSourceKind::Local => format!("local:{}", session_id),
        ConversationSourceKind::Ssh => {
            format!("ssh:{}:{}", host.as_deref().unwrap_or_default(), session_id)
        }
    };
    let Some(project_directory) = project_directory(&cwd).ok() else {
        // An entry the Desktop wrote without a usable working directory cannot be located or
        // resumed; skip it instead of failing the whole listing.
        return Ok(None);
    };
    Ok(Some(Record {
        relative_index_path: PathBuf::from(name),
        project_directory,
        session: ConversationSession {
            id: opaque_id,
            session_id: session_id.into(),
            title,
            cwd,
            source_kind,
            source_profile_id: profile.id.clone(),
            registered_host_name: None,
            availability: if active {
                ConversationAvailability::Active
            } else {
                ConversationAvailability::Available
            },
            metadata,
        },
        identity,
        remote: remote_ref,
    }))
}

fn records(profile: &ConversationProfile) -> Result<Vec<Record>, ConversationImportError> {
    validate_profile(profile)?;
    let directory = index_dir(profile);
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(ConversationImportError::StorageUnavailable),
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err(ConversationImportError::StorageUnavailable);
        }
        Ok(_) => {}
    }
    let entries =
        fs::read_dir(&directory).map_err(|_| ConversationImportError::StorageUnavailable)?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| ConversationImportError::StorageUnavailable)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !(name.starts_with("local_") || name.starts_with("ssh_")) || !name.ends_with(".json") {
            continue;
        }
        if let Some(record) = record_from_file(profile, entry.path(), &name)? {
            result.push(record);
        }
    }
    result.sort_by(|a, b| a.session.id.cmp(&b.session.id));
    Ok(result)
}

fn load_manifest(profile: &ConversationProfile) -> Result<ImportManifest, ConversationImportError> {
    let path = profile.config_dir.join(IMPORT_MANIFEST);
    match fs::read(&path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|_| ConversationImportError::InvalidIndex)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ImportManifest::default()),
        Err(_) => Err(ConversationImportError::StorageUnavailable),
    }
}

fn atomic_write(
    path: &Path,
    bytes: &[u8],
    create_new: bool,
) -> Result<(), ConversationImportError> {
    let parent = path
        .parent()
        .ok_or(ConversationImportError::InvalidProfile)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| ConversationImportError::StorageUnavailable)?;
    temp.write_all(bytes)
        .map_err(|_| ConversationImportError::StorageUnavailable)?;
    temp.as_file()
        .sync_all()
        .map_err(|_| ConversationImportError::StorageUnavailable)?;
    if create_new {
        temp.persist_noclobber(path)
            .map_err(|_| ConversationImportError::StorageUnavailable)?;
    } else {
        temp.persist(path)
            .map_err(|_| ConversationImportError::StorageUnavailable)?;
    }
    Ok(())
}

fn ensure_layout(profile: &ConversationProfile) -> Result<PathBuf, ConversationImportError> {
    validate_profile(profile)?;
    let root = profile.config_dir.clone();
    ensure_path(&root)?;
    let directory = index_dir(profile);
    ensure_dir_chain(&root, &directory)?;
    Ok(directory)
}

/// Encodes a working directory the way Claude Code names `projects/` entries: every `/` and `.`
/// becomes `-`, so `/home/user/app.v2` is stored as `-home-user-app-v2`.
fn project_directory(cwd: &str) -> Result<String, ConversationImportError> {
    let path = Path::new(cwd);
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(ConversationImportError::InvalidIndex);
    }
    let name = cwd.replace(['/', '.'], "-");
    if name.is_empty() {
        return Err(ConversationImportError::InvalidIndex);
    }
    Ok(name)
}

fn transcript_path(
    profile: &ConversationProfile,
    record: &Record,
) -> Result<PathBuf, ConversationImportError> {
    let dir = profile
        .cli_config_dir
        .join("projects")
        .join(&record.project_directory);
    if !dir.starts_with(&profile.cli_config_dir) {
        return Err(ConversationImportError::InvalidProfile);
    }
    Ok(dir.join(format!("{}.jsonl", record.session.session_id)))
}

/// Resolves where the destination should read the transcript: the same relative path under the
/// destination's `projects/`. A shared CLI root therefore reuses the original in place, and a
/// separate root receives the copy where the Desktop looks for it.
fn destination_transcript(
    destination: &ConversationProfile,
    source_transcript: &Path,
) -> Result<PathBuf, ConversationImportError> {
    let project = source_transcript
        .parent()
        .and_then(Path::file_name)
        .ok_or(ConversationImportError::InvalidProfile)?;
    let file = source_transcript
        .file_name()
        .ok_or(ConversationImportError::InvalidProfile)?;
    Ok(destination
        .cli_config_dir
        .join("projects")
        .join(project)
        .join(file))
}

enum TranscriptState {
    Missing,
    Recent,
    Ready(PathBuf),
}

fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

/// Finds the one project directory under `projects/` holding this session's transcript, for
/// working directories whose encoding differs from `project_directory`.
fn find_transcript_by_scan(profile: &ConversationProfile, session_id: &str) -> Option<PathBuf> {
    let projects = profile.cli_config_dir.join("projects");
    let file_name = format!("{session_id}.jsonl");
    let mut found = None;
    let mut ambiguous = false;
    for entry in fs::read_dir(projects).ok()?.flatten() {
        if !entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
        {
            continue;
        }
        let candidate = entry.path().join(&file_name);
        if is_regular_file(&candidate) {
            ambiguous |= found.replace(candidate).is_some();
        }
    }
    if ambiguous { None } else { found }
}

/// Resolves the source transcript, keeping its real location relative to `projects/` so the
/// destination copy lands where the Desktop looks for it.
fn locate_transcript(profile: &ConversationProfile, record: &Record) -> Option<PathBuf> {
    transcript_path(profile, record)
        .ok()
        .filter(|path| is_regular_file(path))
        .or_else(|| find_transcript_by_scan(profile, &record.session.session_id))
}

fn local_transcript_state(profile: &ConversationProfile, record: &Record) -> TranscriptState {
    let Some(path) = locate_transcript(profile, record) else {
        return TranscriptState::Missing;
    };
    if transcript_recently_modified(&path) {
        TranscriptState::Recent
    } else {
        TranscriptState::Ready(path)
    }
}

fn local_availability(source: &ConversationProfile, record: &Record) -> ConversationAvailability {
    if record.session.availability == ConversationAvailability::Active {
        return ConversationAvailability::Active;
    }
    match local_transcript_state(source, record) {
        TranscriptState::Ready(_) => ConversationAvailability::Available,
        TranscriptState::Recent => ConversationAvailability::Active,
        TranscriptState::Missing => ConversationAvailability::Unavailable,
    }
}

fn destination_contains(
    profile: &ConversationProfile,
    record: &Record,
) -> Result<bool, ConversationImportError> {
    for existing in records(profile)? {
        if existing.identity == record.identity {
            return Ok(true);
        }
    }
    Ok(false)
}

fn manifest_key(record: &Record) -> String {
    format!("{}:{}", record.session.source_profile_id, record.identity)
}

/// Copies the transcript atomically: the destination only ever sees a complete file, and a failed
/// write leaves no partial transcript behind.
fn write_transcript_copy(source: &Path, destination: &Path) -> Result<(), ConversationImportError> {
    let metadata =
        fs::symlink_metadata(source).map_err(|_| ConversationImportError::StorageUnavailable)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ConversationImportError::StorageUnavailable);
    }
    let parent = destination
        .parent()
        .ok_or(ConversationImportError::InvalidProfile)?;
    ensure_path(parent)?;
    let mut input = File::open(source).map_err(|_| ConversationImportError::StorageUnavailable)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| ConversationImportError::StorageUnavailable)?;
    let copied = std::io::copy(&mut input, &mut temp)
        .and_then(|_| temp.as_file().sync_all())
        .and_then(|_| {
            temp.persist_noclobber(destination)
                .map_err(std::io::Error::other)
        });
    copied.map_err(|_| ConversationImportError::StorageUnavailable)?;
    Ok(())
}

fn availability_reason(availability: ConversationAvailability) -> Option<&'static str> {
    match availability {
        ConversationAvailability::Active => Some("The conversation is still running."),
        ConversationAvailability::Unavailable => {
            Some("The transcript or working directory is unavailable.")
        }
        ConversationAvailability::Failed => Some("The conversation could not be written."),
        ConversationAvailability::Ignored => Some("The destination already has this conversation."),
        ConversationAvailability::Available | ConversationAvailability::Imported => None,
    }
}

fn result(
    id: &str,
    availability: ConversationAvailability,
    message: Option<String>,
) -> ConversationImportResult {
    ConversationImportResult {
        session_id: id.into(),
        availability,
        message: message.or_else(|| availability_reason(availability).map(str::to_owned)),
    }
}

fn failed(id: &str, message: &str) -> ConversationImportResult {
    result(id, ConversationAvailability::Failed, Some(message.into()))
}

/// Outcome of a single import, before it is mapped to the caller's result.
enum ImportOutcome {
    Imported,
    /// Already in the destination index, previously imported, or recorded as a newer destination
    /// version. Carries the specific reason.
    Ignored(&'static str),
    Unavailable(&'static str),
}

/// A failed import with the detail the caller should surface. The status contract is unchanged:
/// the caller still reports `Failed` with the generic message; only an incomplete rollback
/// replaces the message, naming what is still on disk.
#[derive(Debug)]
struct ImportFailure {
    detail: Option<String>,
}

impl From<ConversationImportError> for ImportFailure {
    fn from(_: ConversationImportError) -> Self {
        Self { detail: None }
    }
}

impl ImportFailure {
    /// Rollback detail when present; otherwise the generic failure reason.
    fn message(&self) -> &str {
        self.detail
            .as_deref()
            .unwrap_or("Conversation could not be written.")
    }
}

/// Removes a file the import itself created. "Already gone" counts as success, so a leftover
/// is the only outcome worth reporting.
fn discard_created(path: &Path) -> Result<(), std::io::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn import_record(
    source: &ConversationProfile,
    destination: &ConversationProfile,
    record: &Record,
    remote: &dyn RemoteConversationValidator,
) -> Result<ImportOutcome, ImportFailure> {
    if !remote.can_write() {
        return Err(ConversationImportError::StorageUnavailable.into());
    }
    if destination_contains(destination, record)? {
        return Ok(ImportOutcome::Ignored(
            "The destination already has this conversation.",
        ));
    }
    let mut manifest = load_manifest(destination)?;
    let key = manifest_key(record);
    if manifest.imported.contains(&key) {
        return Ok(ImportOutcome::Ignored(
            "This conversation was imported before and is no longer in the destination index; the deletion is preserved.",
        ));
    }
    let destination_dir = ensure_layout(destination)?;
    let destination_index = destination_dir.join(&record.relative_index_path);
    let mut created_transcript: Option<PathBuf> = None;
    if record.session.source_kind == ConversationSourceKind::Local {
        let source_transcript = match local_transcript_state(source, record) {
            TranscriptState::Ready(path) => path,
            TranscriptState::Recent => return Ok(ImportOutcome::Unavailable(ACTIVE_REASON)),
            TranscriptState::Missing => return Ok(ImportOutcome::Unavailable(TRANSCRIPT_MISSING)),
        };
        let target_transcript = destination_transcript(destination, &source_transcript)?;
        if source_transcript != target_transcript {
            if let Err(error) = write_transcript_copy(&source_transcript, &target_transcript) {
                if is_regular_file(&target_transcript) {
                    return Ok(ImportOutcome::Ignored(
                        "The destination already has this transcript.",
                    ));
                }
                return Err(error.into());
            }
            created_transcript = Some(target_transcript);
        }
    }
    let bytes = serde_json::to_vec(&record.session.metadata)
        .map_err(|_| ConversationImportError::InvalidIndex)?;
    if let Err(error) = atomic_write(&destination_index, &bytes, true) {
        if let Some(path) = &created_transcript {
            let _ = discard_created(path);
        }
        if error == ConversationImportError::StorageUnavailable && destination_index.exists() {
            return Ok(ImportOutcome::Ignored(
                "The destination already has this conversation.",
            ));
        }
        return Err(error.into());
    }
    // From here on any failure rolls back the index entry: without the manifest, a later deletion
    // in the destination would not be protected from re-import.
    manifest.imported.push(key);
    manifest.imported.sort();
    manifest.imported.dedup();
    let manifest_path = destination.config_dir.join(IMPORT_MANIFEST);
    let manifest_bytes =
        serde_json::to_vec(&manifest).map_err(|_| ConversationImportError::InvalidIndex)?;
    if let Err(error) = atomic_write(&manifest_path, &manifest_bytes, false) {
        // Only the files this import created are removed; nothing else is touched. A rollback
        // that could not complete is reported alongside the original failure, listing what is
        // still on disk.
        let mut leftover = Vec::new();
        if discard_created(&destination_index).is_err() {
            leftover.push(destination_index.display().to_string());
        }
        if let Some(path) = &created_transcript
            && discard_created(path).is_err()
        {
            leftover.push(path.display().to_string());
        }
        let detail = (!leftover.is_empty()).then(|| {
            format!(
                "manifest write failed: {error:?}; rollback incomplete, left on disk: {}",
                leftover.join(", ")
            )
        });
        return Err(ImportFailure { detail });
    }
    Ok(ImportOutcome::Imported)
}

const ACTIVE_REASON: &str = "The conversation is still running.";
const TRANSCRIPT_MISSING: &str = "The transcript or working directory is unavailable.";

pub fn list_sessions(
    source: &ConversationProfile,
    destination: &ConversationProfile,
    remote: &dyn RemoteConversationValidator,
) -> Result<Vec<ConversationSession>, ConversationImportError> {
    validate_profile(source)?;
    validate_profile(destination)?;
    let mut records = records(source)?;
    let refs: Vec<_> = records
        .iter()
        .filter_map(|record| record.remote.clone())
        .collect();
    let statuses = remote.validate_sessions(&refs);
    for (record, status) in records
        .iter_mut()
        .filter(|record| record.remote.is_some())
        .zip(statuses)
    {
        record.session.availability =
            if record.session.availability == ConversationAvailability::Active {
                ConversationAvailability::Active
            } else {
                status.availability
            };
        record.session.registered_host_name = status.registered_host_name;
    }
    Ok(records
        .into_iter()
        .map(|mut record| {
            if record.remote.is_none() {
                record.session.availability = local_availability(source, &record);
            }
            record.session
        })
        .collect())
}

pub fn import_selected(
    source: &ConversationProfile,
    destination: &ConversationProfile,
    selected_ids: &[String],
    remote: &dyn RemoteConversationValidator,
) -> Vec<ConversationImportResult> {
    // Disabled or cancelled import: nothing is read, validated or written on either side.
    if selected_ids.is_empty() {
        return Vec::new();
    }
    let found = match records(source) {
        Ok(records) => records,
        Err(_) => {
            return selected_ids
                .iter()
                .map(|id| failed(id, "Conversation source is unavailable."))
                .collect();
        }
    };
    let refs: Vec<_> = found
        .iter()
        .filter_map(|record| record.remote.clone())
        .collect();
    let statuses = remote.validate_sessions(&refs);
    let mut remote_statuses = std::collections::HashMap::new();
    for (record, status) in found
        .iter()
        .filter(|record| record.remote.is_some())
        .zip(statuses)
    {
        remote_statuses.insert(record.session.id.clone(), status);
    }
    selected_ids
        .iter()
        .map(|id| {
            let Some(record) = found.iter().find(|record| record.session.id == *id) else {
                return failed(id, "Conversation is no longer available.");
            };
            let availability = if record.session.source_kind == ConversationSourceKind::Ssh {
                remote_statuses
                    .get(id)
                    .map_or(ConversationAvailability::Unavailable, |status| {
                        status.availability
                    })
            } else {
                local_availability(source, record)
            };
            if availability != ConversationAvailability::Available {
                return result(id, availability, None);
            }
            match import_record(source, destination, record, remote) {
                Ok(ImportOutcome::Imported) => result(id, ConversationAvailability::Imported, None),
                Ok(ImportOutcome::Ignored(reason)) => {
                    result(id, ConversationAvailability::Ignored, Some(reason.into()))
                }
                Ok(ImportOutcome::Unavailable(reason)) => result(
                    id,
                    ConversationAvailability::Unavailable,
                    Some(reason.into()),
                ),
                Err(failure) => failed(id, failure.message()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// The rollback helper must only report a real leftover: a file it already removed (or that
    /// never existed) is not a leftover, while anything it cannot remove is, so the caller can
    /// name the path that is still on disk.
    #[test]
    fn discard_created_reports_only_files_it_could_not_remove() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("index.json");
        fs::write(&file, "{}").unwrap();
        assert!(discard_created(&file).is_ok());
        assert!(
            discard_created(&file).is_ok(),
            "already gone is not a leftover"
        );

        let directory = dir.path().join("projects");
        fs::create_dir(&directory).unwrap();
        assert!(
            discard_created(&directory).is_err(),
            "a directory is never removed"
        );
    }
}
