use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

/// Domain-level errors produced by `agents-manager-core`.
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize, Type)]
pub enum CoreError {
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
        Self::Io {
            message: err.to_string(),
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
