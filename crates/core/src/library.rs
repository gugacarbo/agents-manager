use crate::{config::detect_secrets, domain::*, error::CoreError, structured};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemMeta {
    id: String,
    category: ConfigCategory,
    name: String,
    source_path: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Link {
    item_id: String,
    target_path: String,
    agent: AgentKind,
    scope: ConfigScope,
    project_id: Option<String>,
    strategy: SyncStrategy,
    #[serde(default)]
    source_path: String,
    #[serde(default)]
    skill: bool,
    last_synced_sha256: String,
    synced_at: String,
}
fn io(message: impl Into<String>) -> CoreError {
    CoreError::IoError {
        message: message.into(),
    }
}
fn not_found(id: &str) -> CoreError {
    CoreError::NotFound { target: id.into() }
}
fn category_name(c: ConfigCategory) -> &'static str {
    match c {
        ConfigCategory::Mcps => "mcps",
        ConfigCategory::Skills => "skills",
        ConfigCategory::Rules => "rules",
        ConfigCategory::Settings => "settings",
    }
}
fn slug(name: &str) -> Result<String, CoreError> {
    let s = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        return Err(io("name must contain a letter or digit"));
    }
    Ok(s)
}
fn read_json<T: serde::de::DeserializeOwned>(p: &Path) -> Result<Vec<T>, CoreError> {
    match fs::read(p) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
        Err(e) => Err(e.into()),
    }
}
fn atomic(p: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let parent = p.parent().ok_or_else(|| io("missing parent"))?;
    fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(bytes)?;
    f.as_file().sync_all()?;
    f.persist(p).map_err(|e| CoreError::from(e.error))?;
    Ok(())
}
fn write_json<T: Serialize>(p: &Path, v: &T) -> Result<(), CoreError> {
    atomic(p, &serde_json::to_vec_pretty(v)?)
}
fn metas(dir: &Path) -> Result<Vec<ItemMeta>, CoreError> {
    read_json(&dir.join("index.json"))
}
fn links(dir: &Path) -> Result<Vec<Link>, CoreError> {
    read_json(&dir.join("sync-state.json"))
}
fn get(dir: &Path, id: &str) -> Result<ItemMeta, CoreError> {
    metas(dir)?
        .into_iter()
        .find(|x| x.id == id)
        .ok_or_else(|| not_found(id))
}
fn hash(s: &[u8]) -> String {
    format!("{:x}", Sha256::digest(s))
}
fn target_hash(path: &Path, skill: bool) -> Result<Option<String>, CoreError> {
    if skill {
        let folder = path.parent().ok_or_else(|| io("missing skill target"))?;
        let real = match fs::canonicalize(folder) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut digest = Sha256::new();
        fingerprint_path(&real, &mut digest)?;
        Ok(Some(format!("{:x}", digest.finalize())))
    } else {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(hash(&bytes))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
fn status(link: &Link) -> SyncTargetStatus {
    let p = Path::new(&link.target_path);
    if (fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())
        || p.parent()
            .is_some_and(|p| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink())))
        && !p.exists()
    {
        return SyncTargetStatus::BrokenSymlink;
    }
    match target_hash(p, link.skill) {
        Ok(Some(digest)) if digest == link.last_synced_sha256 => SyncTargetStatus::InSync,
        Ok(Some(_)) => SyncTargetStatus::Drifted,
        _ => SyncTargetStatus::NotLinked,
    }
}
fn skill_has_secrets(path: &Path) -> Result<bool, CoreError> {
    let mut found = false;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_symlink() {
            return Err(io("skill auxiliary symlinks are not supported"));
        }
        if ty.is_dir() {
            found |= skill_has_secrets(&entry.path())?;
        } else if ty.is_file() {
            match fs::read_to_string(entry.path()) {
                Ok(text) => found |= !detect_secrets(&text).is_empty(),
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => (),
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(found)
}
fn item(_dir: &Path, meta: &ItemMeta, all: &[Link]) -> Result<LibraryItem, CoreError> {
    let content = match fs::read_to_string(&meta.source_path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let has_literal_secrets = if meta.category == ConfigCategory::Skills {
        let parent = Path::new(&meta.source_path)
            .parent()
            .ok_or_else(|| io("missing skill directory"))?;
        if parent.exists() {
            skill_has_secrets(parent)?
        } else {
            false
        }
    } else {
        !detect_secrets(&content).is_empty()
    };
    Ok(LibraryItem {
        id: meta.id.clone(),
        category: meta.category,
        name: meta.name.clone(),
        source_path: meta.source_path.clone(),
        has_literal_secrets,
        targets: all
            .iter()
            .filter(|l| l.item_id == meta.id)
            .map(|l| SyncTarget {
                agent: l.agent,
                scope: l.scope,
                project_id: l.project_id.clone(),
                path: l.target_path.clone(),
                status: status(l),
            })
            .collect(),
    })
}
pub fn list_items(dir: &Path) -> Result<Vec<LibraryItem>, CoreError> {
    let links = links(dir)?;
    metas(dir)?.iter().map(|m| item(dir, m, &links)).collect()
}
pub fn read_item(dir: &Path, id: &str) -> Result<LibraryItemContent, CoreError> {
    let meta = get(dir, id)?;
    let content = fs::read_to_string(meta.source_path)?;
    let secret_spans = detect_secrets(&content);
    Ok(LibraryItemContent {
        content,
        secret_spans,
    })
}
fn path_for(dir: &Path, c: ConfigCategory, s: &str) -> PathBuf {
    match c {
        ConfigCategory::Skills => dir.join("skills").join(s).join("SKILL.md"),
        ConfigCategory::Rules => dir.join("rules").join(format!("{s}.md")),
        _ => dir.join(category_name(c)).join(format!("{s}.json")),
    }
}
pub fn save_item(dir: &Path, request: &SaveLibraryItemRequest) -> Result<LibraryItem, CoreError> {
    if !detect_secrets(&request.content).is_empty() && request.confirm_literal_secrets != Some(true)
    {
        return Err(CoreError::SecretConfirmationRequired);
    }
    if matches!(
        request.category,
        ConfigCategory::Mcps | ConfigCategory::Settings
    ) {
        let value: serde_json::Value =
            serde_json::from_str(&request.content).map_err(|e| CoreError::SyntaxError {
                message: e.to_string(),
            })?;
        if !value.is_object() {
            return Err(io("library JSON must be an object"));
        }
    }
    let mut all = metas(dir)?;
    let meta = if let Some(id) = &request.item_id {
        let mut m = all
            .iter()
            .find(|m| &m.id == id)
            .cloned()
            .ok_or_else(|| not_found(id))?;
        if m.category != request.category {
            return Err(io("cannot change item category"));
        }
        m.name = request.name.clone();
        m
    } else {
        let slug = slug(&request.name)?;
        let id = format!("{}--{slug}", category_name(request.category));
        if all.iter().any(|m| m.id == id) {
            return Err(io("library item already exists"));
        }
        ItemMeta {
            id,
            category: request.category,
            name: request.name.clone(),
            source_path: path_for(dir, request.category, &slug)
                .to_string_lossy()
                .into_owned(),
        }
    };
    atomic(Path::new(&meta.source_path), request.content.as_bytes())?;
    all.retain(|m| m.id != meta.id);
    all.push(meta.clone());
    write_json(&dir.join("index.json"), &all)?;
    item(dir, &meta, &links(dir)?)
}
fn copy_aux(source: &Path, target: &Path, root: bool) -> Result<(), CoreError> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = target.join(entry.file_name());
        if ty.is_dir() {
            copy_aux(&entry.path(), &dest, false)?;
        } else if ty.is_file() && (!root || entry.file_name() != "SKILL.md") {
            fs::copy(entry.path(), dest)?;
        } else if ty.is_symlink() {
            return Err(io("skill auxiliary symlinks are not supported"));
        }
    }
    Ok(())
}
pub fn promote(
    dir: &Path,
    entry: ConfigFileEntry,
    name: &str,
    confirm: Option<bool>,
) -> Result<LibraryItem, CoreError> {
    let mut content = fs::read_to_string(&entry.path)?;
    if entry.category == ConfigCategory::Skills {
        let source = Path::new(&entry.path)
            .parent()
            .ok_or_else(|| io("missing skill directory"))?;
        if skill_has_secrets(source)? && confirm != Some(true) {
            return Err(CoreError::SecretConfirmationRequired);
        }
    }
    if matches!(
        entry.category,
        ConfigCategory::Mcps | ConfigCategory::Settings
    ) {
        let mut v: serde_json::Value = match entry.format {
            ConfigFormat::Toml => {
                let t: toml::Value =
                    content
                        .parse::<toml::Value>()
                        .map_err(|e| CoreError::SyntaxError {
                            message: e.to_string(),
                        })?;
                serde_json::to_value(t)?
            }
            ConfigFormat::Json => {
                serde_json::from_str(&content).map_err(|e| CoreError::SyntaxError {
                    message: e.to_string(),
                })?
            }
            ConfigFormat::Markdown => return Err(io("invalid structured format")),
        };
        if entry.category == ConfigCategory::Mcps {
            let object = v.as_object_mut().ok_or_else(|| io("expected object"))?;
            let servers = object
                .remove("mcpServers")
                .or_else(|| object.remove("mcp_servers"))
                .unwrap_or_else(|| serde_json::json!({}));
            v = serde_json::json!({"mcpServers":servers});
        }
        content = serde_json::to_string_pretty(&v)?;
    }
    let result = save_item(
        dir,
        &SaveLibraryItemRequest {
            item_id: None,
            category: entry.category,
            name: name.into(),
            content,
            confirm_literal_secrets: confirm,
        },
    )?;
    if entry.category == ConfigCategory::Skills {
        let source = Path::new(&entry.path)
            .parent()
            .ok_or_else(|| io("missing skill directory"))?;
        let target = Path::new(&result.source_path)
            .parent()
            .ok_or_else(|| io("missing skill directory"))?;
        copy_aux(source, target, true)?;
    }
    let meta = get(dir, &result.id)?;
    item(dir, &meta, &links(dir)?)
}
pub fn delete_item(dir: &Path, id: &str) -> Result<(), CoreError> {
    let meta = get(dir, id)?;
    let path = Path::new(&meta.source_path);
    if meta.category == ConfigCategory::Skills {
        let parent = path.parent().ok_or_else(|| io("missing skill directory"))?;
        match fs::remove_dir_all(parent) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    } else {
        match fs::remove_file(path) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    let mut all = metas(dir)?;
    all.retain(|m| m.id != id);
    write_json(&dir.join("index.json"), &all)
}

fn fingerprint_path(path: &Path, hasher: &mut Sha256) -> Result<(), CoreError> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Err(io("symlinks are not allowed in library items"));
    }
    if meta.is_dir() {
        let mut entries = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            hasher.update(entry.file_name().to_string_lossy().as_bytes());
            fingerprint_path(&entry.path(), hasher)?;
        }
    } else {
        hasher.update(fs::read(path)?);
    }
    Ok(())
}
pub fn item_fingerprint(dir: &Path, id: &str) -> Result<String, CoreError> {
    let m = get(dir, id)?;
    let path = Path::new(&m.source_path);
    let source = if m.category == ConfigCategory::Skills {
        path.parent().ok_or_else(|| io("missing skill directory"))?
    } else {
        path
    };
    let mut digest = Sha256::new();
    fingerprint_path(source, &mut digest)?;
    Ok(format!("{:x}", digest.finalize()))
}
fn link_path(meta: &ItemMeta, target: &Path) -> Result<(PathBuf, PathBuf), CoreError> {
    let source = Path::new(&meta.source_path);
    if meta.category == ConfigCategory::Skills {
        Ok((
            target
                .parent()
                .ok_or_else(|| io("missing skill target"))?
                .to_path_buf(),
            source
                .parent()
                .ok_or_else(|| io("missing skill source"))?
                .to_path_buf(),
        ))
    } else {
        Ok((target.to_path_buf(), source.to_path_buf()))
    }
}
pub fn is_managed_target(dir: &Path, path: &Path) -> Result<bool, CoreError> {
    let target = path.to_string_lossy();
    let state = links(dir)?;
    for link in state
        .iter()
        .filter(|l| l.target_path == target && l.strategy == SyncStrategy::Symlink)
    {
        let actual = if link.skill {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        let source = Path::new(&link.source_path);
        let expected = if link.skill {
            source.parent().unwrap_or(source)
        } else {
            source
        };
        if !link.source_path.is_empty() && fs::read_link(actual).is_ok_and(|v| v == expected) {
            return Ok(true);
        }
    }
    Ok(false)
}
fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}
fn checked_target(dir: &Path, meta: &ItemMeta, path: &Path) -> Result<(), CoreError> {
    let (actual, _) = link_path(meta, path)?;
    if (is_symlink(path) || is_symlink(&actual)) && !is_managed_target(dir, path)? {
        return Err(CoreError::PathNotAllowed {
            path: path.display().to_string(),
        });
    }
    Ok(())
}
fn proposed(
    meta: &ItemMeta,
    target: &ResolvedSyncTarget,
    current: Option<&str>,
) -> Result<String, CoreError> {
    let source = fs::read_to_string(&meta.source_path)?;
    if !matches!(
        meta.category,
        ConfigCategory::Mcps | ConfigCategory::Settings
    ) {
        return Ok(source);
    }
    let mut patch: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&source)
        .map_err(|e| CoreError::SyntaxError {
            message: e.to_string(),
        })?;
    if meta.category == ConfigCategory::Mcps
        && target.entry.format == ConfigFormat::Toml
        && let Some(servers) = patch.remove("mcpServers")
    {
        patch.insert("mcp_servers".into(), servers);
    }
    let req = StructuredSaveRequest {
        path: target.entry.path.clone(),
        expected_sha256: String::new(),
        mcp_servers: None,
        settings_patch: Some(patch),
        force_overwrite: None,
    };
    match target.entry.format {
        ConfigFormat::Json => structured::json_content(current.unwrap_or("{}"), &req),
        ConfigFormat::Toml => structured::toml_content(current.unwrap_or(""), &req),
        ConfigFormat::Markdown => Err(io("unsupported merge format")),
    }
}
pub fn preview_sync(
    dir: &Path,
    id: &str,
    targets: &[ResolvedSyncTarget],
) -> Result<Vec<SyncPreviewItem>, CoreError> {
    let meta = get(dir, id)?;
    let mut out = Vec::new();
    for target in targets {
        if target.entry.category != meta.category {
            return Err(io("target category mismatch"));
        }
        let path = Path::new(&target.entry.path);
        checked_target(dir, &meta, path)?;
        let current = match fs::read_to_string(path) {
            Ok(s) => Some(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let strategy = if matches!(
            meta.category,
            ConfigCategory::Mcps | ConfigCategory::Settings
        ) {
            SyncStrategy::Merge
        } else if target.copy {
            SyncStrategy::Copy
        } else {
            SyncStrategy::Symlink
        };
        let content = proposed(&meta, target, current.as_deref())?;
        let expected_sha256 = target_hash(path, meta.category == ConfigCategory::Skills)?;
        let (actual, source) = link_path(&meta, path)?;
        let linked = fs::read_link(&actual).is_ok_and(|p| p == source);
        let changed = if strategy == SyncStrategy::Symlink {
            !linked
        } else if meta.category == ConfigCategory::Skills {
            expected_sha256.as_deref() != Some(item_fingerprint(dir, id)?.as_str())
        } else {
            current.as_deref() != Some(content.as_str())
        };
        let mut warnings = Vec::new();
        if strategy == SyncStrategy::Symlink && actual.exists() && !linked {
            warnings.push("Target exists; choose copy explicitly to replace it".into());
        }
        out.push(SyncPreviewItem {
            target_path: target.entry.path.clone(),
            agent: target.entry.agent,
            scope: target.scope,
            strategy,
            current_secret_spans: current.as_deref().map(detect_secrets).unwrap_or_default(),
            proposed_secret_spans: detect_secrets(&content),
            current_content: current,
            proposed_content: content,
            expected_sha256,
            changed,
            warnings,
        });
    }
    Ok(out)
}
fn backup_file(dir: &Path, target: &Path) -> Result<Option<String>, CoreError> {
    let bytes = match fs::read(target) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    fs::create_dir_all(dir)?;
    let prefix = hash(target.to_string_lossy().as_bytes());
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| io(e.to_string()))?
        .as_nanos();
    let mut f = tempfile::Builder::new()
        .prefix(&format!("{prefix}-{nanos:039}-"))
        .tempfile_in(dir)?;
    f.write_all(&bytes)?;
    f.as_file().sync_all()?;
    let (_, path) = f.keep().map_err(|e| CoreError::from(e.error))?;
    let mut all = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&format!("{prefix}-"))
        })
        .map(|e| e.path())
        .collect::<Vec<_>>();
    all.sort();
    let excess = all.len().saturating_sub(20);
    for old in all.into_iter().take(excess) {
        fs::remove_file(old)?;
    }
    Ok(Some(path.to_string_lossy().into_owned()))
}
fn copy_skill_files(
    source: &Path,
    target: &Path,
    backups: &Path,
    root: bool,
) -> Result<(), CoreError> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let dest = target.join(entry.file_name());
        if kind.is_symlink() || is_symlink(&dest) {
            return Err(io("skill auxiliary symlinks are not supported"));
        }
        if kind.is_dir() {
            copy_skill_files(&entry.path(), &dest, backups, false)?;
        } else if kind.is_file() && (!root || entry.file_name() != "SKILL.md") {
            let bytes = fs::read(entry.path())?;
            backup_file(backups, &dest)?;
            atomic(&dest, &bytes)?;
        }
    }
    Ok(())
}
fn write_target(
    dir: &Path,
    backups: &Path,
    meta: &ItemMeta,
    target: &ResolvedSyncTarget,
    preview: &SyncPreviewItem,
) -> Result<Option<String>, CoreError> {
    let p = Path::new(&target.entry.path);
    checked_target(dir, meta, p)?;
    let current = target_hash(p, meta.category == ConfigCategory::Skills)?;
    if current != preview.expected_sha256 {
        return Err(CoreError::ExternalConflict);
    }
    if !preview.changed {
        return Ok(None);
    }
    let (actual, source) = link_path(meta, p)?;
    let parent = actual.parent().ok_or_else(|| io("missing target parent"))?;
    fs::create_dir_all(parent)?;
    if preview.strategy == SyncStrategy::Symlink {
        if fs::symlink_metadata(&actual).is_ok() {
            if fs::read_link(&actual).is_ok_and(|v| v == source) {
                return Ok(None);
            }
            return Err(io("target exists; explicit copy is required"));
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &actual)?;
        #[cfg(not(unix))]
        return Err(io("symlink unsupported; explicit copy required"));
        Ok(None)
    } else {
        if meta.category == ConfigCategory::Skills {
            if is_symlink(&actual) {
                return Err(io("unlink managed skill before copying"));
            }
            let backup = backup_file(backups, p)?;
            copy_skill_files(&source, &actual, backups, true)?;
            atomic(p, preview.proposed_content.as_bytes())?;
            return Ok(backup);
        }
        let backup = backup_file(backups, p)?;
        atomic(p, preview.proposed_content.as_bytes())?;
        Ok(backup)
    }
}
pub fn apply_sync(
    dir: &Path,
    backups: &Path,
    id: &str,
    targets: &[ResolvedSyncTarget],
    previews: &[SyncPreviewItem],
) -> Result<Vec<SyncApplyResult>, CoreError> {
    if targets.len() != previews.len() {
        return Err(io("preview required for every target"));
    }
    let meta = get(dir, id)?;
    let mut state = links(dir)?;
    let mut out = Vec::new();
    for (target, preview) in targets.iter().zip(previews) {
        let strategy = if matches!(
            meta.category,
            ConfigCategory::Mcps | ConfigCategory::Settings
        ) {
            SyncStrategy::Merge
        } else if target.copy {
            SyncStrategy::Copy
        } else {
            SyncStrategy::Symlink
        };
        let result = if preview.target_path != target.entry.path
            || preview.agent != target.entry.agent
            || preview.scope != target.scope
            || preview.strategy != strategy
        {
            Err(io("preview does not match target"))
        } else {
            write_target(dir, backups, &meta, target, preview)
        };
        match result {
            Ok(backup) => {
                let path = Path::new(&target.entry.path);
                let digest = target_hash(path, meta.category == ConfigCategory::Skills)?
                    .ok_or_else(|| io("missing synced target"))?;
                state.retain(|l| l.item_id != id || l.target_path != target.entry.path);
                state.push(Link {
                    item_id: id.into(),
                    target_path: target.entry.path.clone(),
                    agent: target.entry.agent,
                    scope: target.scope,
                    project_id: target.project_id.clone(),
                    strategy,
                    source_path: meta.source_path.clone(),
                    skill: meta.category == ConfigCategory::Skills,
                    last_synced_sha256: digest,
                    synced_at: chrono::Utc::now().to_rfc3339(),
                });
                write_json(&dir.join("sync-state.json"), &state)?;
                out.push(SyncApplyResult {
                    target_path: target.entry.path.clone(),
                    applied: true,
                    strategy_used: strategy,
                    backup_path: backup,
                    error: None,
                });
            }
            Err(error) => out.push(SyncApplyResult {
                target_path: target.entry.path.clone(),
                applied: false,
                strategy_used: strategy,
                backup_path: None,
                error: Some(error),
            }),
        }
    }
    Ok(out)
}
pub fn unlink(dir: &Path, id: &str, path: &Path) -> Result<(), CoreError> {
    let mut state = links(dir)?;
    let target = path.to_string_lossy();
    let entry = state
        .iter()
        .find(|l| l.item_id == id && l.target_path == target)
        .cloned()
        .ok_or_else(|| not_found(&target))?;
    if entry.strategy == SyncStrategy::Symlink && is_managed_target(dir, path)? {
        let actual = if entry.skill {
            path.parent().ok_or_else(|| io("missing skill target"))?
        } else {
            path
        };
        fs::remove_file(actual)?;
    }
    state.retain(|l| l.item_id != id || l.target_path != target);
    write_json(&dir.join("sync-state.json"), &state)
}
