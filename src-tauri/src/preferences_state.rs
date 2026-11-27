use std::sync::atomic::{AtomicBool, Ordering};

/// In-memory desktop preference state shared with window event handlers.
#[derive(Debug)]
pub struct PreferencesState {
    minimize_to_tray: AtomicBool,
}

impl PreferencesState {
    /// Creates a new [`PreferencesState`] initialized with `minimize_to_tray`.
    pub fn new(minimize_to_tray: bool) -> Self {
        Self {
            minimize_to_tray: AtomicBool::new(minimize_to_tray),
        }
    }

    /// Returns whether closing the main window should hide to the system tray.
    pub fn minimize_to_tray(&self) -> bool {
        self.minimize_to_tray.load(Ordering::Relaxed)
    }

    /// Updates the cached `minimize_to_tray` setting.
    pub fn set_minimize_to_tray(&self, enabled: bool) {
        self.minimize_to_tray.store(enabled, Ordering::Relaxed);
    }
}
