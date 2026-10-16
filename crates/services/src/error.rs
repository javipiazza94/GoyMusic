use goymusic_library::error::LibraryError;
use serde::Serialize;

/// Error returned by every service call: a stable code for the UI plus a readable message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
#[error("{code:?}: {message}")]
pub struct ServiceError {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    NotFound,
    InvalidInput,
    FolderOverlap,
    Io,
    Database,
    Internal,
}

impl ServiceError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
}

impl From<LibraryError> for ServiceError {
    fn from(e: LibraryError) -> Self {
        let code = match &e {
            LibraryError::NotFound { .. } => ErrorCode::NotFound,
            LibraryError::InvalidPosition { .. } => ErrorCode::InvalidInput,
            LibraryError::FolderOverlap(_) => ErrorCode::FolderOverlap,
            LibraryError::Io(_) | LibraryError::NonUtf8Path(_) => ErrorCode::Io,
            LibraryError::Db(_) | LibraryError::Migration(_) | LibraryError::Json(_) | LibraryError::Tags { .. } => {
                ErrorCode::Database
            }
        };
        Self::new(code, e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ServiceError>;
