#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

use agents_manager_core::claude_accounts::{
    AccountError, AccountState, ClaudeAccounts, ClaudePrograms, is_private_path,
};
use std::{
    ffi::OsString,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    time::Duration,
};

const CLI: &str = r#"
[ "$#" = 2 ] && [ "$1" = auth ] && [ "$PWD" = / ] || exit 41
[ -z "${ANTHROPIC_API_KEY+x}${CLAUDE_CODE_OAUTH_TOKEN+x}${BASH_ENV+x}" ] || exit 42
[ "$HOME" != "$CLAUDE_CONFIG_DIR" ] && [ -d "$HOME" ] && [ -d "$CLAUDE_CONFIG_DIR" ] || exit 43
case "$2" in
  status)
    case "$0" in
      */bad) printf '{"loggedIn":false,"authMethod":"none"}'; exit 1;;
      */wrong) printf '{"loggedIn":true,"authMethod":"claude.ai","configDirectory":"/wrong"}'; exit 0;;
      */foreign) printf '{"loggedIn":true,"authMethod":"oauth_token","configDirectory":"%s"}' "$CLAUDE_CONFIG_DIR"; exit 0;;
      */contradiction) printf '{"loggedIn":true,"authMethod":"claude.ai","configDirectory":"%s"}' "$CLAUDE_CONFIG_DIR"; exit 1;;
      */timeout) ( /bin/sleep 30 & ); exit 1;;
      */large) /usr/bin/head -c 100000 /dev/zero; exit 1;;
    esac
    printf '%s' 'fictitious-secret-stderr' >&2
    if [ -f "$CLAUDE_CONFIG_DIR/account" ]; then
      printf '{"configDirectory":"%s","authMethod":"claude.ai","loggedIn":true,"email":"fictitious-secret","unknown":{"token":"fictitious-secret"}}' "$CLAUDE_CONFIG_DIR"
      exit 0
    fi
    printf '{"configDirectory":"%s","authMethod":"none","loggedIn":false}' "$CLAUDE_CONFIG_DIR"
    exit 1;;
  login)
    printf '%s' "$CLAUDE_CONFIG_DIR" > "$CLAUDE_CONFIG_DIR/account"
    printf '%s' fictitious-secret-login >&2;;
  *) exit 44;;
esac
"#;
const TERMINAL: &str = r#"
[ "$1" = --wait ] && [ "$2" = --working-directory=/ ] && [ "$3" = -- ] && [ "$4" = /usr/bin/env ] && [ "$5" = -i ] || exit 51
shift 3
export ANTHROPIC_API_KEY=fictitious-secret-from-terminal-server
exec "$@"
"#;

fn executable(root: &Path, name: &str, code: &str) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, format!("#!/bin/sh\n{code}")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

struct Fixture {
    tmp: tempfile::TempDir,
    store: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let store = tmp.path().join("claude-accounts");
        Self { tmp, store }
    }
    fn accounts(&self, cli: &str, terminal: &str) -> ClaudeAccounts {
        let path = self.tmp.path();
        let cli = if cli == "missing" {
            path.join(cli)
        } else {
            executable(path, cli, CLI)
        };
        let terminal = executable(path, "terminal", terminal);
        ClaudeAccounts::new(
            self.store.clone(),
            ClaudePrograms::new(
                cli,
                terminal,
                Duration::from_millis(700),
                [
                    (
                        OsString::from("ANTHROPIC_API_KEY"),
                        OsString::from("fictitious-secret-from-app"),
                    ),
                    (
                        OsString::from("CLAUDE_CODE_OAUTH_TOKEN"),
                        OsString::from("fictitious-secret-from-app"),
                    ),
                ],
            ),
        )
    }
}

#[test]
fn two_profiles_survive_reopen_without_reusing_default_or_each_other() {
    let f = Fixture::new();
    let standard = f.tmp.path().join(".claude.json");
    fs::write(&standard, "default-auth-fixture").unwrap();
    let app = f.accounts("claude", TERMINAL);
    let a = app.create("First").unwrap();
    let b = app.create("Second").unwrap();
    assert_ne!(a.id, b.id);
    assert_eq!(
        app.start_login(&a.id).unwrap().state,
        AccountState::Connected
    );
    assert_eq!(app.status(&b.id).unwrap().state, AccountState::Disconnected);
    assert_eq!(
        app.start_login(&b.id).unwrap().state,
        AccountState::Connected
    );
    let reopened = f.accounts("claude", TERMINAL);
    let rows = reopened.list().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.contains(&a) && rows.contains(&b));
    for profile in [a, b] {
        let state = reopened.status(&profile.id).unwrap();
        assert_eq!(state.profile_id, profile.id);
        assert_eq!(state.state, AccountState::Connected);
        let config = f.store.join(profile.id).join("config");
        assert_eq!(
            fs::metadata(&config).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::read_to_string(config.join("account")).unwrap(),
            config.to_str().unwrap()
        );
    }
    assert_eq!(
        fs::read_to_string(standard).unwrap(),
        "default-auth-fixture"
    );
}

