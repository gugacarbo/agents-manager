use tauri::{
    Manager,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::preferences_state::PreferencesState;

const APP_ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");
const TRAY_ICON_PNG: &[u8] = include_bytes!("../icons/tray.png");

fn restore_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Registers the desktop system tray icon and context menu (`Open Agents Manager` / `Quit`).
pub fn setup_tray<R: tauri::Runtime>(app: &tauri::App<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open Agents Manager", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Agents Manager")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => restore_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                restore_main_window(tray.app_handle());
            }
        });

    let tray_icon = Image::from_bytes(TRAY_ICON_PNG)
        .ok()
        .or_else(|| app.default_window_icon().cloned());
    if let Some(icon) = tray_icon {
        tray = tray.icon(icon);
    }

    if let Some(window) = app.get_webview_window("main")
        && let Ok(app_icon) = Image::from_bytes(APP_ICON_PNG)
    {
        let _ = window.set_icon(app_icon);
    }

    #[cfg(all(debug_assertions, target_os = "linux"))]
    ensure_dev_desktop_entry();

    tray.build(app)?;
    Ok(())
}

#[cfg(all(debug_assertions, target_os = "linux"))]
fn ensure_dev_desktop_entry() {
    let Some(base_dirs) = directories::BaseDirs::new() else {
        return;
    };
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let _ = write_dev_desktop_entry_in(base_dirs.data_local_dir(), &exe);
}

#[cfg(all(debug_assertions, target_os = "linux"))]
fn write_dev_desktop_entry_in(
    data_local_dir: &std::path::Path,
    exe: &std::path::Path,
) -> std::io::Result<()> {
    let icon_dir = data_local_dir.join("icons/hicolor/512x512/apps");
    let icon_path = icon_dir.join("agents-manager.png");
    if std::fs::read(&icon_path).ok().as_deref() != Some(APP_ICON_PNG) {
        std::fs::create_dir_all(&icon_dir)?;
        std::fs::write(&icon_path, APP_ICON_PNG)?;
    }

    let apps_dir = data_local_dir.join("applications");
    let desktop_path = apps_dir.join("dev.agentsmanager.app.desktop");
    let desktop_content = format!(
        "[Desktop Entry]\nType=Application\nName=Agents Manager\nComment=Manage local and remote AI coding agent configurations\nExec={}\nIcon=agents-manager\nTerminal=false\nCategories=Development;Utility;\nStartupWMClass=agents-manager\n",
        exe.display(),
    );
    if std::fs::read_to_string(&desktop_path).ok().as_deref() != Some(desktop_content.as_str()) {
        std::fs::create_dir_all(&apps_dir)?;
        std::fs::write(&desktop_path, desktop_content)?;
    }
    Ok(())
}

/// Intercepts `main` window close requests and hides the window when tray mode is enabled.
pub fn handle_window_event<R: tauri::Runtime>(
    window: &tauri::Window<R>,
    event: &tauri::WindowEvent,
) {
    if window.label() == "main"
        && let tauri::WindowEvent::CloseRequested { api, .. } = event
        && window.state::<PreferencesState>().minimize_to_tray()
    {
        api.prevent_close();
        let _ = window.hide();
    }
}

#[cfg(all(test, debug_assertions, target_os = "linux"))]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn dev_desktop_entry_writes_icon_and_desktop_file() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("agents-manager");
        write_dev_desktop_entry_in(temp.path(), &exe).unwrap();

        let icon = temp
            .path()
            .join("icons/hicolor/512x512/apps/agents-manager.png");
        let desktop = temp
            .path()
            .join("applications/dev.agentsmanager.app.desktop");

        assert_eq!(std::fs::read(&icon).unwrap(), APP_ICON_PNG);
        let content = std::fs::read_to_string(&desktop).unwrap();
        assert!(content.contains("Icon=agents-manager"));
        assert!(content.contains("StartupWMClass=agents-manager"));
        assert!(content.contains(&format!("Exec={}", exe.display())));
    }
}
