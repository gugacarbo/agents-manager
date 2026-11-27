pub mod commands;

use std::path::{Path, PathBuf};

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands};

fn specta_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new().commands(collect_commands![commands::list_agents])
}

fn with_ipc_handler<R: tauri::Runtime>(
    app: tauri::Builder<R>,
    specta: &Builder<R>,
) -> tauri::Builder<R> {
    app.invoke_handler(specta.invoke_handler())
}

#[cfg(all(test, target_os = "linux"))]
#[allow(clippy::expect_used)]
mod ipc_smoke;

/// Returns the canonical path for generated TypeScript IPC bindings.
pub fn default_bindings_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings/tauri.ts")
}

/// Exports TypeScript bindings for registered Tauri commands and domain types to `path`.
pub fn export_bindings_to(path: &Path) -> Result<(), String> {
    export_with_builder(&specta_builder(), path)
}

fn export_with_builder(builder: &Builder<tauri::Wry>, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create bindings directory: {err}"))?;
    }

    builder
        .export(Typescript::default(), path)
        .map_err(|err| format!("failed to export TypeScript bindings: {err}"))
}

/// Launches the Tauri desktop application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    if let Err(err) = export_with_builder(&builder, &default_bindings_path()) {
        eprintln!("failed to export TypeScript bindings: {err}");
    }

    with_ipc_handler(tauri::Builder::default(), &builder)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn exports_typescript_bindings_to_temp_file() {
        let temp = tempdir().unwrap();
        let bindings_path = temp.path().join("bindings.ts");

        export_bindings_to(&bindings_path).unwrap();

        let content = fs::read_to_string(&bindings_path).unwrap();
        assert!(content.contains("listAgents"));
        assert!(content.contains("AgentSummary"));
        assert!(content.contains("CoreError"));
    }
}
