#![allow(clippy::unwrap_used)]
use agents_manager_core::{
    domain::{
        AgentKind, ConfigCategory, ConfigFileEntry, ConfigFormat, McpEnvVar, McpServerConfig,
        McpTransport, StructuredSaveRequest,
    },
    error::CoreError,
    projects, structured,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn registry_canonicalizes_deduplicates_and_removes_without_touching_project() {
    let dir = tempdir().unwrap();
    let registry = dir.path().join("data/projects.json");
    let root = dir.path().join("project");
    fs::create_dir(&root).unwrap();
    assert!(projects::register_project(&registry, &root.join("missing")).is_err());
    assert!(!registry.exists());
    let project = projects::register_project(&registry, &root).unwrap();
    assert!(matches!(
        projects::register_project(&registry, &root.join(".")),
        Err(CoreError::DuplicateProject { .. })
    ));
    assert_eq!(projects::list_projects(&registry).unwrap().len(), 1);
    projects::remove_project(&registry, &project.id).unwrap();
    assert!(projects::list_projects(&registry).unwrap().is_empty());
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

fn entry(path: &std::path::Path, format: ConfigFormat) -> ConfigFileEntry {
    ConfigFileEntry {
        agent: AgentKind::Codex,
        category: ConfigCategory::Mcps,
        label: "mcp".into(),
        path: path.display().to_string(),
        format,
        exists: true,
    }
}
fn request(path: &std::path::Path, hash: String) -> StructuredSaveRequest {
    StructuredSaveRequest {
        path: path.display().to_string(),
        expected_sha256: hash,
        force_overwrite: None,
        settings_patch: None,
        mcp_servers: Some(vec![McpServerConfig {
            name: "sample".into(),
            transport: McpTransport::Stdio,
            command: Some("new".into()),
            args: Some(vec!["run".into()]),
            url: None,
            env: vec![McpEnvVar {
                key: "TOKEN".into(),
                value: "literal".into(),
                is_secret: true,
                is_env_ref: false,
            }],
            enabled: true,
        }]),
    }
}
#[test]
fn toml_preview_preserves_comments_and_unknown_fields_and_conflicts() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(
        &path,
        "# top\n[mcp_servers.sample]\n# comment\ncommand = 'old'\ncustom = 42\n",
    )
    .unwrap();
    let hash = agents_manager_core::config::disk_sha256(&path).unwrap();
    let req = request(&path, hash);
    let preview =
        structured::preview_structured(entry(&path, ConfigFormat::Toml), &req, None).unwrap();
    assert!(preview.content.contains("# comment"));
    assert!(preview.content.contains("custom = 42"));
    assert!(preview.content.contains("new"));
    fs::write(&path, "# externally changed\n").unwrap();
    assert!(matches!(
        structured::save_structured(
            entry(&path, ConfigFormat::Toml),
            &req,
            &dir.path().join("backups"),
            None
        ),
        Err(CoreError::ExternalConflict)
    ));
}
#[test]
fn json_preview_preserves_unknown_fields() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    fs::write(
        &path,
        r#"{"unknown":1,"mcpServers":{"sample":{"custom":42,"command":"old"}}}"#,
    )
    .unwrap();
    let req = request(
        &path,
        agents_manager_core::config::disk_sha256(&path).unwrap(),
    );
    let preview =
        structured::preview_structured(entry(&path, ConfigFormat::Json), &req, None).unwrap();
    let json: serde_json::Value = serde_json::from_str(&preview.content).unwrap();
    assert_eq!(json["unknown"], 1);
    assert_eq!(json["mcpServers"]["sample"]["custom"], 42);
    assert_eq!(json["mcpServers"]["sample"]["command"], "new");
}

