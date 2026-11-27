#![allow(clippy::unwrap_used)]
use agents_manager_core::{
    domain::{
        AgentKind, ConfigCategory, ConfigFileEntry, ConfigFormat, ConfigScope, ResolvedSyncTarget,
        SaveLibraryItemRequest, SyncTargetStatus,
    },
    library,
};
use std::fs;
use tempfile::tempdir;
#[test]
fn literal_secrets_require_confirmation() {
    let tmp = tempdir().unwrap();
    let req = SaveLibraryItemRequest {
        item_id: None,
        category: ConfigCategory::Mcps,
        name: "server".into(),
        content: r#"{"mcpServers":{"x":{"env":{"TOKEN":"literal"}}}}"#.into(),
        confirm_literal_secrets: Some(false),
    };
    assert!(library::save_item(tmp.path(), &req).is_err());
    assert!(library::list_items(tmp.path()).unwrap().is_empty());
}
#[test]
fn rule_symlink_sync_and_broken_status() {
    let tmp = tempdir().unwrap();
    let lib = tmp.path().join("library");
    let dest = tmp.path().join("target.md");
    let item = library::save_item(
        &lib,
        &SaveLibraryItemRequest {
            item_id: None,
            category: ConfigCategory::Rules,
            name: "rule".into(),
            content: "hello".into(),
            confirm_literal_secrets: None,
        },
    )
    .unwrap();
    let target = ResolvedSyncTarget {
        entry: ConfigFileEntry {
            agent: AgentKind::Codex,
            category: ConfigCategory::Rules,
            label: "rule".into(),
            path: dest.display().to_string(),
            format: ConfigFormat::Markdown,
            exists: false,
        },
        scope: ConfigScope::Global,
        project_id: None,
        copy: false,
    };
    let previews = library::preview_sync(&lib, &item.id, std::slice::from_ref(&target)).unwrap();
    let results = library::apply_sync(
        &lib,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&target),
        &previews,
    )
    .unwrap();
    assert!(results[0].applied);
    assert!(
        fs::symlink_metadata(&dest)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        library::list_items(&lib).unwrap()[0].targets[0].status,
        SyncTargetStatus::InSync
    );
    library::delete_item(&lib, &item.id).unwrap();
    assert!(
        fs::symlink_metadata(&dest)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

fn target(
    path: &std::path::Path,
    category: ConfigCategory,
    format: ConfigFormat,
    copy: bool,
) -> ResolvedSyncTarget {
    ResolvedSyncTarget {
        entry: ConfigFileEntry {
            agent: AgentKind::Codex,
            category,
            label: "target".into(),
            path: path.display().to_string(),
            format,
            exists: path.exists(),
        },
        scope: ConfigScope::Global,
        project_id: None,
        copy,
    }
}
fn create(
    dir: &std::path::Path,
    category: ConfigCategory,
    content: &str,
) -> agents_manager_core::domain::LibraryItem {
    library::save_item(
        dir,
        &SaveLibraryItemRequest {
            item_id: None,
            category,
            name: "example".into(),
            content: content.into(),
            confirm_literal_secrets: Some(true),
        },
    )
    .unwrap()
}
#[test]
fn conflict_and_explicit_copy_fallback() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("library");
    let dest = tmp.path().join("rules.md");
    fs::write(&dest, "old").unwrap();
    let item = create(&dir, ConfigCategory::Rules, "new");
    let t = target(&dest, ConfigCategory::Rules, ConfigFormat::Markdown, false);
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    let r = library::apply_sync(
        &dir,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    assert!(!r[0].applied);
    assert_eq!(fs::read_to_string(&dest).unwrap(), "old");
    let t = target(&dest, ConfigCategory::Rules, ConfigFormat::Markdown, true);
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    fs::write(&dest, "external").unwrap();
    let r = library::apply_sync(
        &dir,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    assert!(matches!(
        r[0].error,
        Some(agents_manager_core::error::CoreError::ExternalConflict)
    ));
    assert_eq!(fs::read_to_string(&dest).unwrap(), "external");
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    let r = library::apply_sync(
        &dir,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    assert!(r[0].applied);
    assert_eq!(
        fs::read_to_string(r[0].backup_path.as_ref().unwrap()).unwrap(),
        "external"
    );
    let previews = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    assert!(!previews[0].changed);
    let unchanged = library::apply_sync(
        &dir,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&t),
        &previews,
    )
    .unwrap();
    assert!(unchanged[0].backup_path.is_none());
    fs::write(&dest, "drift").unwrap();
    assert_eq!(
        library::list_items(&dir).unwrap()[0].targets[0].status,
        SyncTargetStatus::Drifted
    );
}
#[test]
fn mcp_json_merges_into_toml_without_losing_unknowns_or_comments() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("lib");
    let dest = tmp.path().join("config.toml");
    fs::write(
        &dest,
        "# keep\n[mcp_servers.demo]\ncustom = true\ncommand = 'old'\n",
    )
    .unwrap();
    let item = create(
        &dir,
        ConfigCategory::Mcps,
        r#"{"mcpServers":{"demo":{"command":"new"}}}"#,
    );
    let t = target(&dest, ConfigCategory::Mcps, ConfigFormat::Toml, false);
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    assert!(p[0].proposed_content.contains("# keep"));
    assert!(p[0].proposed_content.contains("custom = true"));
    assert!(p[0].proposed_content.contains("new"));
}
#[test]
fn broken_skill_link_is_reported_and_auxiliaries_remain_linked() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("lib");
    let source = tmp.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "hello").unwrap();
    fs::write(source.join("extra.txt"), "extra").unwrap();
    let source_entry = target(
        &source.join("SKILL.md"),
        ConfigCategory::Skills,
        ConfigFormat::Markdown,
        false,
    )
    .entry;
    let item = library::promote(&dir, source_entry, "demo", None).unwrap();
    let dest = tmp.path().join("target/demo/SKILL.md");
    let t = target(&dest, ConfigCategory::Skills, ConfigFormat::Markdown, false);
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    let r = library::apply_sync(
        &dir,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    assert!(r[0].applied);
    assert_eq!(
        fs::read_to_string(dest.parent().unwrap().join("extra.txt")).unwrap(),
        "extra"
    );
    fs::remove_dir_all(std::path::Path::new(&item.source_path).parent().unwrap()).unwrap();
    assert_eq!(
        library::list_items(&dir).unwrap()[0].targets[0].status,
        SyncTargetStatus::BrokenSymlink
    );
}
#[test]
fn skill_copy_fallback_updates_existing_files_with_backups() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("lib");
    let source = tmp.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("SKILL.md"), "new").unwrap();
    fs::write(source.join("helper.txt"), "new helper").unwrap();
    let item = library::promote(
        &dir,
        target(
            &source.join("SKILL.md"),
            ConfigCategory::Skills,
            ConfigFormat::Markdown,
            false,
        )
        .entry,
        "demo",
        None,
    )
    .unwrap();
    let folder = tmp.path().join("target/demo");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("SKILL.md"), "old").unwrap();
    fs::write(folder.join("helper.txt"), "old helper").unwrap();
    let t = target(
        &folder.join("SKILL.md"),
        ConfigCategory::Skills,
        ConfigFormat::Markdown,
        true,
    );
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    let result = library::apply_sync(
        &dir,
        &tmp.path().join("backups"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    assert!(result[0].applied);
    assert_eq!(
        fs::read_to_string(folder.join("helper.txt")).unwrap(),
        "new helper"
    );
    assert_eq!(
        fs::read_to_string(result[0].backup_path.as_ref().unwrap()).unwrap(),
        "old"
    );
}
#[test]
fn skill_auxiliary_external_edit_after_preview_is_conflict() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("lib");
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("SKILL.md"), "new").unwrap();
    fs::write(src.join("aux"), "new").unwrap();
    let item = library::promote(
        &dir,
        target(
            &src.join("SKILL.md"),
            ConfigCategory::Skills,
            ConfigFormat::Markdown,
            false,
        )
        .entry,
        "demo",
        None,
    )
    .unwrap();
    let dest = tmp.path().join("dest");
    fs::create_dir(&dest).unwrap();
    fs::write(dest.join("SKILL.md"), "old").unwrap();
    fs::write(dest.join("aux"), "old").unwrap();
    let t = target(
        &dest.join("SKILL.md"),
        ConfigCategory::Skills,
        ConfigFormat::Markdown,
        true,
    );
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    fs::write(dest.join("aux"), "external").unwrap();
    let r = library::apply_sync(
        &dir,
        &tmp.path().join("backup"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    assert!(matches!(
        r[0].error,
        Some(agents_manager_core::error::CoreError::ExternalConflict)
    ));
    assert_eq!(fs::read_to_string(dest.join("aux")).unwrap(), "external");
}
#[test]
fn deleted_item_can_unlink_only_its_managed_symlink() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("lib");
    let dest = tmp.path().join("rule.md");
    let item = create(&dir, ConfigCategory::Rules, "content");
    let t = target(&dest, ConfigCategory::Rules, ConfigFormat::Markdown, false);
    let p = library::preview_sync(&dir, &item.id, std::slice::from_ref(&t)).unwrap();
    library::apply_sync(
        &dir,
        &tmp.path().join("backup"),
        &item.id,
        std::slice::from_ref(&t),
        &p,
    )
    .unwrap();
    library::delete_item(&dir, &item.id).unwrap();
    library::unlink(&dir, &item.id, &dest).unwrap();
    assert!(fs::symlink_metadata(&dest).is_err());
}
#[test]
fn promotion_rejects_literal_secrets_in_skill_auxiliary_files() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("SKILL.md"), "safe").unwrap();
    fs::write(src.join("config.json"), r#"{"env":{"TOKEN":"literal"}}"#).unwrap();
    let lib = tmp.path().join("lib");
    let res = library::promote(
        &lib,
        target(
            &src.join("SKILL.md"),
            ConfigCategory::Skills,
            ConfigFormat::Markdown,
            false,
        )
        .entry,
        "demo",
        None,
    );
    assert!(matches!(
        res,
        Err(agents_manager_core::error::CoreError::SecretConfirmationRequired)
    ));
    assert!(library::list_items(&lib).unwrap().is_empty());
}
#[test]
fn deleted_library_source_can_be_removed_from_index() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("lib");
    let item = create(&dir, ConfigCategory::Rules, "content");
    fs::remove_file(&item.source_path).unwrap();
    library::delete_item(&dir, &item.id).unwrap();
    assert!(library::list_items(&dir).unwrap().is_empty());
}
