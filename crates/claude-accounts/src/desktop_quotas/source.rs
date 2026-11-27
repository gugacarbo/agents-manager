#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn selects_only_unexpired_active_account_profile_token() {
        let cache = br#"{"acct:active|client:org:scope:user:profile":{"token":"fabricated","expiresAt":2000},"acct:other|client:org:scope:user:profile":{"token":"wrong","expiresAt":3000}}"#;
        assert_eq!(
            select_token(cache, "active", 1000).as_deref(),
            Ok("fabricated")
        );
        assert!(select_token(cache, "active", 2000).is_err());
        assert!(select_token(cache, "missing", 1000).is_err());
    }

    #[test]
    fn decrypts_fabricated_basic_cache_and_rejects_unknown_format() {
        use cbc::cipher::BlockEncryptMut;
        let mut key = [0u8; 16];
        pbkdf2::pbkdf2_hmac::<sha1::Sha1>(b"peanuts", b"saltysalt", 1, &mut key);
        let ciphertext = cbc::Encryptor::<Aes128>::new(&key.into(), &[b' '; 16].into())
            .encrypt_padded_vec_mut::<Pkcs7>(br#"{"fixture":true}"#);
        let encoded = STANDARD.encode([b"v10".as_slice(), ciphertext.as_slice()].concat());
        assert_eq!(
            decrypt_cache(&encoded).as_deref(),
            Ok(br#"{"fixture":true}"#.as_slice())
        );
        assert!(decrypt_cache(&STANDARD.encode(b"v99invalid")).is_err());
    }

    #[test]
    fn partial_windows_preserve_missing_fields() {
        let result = parse_usage(
            br#"{"five_hour":{"utilization":0},"seven_day":{"resets_at":"2026-11-30T22:00:00Z"}}"#,
        );
        assert!(matches!(
            result,
            Ok(DesktopQuotaWindows {
                five_hour: Some(DesktopQuotaWindow {
                    utilization: Some(0.0),
                    resets_at: None
                }),
                seven_day: Some(DesktopQuotaWindow {
                    utilization: None,
                    resets_at: Some(_)
                })
            })
        ));
        assert!(parse_usage(b"not-json").is_err());
        assert!(parse_usage(br#"{"seven_day":{"resets_at":"invalid-date"}}"#).is_err());
    }

    fn override_from(dir: &Path, contents: &str) -> Option<String> {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("ccd-environment-config.json"), contents).unwrap();
        conversation_cli_config_override(dir).unwrap()
    }

    #[test]
    fn cli_config_override_reads_the_map_shape() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            override_from(dir.path(), r#"{"CLAUDE_CONFIG_DIR":"/x/y"}"#).as_deref(),
            Some("/x/y")
        );
    }

    #[test]
    fn cli_config_override_reads_the_list_shape_with_name_or_key() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            override_from(
                dir.path(),
                r#"[{"name":"HOME","value":"/home/u"},{"name":"CLAUDE_CONFIG_DIR","value":"/x/y"}]"#,
            )
            .as_deref(),
            Some("/x/y")
        );
        assert_eq!(
            override_from(
                dir.path(),
                r#"[{"key":"CLAUDE_CONFIG_DIR","value":"/x/z"}]"#,
            )
            .as_deref(),
            Some("/x/z")
        );
    }

    #[test]
    fn cli_config_override_finds_the_variable_nested_in_objects() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            override_from(
                dir.path(),
                r#"{"env":{"session":{"vars":{"CLAUDE_CONFIG_DIR":"/x/nested"}}}}"#,
            )
            .as_deref(),
            Some("/x/nested")
        );
    }

    /// Contract: a `name: CLAUDE_CONFIG_DIR` field in a non-variable context is only honoured
    /// when its value is an absolute path without `.`/`..` components. Anything else (relative,
    /// empty, parent traversal) yields `None`, so the caller can never be pointed at an invalid
    /// directory by unrelated JSON that happens to reuse the field name.
    #[test]
    fn cli_config_override_rejects_adversarial_named_entries_with_unusable_values() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            override_from(
                dir.path(),
                r#"{"label":{"name":"CLAUDE_CONFIG_DIR","value":"not-a-path"}}"#,
            ),
            None
        );
        assert_eq!(
            override_from(
                dir.path(),
                r#"{"label":{"name":"CLAUDE_CONFIG_DIR","value":"/x/../etc"}}"#,
            ),
            None
        );
        assert_eq!(
            override_from(
                dir.path(),
                r#"{"label":{"name":"CLAUDE_CONFIG_DIR","value":""}}"#,
            ),
            None
        );
        // Map shape is held to the same rule.
        assert_eq!(
            override_from(dir.path(), r#"{"CLAUDE_CONFIG_DIR":"relative/dir"}"#),
            None
        );
        // A rejected entry does not shadow a valid sibling.
        assert_eq!(
            override_from(
                dir.path(),
                r#"{"label":{"name":"CLAUDE_CONFIG_DIR","value":"not-a-path"},"env":{"CLAUDE_CONFIG_DIR":"/x/y"}}"#,
            )
            .as_deref(),
            Some("/x/y")
        );
    }

    #[test]
    fn cli_config_override_is_none_when_the_variable_or_file_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(conversation_cli_config_override(dir.path()).unwrap(), None);
        assert_eq!(
            override_from(
                dir.path(),
                r#"{"HOME":"/home/u","env":{"PATH":"/usr/bin"}}"#
            )
            .as_deref(),
            None
        );
    }

    #[test]
    fn cli_config_override_reports_unreadable_or_incompatible_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path()).unwrap();
        let path = dir.path().join("ccd-environment-config.json");
        // Not JSON: incompatible, never a partial guess.
        fs::write(&path, "not-json").unwrap();
        assert_eq!(
            conversation_cli_config_override(dir.path()),
            Err(DesktopQuotaError::SourceIncompatible)
        );
        // A directory where the file should be: incompatible.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(
            conversation_cli_config_override(dir.path()),
            Err(DesktopQuotaError::SourceIncompatible)
        );
        // Unreadable because the containing directory denies access: unavailable. Skipped when
        // the suite runs with privileges that bypass mode bits (e.g. root in CI containers).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::remove_dir(&path);
            fs::write(&path, "{}").unwrap();
            fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o000)).unwrap();
            let result = conversation_cli_config_override(dir.path());
            let _ = fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700));
            let expected = if nix::unistd::Uid::current().is_root() {
                Ok(None)
            } else {
                Err(DesktopQuotaError::SourceUnavailable)
            };
            assert_eq!(result, expected);
        }
    }
}