#[test]
fn structured_draft_preserves_raw_edits_and_backs_up_disk() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    fs::write(&path, r#"{"unknown":1,"mcpServers":{}}"#).unwrap();
    let req = request(
        &path,
        agents_manager_core::config::disk_sha256(&path).unwrap(),
    );
    let draft = r#"{"unknown":2,"rawField":true,"mcpServers":{}}"#;
    let saved = structured::save_structured(
        entry(&path, ConfigFormat::Json),
        &req,
        &dir.path().join("backups"),
        Some(draft),
    )
    .unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(json["unknown"], 2);
    assert_eq!(json["rawField"], true);
    assert_eq!(
        fs::read_to_string(saved.backup_path).unwrap(),
        r#"{"unknown":1,"mcpServers":{}}"#
    );
}

#[test]
fn settings_merge_preserves_unknown_nested_fields_in_both_formats() {
    let dir = tempdir().unwrap();
    for (file, format, data) in [
        (
            "config.json",
            ConfigFormat::Json,
            "{\"nested\":{\"known\":1,\"unknown\":2}}",
        ),
        (
            "config.toml",
            ConfigFormat::Toml,
            "[nested]\nknown = 1\nunknown = 2\n",
        ),
    ] {
        let path = dir.path().join(file);
        fs::write(&path, data).unwrap();
        let mut req = request(
            &path,
            agents_manager_core::config::disk_sha256(&path).unwrap(),
        );
        req.mcp_servers = None;
        req.settings_patch = Some(
            serde_json::json!({"nested":{"known":3}})
                .as_object()
                .unwrap()
                .clone(),
        );
        let result = structured::preview_structured(entry(&path, format), &req, None).unwrap();
        if format == ConfigFormat::Json {
            let value: serde_json::Value = serde_json::from_str(&result.content).unwrap();
            assert_eq!(value["nested"]["unknown"], 2);
            assert_eq!(value["nested"]["known"], 3);
        } else {
            let value: toml::Value = result.content.parse().unwrap();
            assert_eq!(value["nested"]["unknown"].as_integer(), Some(2));
            assert_eq!(value["nested"]["known"].as_integer(), Some(3));
        }
    }
}

#[test]
fn toml_retains_comments_inside_env_when_editing_another_field() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path,"[mcp_servers.sample]\ncommand = 'old'\n[mcp_servers.sample.env]\n# keep secret comment\nTOKEN = 'literal'\n").unwrap();
    let req = request(
        &path,
        agents_manager_core::config::disk_sha256(&path).unwrap(),
    );
    let result =
        structured::preview_structured(entry(&path, ConfigFormat::Toml), &req, None).unwrap();
    assert!(result.content.contains("# keep secret comment"));
}

#[test]
fn switching_transport_removes_incompatible_connection_fields() {
    let dir = tempdir().unwrap();
    for (filename, format, content) in [
        (
            "config.json",
            ConfigFormat::Json,
            r#"{"mcpServers":{"sample":{"command":"old","args":["a"],"custom":1}}}"#,
        ),
        (
            "config.toml",
            ConfigFormat::Toml,
            "[mcp_servers.sample]\ncommand='old'\nargs=['a']\ncustom=1\n",
        ),
    ] {
        let path = dir.path().join(filename);
        fs::write(&path, content).unwrap();
        let mut req = request(
            &path,
            agents_manager_core::config::disk_sha256(&path).unwrap(),
        );
        let server = &mut req.mcp_servers.as_mut().unwrap()[0];
        server.transport = McpTransport::Http;
        server.url = Some("https://example.test".into());
        server.command = None;
        server.args = None;
        let result = structured::preview_structured(entry(&path, format), &req, None).unwrap();
        if format == ConfigFormat::Json {
            let v: serde_json::Value = serde_json::from_str(&result.content).unwrap();
            assert!(v["mcpServers"]["sample"].get("command").is_none());
        } else {
            let v: toml::Value = result.content.parse().unwrap();
            assert!(v["mcp_servers"]["sample"].get("command").is_none());
        }
    }
}
