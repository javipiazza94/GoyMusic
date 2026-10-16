use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("cannot open {path}: {reason}")]
    Open { path: String, reason: String },
    #[error("unsupported audio: {0}")]
    Unsupported(String),
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("resampler: {0}")]
    Resample(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;
