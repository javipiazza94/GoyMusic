//! GoyMusic audio engine: file → symphonia → channel mapping + rubato → ReplayGain → sink.

pub mod decoder;
pub mod error;

pub use error::AudioError;
