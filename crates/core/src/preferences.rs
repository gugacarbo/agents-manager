use std::{fs, io::Write, path::Path};

use crate::{domain::AppPreferences, error::CoreError};

/// Loads persisted desktop preferences from `path`, returning defaults when absent.
pub fn load_preferences(path: &Path) -> Result<AppPreferences, CoreError> {
    match fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(AppPreferences::default()),
        Err(err) => Err(err.into()),
    }
}

/// Atomically persists desktop `preferences` to `path`.
pub fn save_preferences(
    path: &Path,
    preferences: AppPreferences,
) -> Result<AppPreferences, CoreError> {
    let parent = path.parent().ok_or_else(|| CoreError::InvalidPath {
        path: path.display().to_string(),
    })?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&serde_json::to_vec_pretty(&preferences)?)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .map_err(|err| CoreError::from(err.error))?;
    Ok(preferences)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_preferences_file_returns_default_enabled() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("preferences.json");

        let prefs = load_preferences(&path).unwrap();
        assert_eq!(
            prefs,
            AppPreferences {
                minimize_to_tray: true
            }
        );
    }

    #[test]
    fn empty_json_object_deserializes_with_default_enabled() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("preferences.json");
        fs::write(&path, "{}").unwrap();

        let prefs = load_preferences(&path).unwrap();
        assert_eq!(
            prefs,
            AppPreferences {
                minimize_to_tray: true
            }
        );
    }

    #[test]
    fn save_preferences_round_trips_boolean_values() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("nested/preferences.json");

        let saved_false = save_preferences(
            &path,
            AppPreferences {
                minimize_to_tray: false,
            },
        )
        .unwrap();
        assert_eq!(
            saved_false,
            AppPreferences {
                minimize_to_tray: false
            }
        );
        assert_eq!(
            load_preferences(&path).unwrap(),
            AppPreferences {
                minimize_to_tray: false
            }
        );

        let saved_true = save_preferences(
            &path,
            AppPreferences {
                minimize_to_tray: true,
            },
        )
        .unwrap();
        assert_eq!(
            saved_true,
            AppPreferences {
                minimize_to_tray: true
            }
        );
        assert_eq!(
            load_preferences(&path).unwrap(),
            AppPreferences {
                minimize_to_tray: true
            }
        );
    }
}