use super::{
    DesktopAccountIdentity, DesktopQuotaError, DesktopQuotaSession, DesktopQuotaSource,
    DesktopQuotaWindow, DesktopQuotaWindows,
};
use aes::Aes128;
use base64::{Engine, engine::general_purpose::STANDARD};
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use serde::Deserialize;
use std::{
    cell::Cell,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub struct LinuxDesktopQuotaSource {
    config_dir: PathBuf,
}
impl LinuxDesktopQuotaSource {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir: config_dir.join("Claude"),
        }
    }
}

struct Session {
    config: PathBuf,
    account: String,
    cache: String,
    token: String,
    confirmed: Cell<bool>,
    client: reqwest::blocking::Client,
}

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "lastKnownAccountUuid")]
    account: Option<String>,
    #[serde(rename = "oauth:tokenCacheV2")]
    cache: Option<String>,
}
fn read_config(path: &Path) -> Result<(String, String), DesktopQuotaError> {
    const MAX_CONFIG_BYTES: usize = 4 * 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?
        .take(MAX_CONFIG_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DesktopQuotaError::SourceIncompatible);
    }
    let c: Config =
        serde_json::from_slice(&bytes).map_err(|_| DesktopQuotaError::SourceIncompatible)?;
    let account = c
        .account
        .filter(|s| !s.is_empty())
        .ok_or(DesktopQuotaError::Unauthenticated)?;
    let cache = c
        .cache
        .filter(|s| !s.is_empty())
        .ok_or(DesktopQuotaError::Unauthenticated)?;
    Ok((account, cache))
}
pub fn ensure_closed() -> Result<(), DesktopQuotaError> {
    let entries = fs::read_dir("/proc").map_err(|_| DesktopQuotaError::SourceUnavailable)?;
    for entry in entries {
        let entry = entry.map_err(|_| DesktopQuotaError::SourceUnavailable)?;
        if !entry
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        use std::os::unix::fs::MetadataExt;
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(DesktopQuotaError::SourceUnavailable),
        };
        if metadata.uid() != nix::unistd::Uid::current().as_raw() {
            continue;
        }
        let process = entry.path();
        let is_desktop = match fs::read_link(process.join("exe")) {
            Ok(exe) => exe.file_name().is_some_and(|name| {
                matches!(
                    name.to_str(),
                    Some("claude-desktop" | "claude-desktop (deleted)")
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                match fs::read_to_string(process.join("comm")) {
                    Ok(name) => name.trim_end() == "claude-desktop",
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(_) => return Err(DesktopQuotaError::SourceUnavailable),
                }
            }
            Err(_) => return Err(DesktopQuotaError::SourceUnavailable),
        };
        if is_desktop {
            return Err(DesktopQuotaError::DesktopRunning);
        }
    }
    Ok(())
}

/// `CLAUDE_CONFIG_DIR` the Desktop saved in this profile's environment
/// (`ccd-environment-config.json`), which the Desktop honours before the process environment.
/// Returns `None` when the file or the variable is absent.
pub fn conversation_cli_config_override(
    config_dir: &Path,
) -> Result<Option<String>, DesktopQuotaError> {
    const MAX_BYTES: usize = 1024 * 1024;
    let path = config_dir.join("ccd-environment-config.json");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(DesktopQuotaError::SourceUnavailable),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_BYTES as u64
    {
        return Err(DesktopQuotaError::SourceIncompatible);
    }
    let mut bytes = Vec::new();
    fs::File::open(&path)
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| DesktopQuotaError::SourceIncompatible)?;
    // Accepted shapes: a name → value map, or a list of `{name|key, value}` entries, possibly
    // nested under any object (the exact layout is not documented by the Desktop). A matched
    // value is only trusted when it is an absolute path without `.`/`..` components: an object
    // that merely carries a `name: CLAUDE_CONFIG_DIR` field (a label, not an environment
    // variable) must never yield a directory.
    fn find(value: &serde_json::Value) -> Option<String> {
        fn usable(found: &str) -> Option<String> {
            let path = Path::new(found);
            (path.is_absolute()
                && !path.components().any(|part| {
                    matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )
                }))
            .then(|| found.to_owned())
        }
        match value {
            serde_json::Value::Object(map) => {
                if let Some(found) = map
                    .get("CLAUDE_CONFIG_DIR")
                    .and_then(serde_json::Value::as_str)
                    .and_then(usable)
                {
                    return Some(found);
                }
                let named = map
                    .get("name")
                    .or_else(|| map.get("key"))
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|name| name == "CLAUDE_CONFIG_DIR");
                if named {
                    if let Some(found) = map
                        .get("value")
                        .and_then(serde_json::Value::as_str)
                        .and_then(usable)
                    {
                        return Some(found);
                    }
                    // A named entry with an unusable value is a spent match: do not descend into
                    // it, as its fields describe this variable and nothing else.
                    return None;
                }
                map.values().find_map(find)
            }
            serde_json::Value::Array(items) => items.iter().find_map(find),
            _ => None,
        }
    }
    Ok(find(&value))
}
fn decrypt_cache(cache: &str) -> Result<Vec<u8>, DesktopQuotaError> {
    let bytes = STANDARD
        .decode(cache)
        .map_err(|_| DesktopQuotaError::SourceIncompatible)?;
    let (version, ciphertext) = bytes
        .split_at_checked(3)
        .ok_or(DesktopQuotaError::SourceIncompatible)?;
    let password = match version {
        b"v10" => b"peanuts".to_vec(),
        b"v11" => {
            // secret-tool queries the user's Secret Service; never place secrets in arguments or logs.
            let output = Command::new("timeout")
                .args(["10s", "secret-tool", "lookup", "application", "Claude"])
                .stderr(Stdio::null())
                .output()
                .map_err(|_| DesktopQuotaError::KeyringUnavailable)?;
            if !output.status.success() || output.stdout.len() > 4096 {
                return Err(DesktopQuotaError::KeyringUnavailable);
            }
            let mut password = output.stdout;
            if password.last() == Some(&b'\n') {
                password.pop();
            }
            if password.is_empty() {
                return Err(DesktopQuotaError::KeyringUnavailable);
            }
            password
        }
        _ => return Err(DesktopQuotaError::SourceIncompatible),
    };
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(&password, b"saltysalt", 1, &mut key);
    cbc::Decryptor::<Aes128>::new(&key.into(), &[b' '; 16].into())
        .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
        .map_err(|_| DesktopQuotaError::SourceIncompatible)
}
#[derive(Deserialize)]
struct Token {
    token: String,
    #[serde(rename = "expiresAt")]
    expires_at: i64,
}
fn select_token(data: &[u8], account: &str, now: i64) -> Result<String, DesktopQuotaError> {
    let entries: std::collections::HashMap<String, Token> =
        serde_json::from_slice(data).map_err(|_| DesktopQuotaError::SourceIncompatible)?;
    let prefix = format!("acct:{account}|");
    entries
        .into_iter()
        .filter(|(k, v)| {
            k.starts_with(&prefix)
                && k.split(':').any(|part| part == "user")
                && k.contains("user:profile")
                && v.expires_at > now
                && !v.token.is_empty()
        })
        .max_by_key(|(_, v)| v.expires_at)
        .map(|(_, v)| v.token)
        .ok_or(DesktopQuotaError::Unauthenticated)
}
impl DesktopQuotaSource for LinuxDesktopQuotaSource {
    fn open(&self) -> Result<Box<dyn DesktopQuotaSession + '_>, DesktopQuotaError> {
        ensure_closed()?;
        let path = self.config_dir.join("config.json");
        let (account, cache) = read_config(&path)?;
        let plain = decrypt_cache(&cache)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DesktopQuotaError::SourceUnavailable)?
            .as_millis();
        let token = select_token(
            &plain,
            &account,
            i64::try_from(now).map_err(|_| DesktopQuotaError::SourceUnavailable)?,
        )?;
        ensure_closed()?;
        if read_config(&path)? != (account.clone(), cache.clone()) {
            return Err(DesktopQuotaError::IdentityMismatch);
        }
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .no_proxy()
            .build()
            .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
        Ok(Box::new(Session {
            config: path,
            account,
            cache,
            token,
            confirmed: Cell::new(false),
            client,
        }))
    }
}
impl Session {
    fn guard(&self) -> Result<(), DesktopQuotaError> {
        ensure_closed()?;
        if read_config(&self.config)? != (self.account.clone(), self.cache.clone()) {
            return Err(DesktopQuotaError::IdentityMismatch);
        }
        Ok(())
    }
    fn get(&self, path: &'static str) -> Result<Vec<u8>, DesktopQuotaError> {
        self.guard()?;
        let mut response = self
            .client
            .get(format!("https://api.anthropic.com{path}"))
            .bearer_auth(&self.token)
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("user-agent", "agents-manager")
            .send()
            .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
        match response.status().as_u16() {
            200 => (),
            401 | 403 => return Err(DesktopQuotaError::Unauthenticated),
            429 => return Err(DesktopQuotaError::RateLimited),
            _ => return Err(DesktopQuotaError::SourceUnavailable),
        }
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
        if bytes.len() > 1024 * 1024 {
            return Err(DesktopQuotaError::SourceIncompatible);
        }
        self.guard()?;
        Ok(bytes)
    }
}
#[derive(Deserialize)]
struct Profile {
    account: ProfileAccount,
}
#[derive(Deserialize)]
struct ProfileAccount {
    uuid: String,
    display_name: Option<String>,
    email: Option<String>,
}
#[derive(Deserialize)]
struct Usage {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
}
#[derive(Deserialize)]
struct Window {
    utilization: Option<f64>,
    resets_at: Option<String>,
}
fn parse_usage(bytes: &[u8]) -> Result<DesktopQuotaWindows, DesktopQuotaError> {
    let usage: Usage =
        serde_json::from_slice(bytes).map_err(|_| DesktopQuotaError::SourceIncompatible)?;
    let convert = |w: Window| {
        if let Some(time) = &w.resets_at {
            chrono::DateTime::parse_from_rfc3339(time)
                .map_err(|_| DesktopQuotaError::SourceIncompatible)?;
        }
        Ok(DesktopQuotaWindow {
            utilization: w.utilization,
            resets_at: w.resets_at,
        })
    };
    Ok(DesktopQuotaWindows {
        five_hour: usage.five_hour.map(convert).transpose()?,
        seven_day: usage.seven_day.map(convert).transpose()?,
    })
}
impl DesktopQuotaSession for Session {
    fn identity(&self) -> Result<DesktopAccountIdentity, DesktopQuotaError> {
        self.confirmed.set(false);
        let profile: Profile = serde_json::from_slice(&self.get("/api/oauth/profile")?)
            .map_err(|_| DesktopQuotaError::SourceIncompatible)?;
        if profile.account.uuid.is_empty() || profile.account.uuid != self.account {
            return Err(DesktopQuotaError::IdentityMismatch);
        }
        let display_name = profile
            .account
            .display_name
            .filter(|s| !s.trim().is_empty())
            .or(profile.account.email.filter(|s| !s.trim().is_empty()))
            .ok_or(DesktopQuotaError::SourceIncompatible)?;
        self.confirmed.set(true);
        Ok(DesktopAccountIdentity {
            id: profile.account.uuid,
            display_name,
        })
    }
    fn usage(&self) -> Result<DesktopQuotaWindows, DesktopQuotaError> {
        if !self.confirmed.get() {
            return Err(DesktopQuotaError::IdentityMismatch);
        }
        parse_usage(&self.get("/api/oauth/usage")?)
    }
}
