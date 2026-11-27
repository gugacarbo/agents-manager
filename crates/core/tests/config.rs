#![allow(clippy::unwrap_used)]
use agents_manager_core::{
    config::{read_config, save_config},
    domain::{AgentKind, ConfigCategory, ConfigFileEntry, ConfigFormat, SaveConfigRequest},
    error::CoreError,
};
use std::fs;
use tempfile::tempdir;

fn entry(path: &std::path::Path, format: ConfigFormat) -> ConfigFileEntry {
    ConfigFileEntry {
        agent: AgentKind::Codex,
        category: ConfigCategory::Settings,
        label: "config".into(),
        path: path.display().to_string(),
        format,
        exists: true,
    }
}

#[test]
fn detects_sensitive_value_spans_as_utf8_byte_offsets() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    let content = r#"{"name":"é", "api_key":"literal-secret", "env":{"TOKEN":"another-secret"}, "password":"${PASSWORD}"}"#;
    fs::write(&path, content).unwrap();
    let doc = read_config(entry(&path, ConfigFormat::Json)).unwrap();
    let spans: Vec<&str> = doc
        .secret_spans
        .iter()
        .map(|span| &content[span.start_byte as usize..span.end_byte as usize])
        .collect();
    assert_eq!(spans, vec!["literal-secret", "another-secret"]);
}

#[test]
fn saves_atomically_with_backup_and_rejects_conflicts_and_invalid_syntax() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    let backups = dir.path().join("backups");
    fs::write(&path, "{\"a\":1}").unwrap();
    let doc = read_config(entry(&path, ConfigFormat::Json)).unwrap();
    let mut request = SaveConfigRequest {
        path: path.display().to_string(),
        content: "{".into(),
        expected_sha256: doc.sha256,
        force_overwrite: None,
    };
    assert!(matches!(
        save_config(&request, &backups),
        Err(CoreError::SyntaxError { .. })
    ));
    assert!(!backups.exists());
    request.content = "{\"a\":2}".into();
    fs::write(&path, "{\"a\":3}").unwrap();
    assert!(matches!(
        save_config(&request, &backups),
        Err(CoreError::ExternalConflict)
    ));
    assert!(!backups.exists());
    request.force_overwrite = Some(true);
    let result = save_config(&request, &backups).unwrap();
    assert_eq!(fs::read_to_string(result.backup_path).unwrap(), "{\"a\":3}");
    assert_eq!(fs::read_to_string(&path).unwrap(), request.content);
}

#[test]
fn retains_twenty_backups_per_target() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let backups = dir.path().join("backups");
    fs::write(&path, "n = 0\n").unwrap();
    for n in 1..=23 {
        let doc = read_config(entry(&path, ConfigFormat::Toml)).unwrap();
        save_config(
            &SaveConfigRequest {
                path: path.display().to_string(),
                content: format!("n = {n}\n"),
                expected_sha256: doc.sha256,
                force_overwrite: None,
            },
            &backups,
        )
        .unwrap();
    }
    assert_eq!(fs::read_dir(backups).unwrap().count(), 20);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn masks_all_literal_values_in_env_and_headers_for_json_and_toml() {
    let dir = tempdir().unwrap();
    for (name, content, format) in [
        (
            "config.json",
            r#"{"env":{"DATABASE_URL":"postgres://user:pass@host"},"headers":{"X-Custom":"value"}}"#,
            ConfigFormat::Json,
        ),
        (
            "config.toml",
            "[mcp_servers.foo.env]\nDATABASE_URL = \"postgres://user:pass@host\"\n[headers]\nX_Custom = \"value\"\n",
            ConfigFormat::Toml,
        ),
    ] {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        let doc = read_config(entry(&path, format)).unwrap();
        let spans: Vec<&str> = doc
            .secret_spans
            .iter()
            .map(|span| &content[span.start_byte as usize..span.end_byte as usize])
            .collect();
        assert_eq!(spans, vec!["postgres://user:pass@host", "value"]);
    }
}

#[test]
fn failed_backup_or_temp_write_leaves_target_untouched() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "n = 1").unwrap();
    let doc = read_config(entry(&path, ConfigFormat::Toml)).unwrap();
    let backup_file = dir.path().join("not-a-directory");
    fs::write(&backup_file, "occupied").unwrap();
    let result = save_config(
        &SaveConfigRequest {
            path: path.display().to_string(),
            content: "n = 2".into(),
            expected_sha256: doc.sha256,
            force_overwrite: None,
        },
        &backup_file,
    );
    assert!(matches!(result, Err(CoreError::IoError { .. })));
    assert_eq!(fs::read_to_string(path).unwrap(), "n = 1");
}

#[test]
fn masks_escaped_quotes_in_secret_values() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.json");
    let content = r#"{"password":"abc\"secret"}"#;
    fs::write(&path, content).unwrap();
    let doc = read_config(entry(&path, ConfigFormat::Json)).unwrap();
    let span = &doc.secret_spans[0];
    assert_eq!(
        &content[span.start_byte as usize..span.end_byte as usize],
        r#"abc\"secret"#
    );
}
