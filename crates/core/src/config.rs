use crate::{
    domain::{ConfigDocument, ConfigFileEntry, SaveConfigRequest, SaveConfigResult, SecretSpan},
    error::CoreError,
};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn disk_sha256(path: &Path) -> Result<String, CoreError> {
    Ok(hash(&fs::read(path)?))
}

pub fn read_config(entry: ConfigFileEntry) -> Result<ConfigDocument, CoreError> {
    let content = fs::read_to_string(&entry.path)?;
    let sha256 = hash(content.as_bytes());
    let secret_spans = detect_secrets(&content);
    Ok(ConfigDocument {
        entry,
        content,
        sha256,
        secret_spans,
    })
}

pub(crate) fn detect_secrets(content: &str) -> Vec<SecretSpan> {
    let Ok(assignments) = Regex::new(
        r#"(?i)["']?([a-z0-9_.-]+)["']?\s*[:=]\s*(?:"((?:\\.|[^"\\\r\n])*)"|'([^'\r\n]*)')"#,
    ) else {
        return vec![];
    };
    let Ok(objects) = Regex::new(r#"(?i)["'](?:env|headers)["']\s*:\s*\{([^{}]*)\}"#) else {
        return vec![];
    };
    let Ok(sections) = Regex::new(r"(?im)^\s*\[(?:[^\]\r\n]*[.])?(?:env|headers)\]\s*$") else {
        return vec![];
    };
    let mut ranges: Vec<(usize, usize)> = objects
        .captures_iter(content)
        .filter_map(|c| c.get(1).map(|m| (m.start(), m.end())))
        .collect();
    let Ok(any_section) = Regex::new(r"(?m)^\s*\[[^\]\r\n]+\]\s*$") else {
        return vec![];
    };
    for section in sections.find_iter(content) {
        let end = any_section
            .find_at(content, section.end())
            .map_or(content.len(), |m| m.start());
        ranges.push((section.end(), end));
    }
    assignments
        .captures_iter(content)
        .filter_map(|capture| {
            let key = capture.get(1)?.as_str().to_lowercase();
            let value = capture.get(2).or_else(|| capture.get(3))?;
            let sensitive = [
                "token",
                "secret",
                "password",
                "passwd",
                "api_key",
                "api-key",
                "apikey",
                "authorization",
                "credential",
            ]
            .iter()
            .any(|needle| key.contains(needle))
                || ranges
                    .iter()
                    .any(|(start, end)| value.start() >= *start && value.end() <= *end);
            if !sensitive || (value.as_str().starts_with("${") && value.as_str().ends_with('}')) {
                return None;
            }
            Some(SecretSpan {
                start_byte: u32::try_from(value.start()).ok()?,
                end_byte: u32::try_from(value.end()).ok()?,
                kind: key,
            })
        })
        .collect()
}

fn validate(path: &Path, content: &str) -> Result<(), CoreError> {
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    match extension.to_ascii_lowercase().as_str() {
        "json" => {
            serde_json::from_str::<serde_json::Value>(content).map_err(|e| {
                CoreError::SyntaxError {
                    message: e.to_string(),
                }
            })?;
        }
        "toml" => {
            content
                .parse::<toml::Table>()
                .map_err(|e| CoreError::SyntaxError {
                    message: e.message().to_owned(),
                })?;
        }
        _ => {}
    }
    Ok(())
}

pub fn save_config(
    request: &SaveConfigRequest,
    backups_dir: &Path,
) -> Result<SaveConfigResult, CoreError> {
    let path = Path::new(&request.path);
    validate(path, &request.content)?;
    let original = fs::read(path)?;
    if request.force_overwrite != Some(true) && hash(&original) != request.expected_sha256 {
        return Err(CoreError::ExternalConflict);
    }
    let parent = path.parent().ok_or_else(|| CoreError::InvalidPath {
        path: request.path.clone(),
    })?;
    let permissions = fs::metadata(path)?.permissions();
    fs::create_dir_all(backups_dir)?;
    let prefix = hash(request.path.as_bytes());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| CoreError::IoError {
            message: e.to_string(),
        })?
        .as_nanos();
    let mut backup = tempfile::Builder::new()
        .prefix(&format!("{prefix}-{nanos:039}-"))
        .tempfile_in(backups_dir)?;
    backup.write_all(&original)?;
    backup.as_file().sync_all()?;
    let (_, backup_path) = backup.keep().map_err(|e| CoreError::from(e.error))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.as_file().set_permissions(permissions)?;
    temp.write_all(request.content.as_bytes())?;
    temp.as_file().sync_all()?;
    // Check again after preparing the write, guarding against edits during backup creation.
    if request.force_overwrite != Some(true) && disk_sha256(path)? != request.expected_sha256 {
        let _ = fs::remove_file(&backup_path);
        return Err(CoreError::ExternalConflict);
    }
    temp.persist(path).map_err(|e| CoreError::from(e.error))?;
    let mut snapshots = fs::read_dir(backups_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(&format!("{prefix}-"))
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    snapshots.sort();
    let excess = snapshots.len().saturating_sub(20);
    for old in snapshots.into_iter().take(excess) {
        fs::remove_file(old)?;
    }
    Ok(SaveConfigResult {
        sha256: hash(request.content.as_bytes()),
        backup_path: backup_path.to_string_lossy().into_owned(),
    })
}
