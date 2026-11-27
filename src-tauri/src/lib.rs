pub mod commands;
mod sync_preview;
mod watcher;

use std::path::{Path, PathBuf};

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands};

fn specta_builder<R: tauri::Runtime>() -> Builder<R> {
    Builder::<R>::new().commands(collect_commands![
        commands::list_agents,
        commands::list_agent_configs,
        commands::read_config_file,
        commands::save_config_file,
        commands::watch_config_file,
        commands::list_projects,
        commands::register_project,
        commands::remove_project,
        commands::list_project_configs,
        commands::preview_structured_save,
        commands::save_structured_config,
        commands::list_library_items,
        commands::read_library_item,
        commands::promote_to_library,
        commands::save_library_item,
        commands::delete_library_item,
        commands::unlink_sync,
        commands::preview_sync,
        commands::apply_sync
    ])
}

fn with_ipc_handler<R: tauri::Runtime>(
    app: tauri::Builder<R>,
    specta: &Builder<R>,
) -> tauri::Builder<R> {
    use tauri::{Emitter, Manager};
    app.manage(watcher::WatchState::default())
        .manage(sync_preview::PreviewState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<watcher::WatchState>();
            *state.emitter.lock().map_err(|err| err.to_string())? =
                Some(std::sync::Arc::new(move |changed| {
                    if changed
                        .to_str()
                        .is_none_or(|path| commands::authorized(path).is_err())
                    {
                        return;
                    }
                    let disk_sha256 = agents_manager_core::config::disk_sha256(&changed).ok();
                    let _ = handle.emit(
                        "config-file-changed",
                        commands::ConfigFileChanged {
                            path: changed.to_string_lossy().into_owned(),
                            disk_sha256,
                        },
                    );
                }));
            Ok(())
        })
        .invoke_handler(specta.invoke_handler())
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
        .map_err(|err| format!("failed to export TypeScript bindings: {err}"))?;
    let generated = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut bindings = String::with_capacity(generated.len());
    for line in generated.lines() {
        bindings.push_str(line.trim_end());
        bindings.push('\n');
    }
    if bindings != generated {
        std::fs::write(path, bindings).map_err(|err| err.to_string())?;
    }
    Ok(())
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod watcher_tests {
    use super::watcher::watch_path;
    use std::{fs, sync::mpsc, time::Duration};
    #[test]
    fn watcher_detects_atomic_replacement_and_ignores_other_files() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("config.toml");
        fs::write(&target, "before").unwrap();
        let (tx, rx) = mpsc::channel();
        let _watcher = watch_path(&target, move |path| {
            tx.send(path).unwrap();
        })
        .unwrap();
        fs::write(temp.path().join("other"), "ignored").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
        let staged = temp.path().join("staged");
        fs::write(&staged, "after").unwrap();
        fs::rename(staged, &target).unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), target);
    }
}