#[test]
fn cancelled_or_unconfirmed_login_is_not_connected_and_can_be_retried() {
    let f = Fixture::new();
    let app = f.accounts("claude", "exit 1");
    let a = app.create("Retry").unwrap();
    assert_eq!(
        app.start_login(&a.id).unwrap().state,
        AccountState::Disconnected
    );
    let app = f.accounts("claude", "exit 0");
    assert_eq!(
        app.start_login(&a.id).unwrap().state,
        AccountState::Disconnected
    );
    let app = f.accounts("claude", TERMINAL);
    assert_eq!(
        app.start_login(&a.id).unwrap().state,
        AccountState::Connected
    );
    let app = f.accounts("claude", "exit 1");
    assert_ne!(
        app.start_login(&a.id).unwrap().state,
        AccountState::Connected
    );
}

#[test]
fn invalid_status_fails_closed_and_does_not_launch_interactive_login() {
    let f = Fixture::new();
    for (cli, expected) in [
        ("missing", AccountError::CliUnavailable),
        ("bad", AccountError::StatusInvalid),
        ("wrong", AccountError::StatusInvalid),
        ("foreign", AccountError::StatusInvalid),
        ("contradiction", AccountError::StatusInvalid),
        ("timeout", AccountError::StatusTimeout),
        ("large", AccountError::StatusInvalid),
    ] {
        let app = f.accounts(cli, "echo launched > /proc/self/fd/1; exit 0");
        let a = app.create(cli).unwrap();
        // If terminal is launched despite a failed preflight it drops a private marker.
        let marker = f.store.join(&a.id).join("launched");
        executable(
            f.tmp.path(),
            "terminal",
            &format!("touch '{}'; exit 0", marker.display()),
        );
        let status = app.start_login(&a.id).unwrap();
        assert_eq!(status.state, AccountState::VerificationFailed, "{cli}");
        assert_eq!(status.error_code, Some(expected), "{cli}");
        assert!(!marker.exists(), "terminal launched for {cli}");
    }
}

#[test]
fn second_login_and_status_stay_pending_until_terminal_and_confirmation_finish() {
    let f = Fixture::new();
    let marker = f.tmp.path().join("terminal-started");
    let release = f.tmp.path().join("terminal-release");
    let app = f.accounts("claude", &format!("printf run >> '{}'; for i in 1 2 3 4 5 6 7 8 9 10; do [ -e '{}' ] && break; /bin/sleep .1; done;\n{TERMINAL}", marker.display(), release.display()));
    let a = app.create("Concurrent").unwrap();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| app.start_login(&a.id).unwrap());
        for _ in 0..100 {
            if marker.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(marker.exists());
        assert_eq!(app.start_login(&a.id).unwrap().state, AccountState::Pending);
        assert_eq!(app.status(&a.id).unwrap().state, AccountState::Pending);
        fs::write(&release, "").unwrap();
        assert_eq!(first.join().unwrap().state, AccountState::Connected);
    });
    assert_eq!(fs::read_to_string(marker).unwrap(), "run");
}

#[test]
fn private_files_and_cli_output_never_join_public_payload_and_symlinks_fail_closed() {
    let f = Fixture::new();
    let app = f.accounts("claude", TERMINAL);
    let a = app.create("Personal").unwrap();
    let status = app.start_login(&a.id).unwrap();
    assert_eq!(
        serde_json::to_value(status).unwrap(),
        serde_json::json!({ "profileId":a.id,"state":"connected" })
    );
    let dir = f.store.join(&a.id);
    let stored: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("profile.json")).unwrap()).unwrap();
    assert_eq!(
        stored,
        serde_json::json!({"id": a.id, "displayName":"Personal"})
    );
    let outside = f.tmp.path().join("outside.json");
    fs::write(&outside, "default-state").unwrap();
    let config = dir.join("config");
    symlink(&outside, config.join(".claude.json")).unwrap();
    let alias = f.tmp.path().join("alias");
    symlink(dir.join("config/account"), &alias).unwrap();
    assert!(is_private_path(&f.store, &config));
    assert!(is_private_path(&f.store, &alias));
    assert!(!is_private_path(&f.store, &outside));
    assert_eq!(
        app.status(&a.id).unwrap().error_code,
        Some(AccountError::StorageUnavailable)
    );
    assert_eq!(
        app.status("../../.claude").unwrap_err(),
        AccountError::ProfileNotFound
    );
    assert_eq!(fs::read_to_string(outside).unwrap(), "default-state");
}
