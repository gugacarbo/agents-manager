use agents_manager_core::claude_accounts::{
    AccountError, AccountStatus, ClaudeAccount, ClaudeAccounts, ClaudePrograms, STORE_NAME,
};
use std::{path::PathBuf, sync::OnceLock};

pub(super) fn private_store() -> Result<PathBuf, agents_manager_core::error::CoreError> {
    Ok(super::data_dir()?.join(STORE_NAME))
}

fn service() -> Result<&'static ClaudeAccounts, AccountError> {
    static ACCOUNTS: OnceLock<ClaudeAccounts> = OnceLock::new();
    if let Some(accounts) = ACCOUNTS.get() {
        return Ok(accounts);
    }
    let store = private_store().map_err(|_| AccountError::StorageUnavailable)?;
    let home = super::home().map_err(|_| AccountError::StorageUnavailable)?;
    Ok(ACCOUNTS.get_or_init(|| ClaudeAccounts::new(store, ClaudePrograms::local(home))))
}

#[tauri::command]
#[specta::specta]
pub fn list_claude_accounts() -> Result<Vec<ClaudeAccount>, AccountError> {
    service()?.list()
}

#[tauri::command]
#[specta::specta]
pub fn create_claude_account(display_name: String) -> Result<ClaudeAccount, AccountError> {
    service()?.create(&display_name)
}

#[tauri::command]
#[specta::specta]
pub async fn start_claude_account_login(profile_id: String) -> Result<AccountStatus, AccountError> {
    tauri::async_runtime::spawn_blocking(move || service()?.start_login(&profile_id))
        .await
        .map_err(|_| AccountError::LoginFailed)?
}

#[tauri::command]
#[specta::specta]
pub async fn get_claude_account_status(profile_id: String) -> Result<AccountStatus, AccountError> {
    tauri::async_runtime::spawn_blocking(move || service()?.status(&profile_id))
        .await
        .map_err(|_| AccountError::StatusInvalid)?
}
