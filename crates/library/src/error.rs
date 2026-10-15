#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("migration: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("tags in {path}: {message}")]
    Tags { path: String, message: String },
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: i64 },
    #[error("position {position} out of range (length {len})")]
    InvalidPosition { position: usize, len: usize },
    #[error("folder {0} overlaps a library folder")]
    FolderOverlap(String),
    #[error("path is not valid UTF-8: {0}")]
    NonUtf8Path(String),
}

pub type Result<T> = std::result::Result<T, LibraryError>;
