use std::{env, fs, path::Path, process::Command};

use tauri::{
    WebviewWindowBuilder,
    ipc::{InvokeBody, InvokeResponseBody},
    test,
    webview::InvokeRequest,
};
use tempfile::tempdir;

use super::{specta_builder, with_ipc_handler};

const CHILD_HOME: &str = "AGENTS_MANAGER_IPC_SMOKE_HOME";

#[test]
fn list_agents_ipc_returns_agent_summaries() {
    if let Some(home) = env::var_os(CHILD_HOME) {
        assert_eq!(env::var_os("HOME").as_ref(), Some(&home));
        assert_list_agents_response(Path::new(&home));
        return;
    }

    let home = tempdir().expect("create temporary home");
    fs::create_dir(home.path().join(".claude")).expect("create Claude Code fixture");
    fs::create_dir_all(home.path().join(".gemini/antigravity"))
        .expect("create Antigravity fixture");

    // BaseDirs reads HOME; set it only for this child, not for parallel tests.
    let output = Command::new(env::current_exe().expect("locate test executable"))
        .args([
            "--exact",
            "ipc_smoke::list_agents_ipc_returns_agent_summaries",
        ])
        .env("HOME", home.path())
        .env(CHILD_HOME, home.path())
        .output()
        .expect("run IPC smoke test in an isolated home");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed"),
        "IPC smoke child exited with {}\n{}\n{}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr),
    );
}

fn assert_list_agents_response(home: &Path) {
    let app = with_ipc_handler(test::mock_builder(), &specta_builder())
        .build(tauri::generate_context!(
            test = true,
            assets = test::noop_assets()
        ))
        .expect("build app with MockRuntime");
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == "main")
        .expect("configured main window");
    let webview = WebviewWindowBuilder::from_config(&app, window_config)
        .expect("configure MockRuntime window")
        .build()
        .expect("build MockRuntime window");
    let response = test::get_ipc_response(
        &webview,
        InvokeRequest {
            cmd: "list_agents".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: webview.url().expect("get configured webview URL"),
            body: Default::default(),
            headers: Default::default(),
            invoke_key: test::INVOKE_KEY.into(),
        },
    )
    .expect("Tauri IPC must route list_agents successfully");
    let InvokeResponseBody::Json(json) = response else {
        panic!("expected a JSON IPC response, got {response:?}");
    };
    let invalid_draft = tauri::utils::config::parse::parse_json_value(
        r#"{"draft":{"name":"test","connection":{"mode":"alias","alias":"-bad"}}}"#,
        Path::new("test_ssh_connection IPC"),
    )
    .expect("parse SSH request");
    let error = test::get_ipc_response(
        &webview,
        InvokeRequest {
            cmd: "test_ssh_connection".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: webview.url().expect("webview URL"),
            body: InvokeBody::Json(invalid_draft),
            headers: Default::default(),
            invoke_key: test::INVOKE_KEY.into(),
        },
    )
    .expect_err("invalid SSH destination must fail over IPC");
    assert_eq!(error["SshError"]["kind"], "invalid-input");
    // Read the wire keys and values independently of AgentSummary's serde rules.
    let payload =
        tauri::utils::config::parse::parse_json_value(&json, Path::new("list_agents IPC"))
            .expect("deserialize list_agents response JSON");
    let agents = payload.as_array().expect("array of agent summaries");
    assert_eq!(agents.len(), 3);
    for (kind, name, relative_path) in [
        ("claude-code", "Claude Code", Some(".claude")),
        ("codex", "Codex", None),
        ("antigravity", "Antigravity", Some(".gemini/antigravity")),
    ] {
        let agent = agents
            .iter()
            .find(|agent| agent["kind"] == kind)
            .unwrap_or_else(|| panic!("missing kind {kind} in {agents:?}"));
        assert_eq!(agent["name"], name);
        assert_eq!(agent["detected"], relative_path.is_some());
        let config_path = agent
            .get("configPath")
            .expect("missing configPath wire key");
        if let Some(relative_path) = relative_path {
            assert_eq!(config_path.as_str(), home.join(relative_path).to_str());
        } else {
            assert!(config_path.is_null(), "expected null configPath for {kind}");
        }
    }
}
