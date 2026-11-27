use crate::{
    config::{detect_secrets, read_config, save_config},
    domain::{
        ConfigCategory, ConfigFileEntry, ConfigFormat, McpServerConfig, McpTransport,
        SaveConfigRequest, SaveConfigResult, StructuredPreviewResult, StructuredSaveRequest,
    },
    error::CoreError,
};
use serde_json::{Map, Value};
use std::path::Path;
use toml_edit::{Array, DocumentMut, Item, Table};

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::SyntaxError {
        message: message.into(),
    }
}
fn merge_json(target: &mut Value, patch: &Value) {
    if let (Some(target), Some(patch)) = (target.as_object_mut(), patch.as_object()) {
        for (key, value) in patch {
            if let Some(existing) = target.get_mut(key)
                && existing.is_object()
                && value.is_object()
            {
                merge_json(existing, value);
                continue;
            }
            target.insert(key.clone(), value.clone());
        }
    }
}
fn server_patch(server: &McpServerConfig) -> Value {
    let mut map = Map::new();
    if let Some(command) = &server.command {
        map.insert("command".into(), Value::String(command.clone()));
    }
    if let Some(args) = &server.args {
        map.insert("args".into(), serde_json::json!(args));
    }
    if let Some(url) = &server.url {
        map.insert("url".into(), Value::String(url.clone()));
    }
    map.insert("enabled".into(), Value::Bool(server.enabled));
    map.insert(
        "env".into(),
        Value::Object(
            server
                .env
                .iter()
                .map(|e| (e.key.clone(), Value::String(e.value.clone())))
                .collect(),
        ),
    );
    Value::Object(map)
}
pub(crate) fn json_content(
    content: &str,
    request: &StructuredSaveRequest,
) -> Result<String, CoreError> {
    let mut doc: Value = serde_json::from_str(content).map_err(|e| invalid(e.to_string()))?;
    if !doc.is_object() {
        return Err(invalid("JSON root must be an object"));
    }
    if let Some(patch) = &request.settings_patch {
        merge_json(&mut doc, &Value::Object(patch.clone()));
    }
    if let Some(servers) = &request.mcp_servers {
        let root = doc
            .as_object_mut()
            .ok_or_else(|| invalid("JSON root must be an object"))?;
        let server_map = root
            .entry("mcpServers")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| invalid("mcpServers must be an object"))?;
        for server in servers {
            let existing = server_map
                .entry(server.name.clone())
                .or_insert_with(|| Value::Object(Map::new()));
            if !existing.is_object() {
                return Err(invalid("server must be an object"));
            }
            if let Some(object) = existing.as_object_mut() {
                match server.transport {
                    McpTransport::Http => {
                        object.remove("command");
                        object.remove("args");
                    }
                    McpTransport::Stdio => {
                        object.remove("url");
                    }
                }
            }
            merge_json(existing, &server_patch(server));
            // env is a complete edited list, allowing users to remove a variable.
            existing["env"] = server_patch(server)["env"].clone();
        }
    }
    Ok(format!("{}\n", serde_json::to_string_pretty(&doc)?))
}
fn json_item(value: &Value) -> Result<Item, CoreError> {
    match value {
        Value::String(s) => Ok(toml_edit::value(s.as_str())),
        Value::Bool(b) => Ok(toml_edit::value(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml_edit::value(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml_edit::value(f))
            } else {
                Err(invalid("unsupported number"))
            }
        }
        Value::Array(values) => {
            let mut array = Array::new();
            for v in values {
                let item = json_item(v)?;
                let Some(v) = item.into_value().ok() else {
                    return Err(invalid("unsupported array value"));
                };
                array.push(v);
            }
            Ok(Item::Value(toml_edit::Value::Array(array)))
        }
        Value::Object(map) => {
            let mut table = Table::new();
            for (k, v) in map {
                table.insert(k, json_item(v)?);
            }
            Ok(Item::Table(table))
        }
        Value::Null => Err(invalid("TOML does not support null")),
    }
}
fn merge_toml(table: &mut Table, patch: &Map<String, Value>) -> Result<(), CoreError> {
    for (key, v) in patch {
        if let Some(object) = v.as_object() {
            if !table.contains_key(key) {
                table.insert(key, Item::Table(Table::new()));
            }
            if let Some(child) = table.get_mut(key).and_then(Item::as_table_mut) {
                merge_toml(child, object)?;
                continue;
            }
        }
        let mut item = json_item(v)?;
        if let (Some(old), Some(new)) =
            (table.get(key).and_then(Item::as_value), item.as_value_mut())
        {
            *new.decor_mut() = old.decor().clone();
        }
        if let Some(existing) = table.get_mut(key) {
            *existing = item;
        } else {
            table.insert(key, item);
        }
    }
    Ok(())
}
pub(crate) fn toml_content(
    content: &str,
    request: &StructuredSaveRequest,
) -> Result<String, CoreError> {
    let mut doc: DocumentMut = content
        .parse::<DocumentMut>()
        .map_err(|e| invalid(e.to_string()))?;
    if let Some(patch) = &request.settings_patch {
        merge_toml(doc.as_table_mut(), patch)?;
    }
    if let Some(servers) = &request.mcp_servers {
        let root = doc.as_table_mut();
        if !root.contains_key("mcp_servers") {
            root.insert("mcp_servers", Item::Table(Table::new()));
        }
        let tables = root
            .get_mut("mcp_servers")
            .and_then(Item::as_table_mut)
            .ok_or_else(|| invalid("mcp_servers must be a table"))?;
        for server in servers {
            if !tables.contains_key(&server.name) {
                tables.insert(&server.name, Item::Table(Table::new()));
            }
            let table = tables
                .get_mut(&server.name)
                .and_then(Item::as_table_mut)
                .ok_or_else(|| invalid("server must be a table"))?;
            match server.transport {
                McpTransport::Http => {
                    table.remove("command");
                    table.remove("args");
                }
                McpTransport::Stdio => {
                    table.remove("url");
                }
            }
            let patch = server_patch(server);
            let object = patch
                .as_object()
                .ok_or_else(|| invalid("server must be an object"))?;
            merge_toml(table, object)?;
            let env = table
                .get_mut("env")
                .and_then(Item::as_table_mut)
                .ok_or_else(|| invalid("env must be a table"))?;
            let retained = patch["env"]
                .as_object()
                .ok_or_else(|| invalid("env must be an object"))?;
            env.retain(|key, _| retained.contains_key(key));
        }
    }
    Ok(doc.to_string())
}
pub fn preview_structured(
    entry: ConfigFileEntry,
    request: &StructuredSaveRequest,
    draft_content: Option<&str>,
) -> Result<StructuredPreviewResult, CoreError> {
    if entry.path != request.path {
        return Err(CoreError::PathNotAllowed {
            path: request.path.clone(),
        });
    }
    if !matches!(
        entry.category,
        ConfigCategory::Mcps | ConfigCategory::Settings
    ) {
        return Err(invalid("unsupported configuration category"));
    }
    let disk = read_config(entry.clone())?;
    let base = draft_content.unwrap_or(&disk.content);
    let content = match entry.format {
        ConfigFormat::Json => json_content(base, request)?,
        ConfigFormat::Toml => toml_content(base, request)?,
        ConfigFormat::Markdown => return Err(invalid("unsupported configuration format")),
    };
    let secret_spans = detect_secrets(&content);
    Ok(StructuredPreviewResult {
        content,
        secret_spans,
    })
}
pub fn save_structured(
    entry: ConfigFileEntry,
    request: &StructuredSaveRequest,
    backups_dir: &Path,
    draft_content: Option<&str>,
) -> Result<SaveConfigResult, CoreError> {
    let preview = preview_structured(entry, request, draft_content)?;
    save_config(
        &SaveConfigRequest {
            path: request.path.clone(),
            content: preview.content,
            expected_sha256: request.expected_sha256.clone(),
            force_overwrite: request.force_overwrite,
        },
        backups_dir,
    )
}
