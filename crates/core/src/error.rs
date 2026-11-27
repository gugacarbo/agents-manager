use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Domain-level errors produced by `agents-manager-core`.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, Type)]
pub enum CoreError {
    #[error("literal secret confirmation required")]
    SecretConfirmationRequired,
    #[error("project already registered: {root_path}")]
    DuplicateProject { id: String, root_path: String },
    #[error("permission denied: {message}")]
    PermissionDenied { message: String },
    #[error("path not allowed: {path}")]
    PathNotAllowed { path: String },
    #[error("syntax error: {message}")]
    SyntaxError { message: String },
    #[error("file changed externally")]
    ExternalConflict,
    #[error("I/O error: {message}")]
    IoError { message: String },
    /// Filesystem or I/O operation failure.
    #[error("I/O error: {message}")]
    Io { message: String },

    /// Serialization or deserialization failure.
    #[error("serialization error: {message}")]
    Serialization { message: String },

    /// Invalid or inaccessible filesystem path.
    #[error("invalid path: {path}")]
    InvalidPath { path: String },

    /// Expected resource was not found.
    #[error("not found: {target}")]
    NotFound { target: String },
}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound {
                target: err.to_string(),
            },
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied {
                message: err.to_string(),
            },
            _ => Self::IoError {
                message: err.to_string(),
            },
        }
    }
}

impl From<serde_json::Error> for CoreError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization {
            message: err.to_string(),
        }
    }
}
