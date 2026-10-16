//! GoyMusic audio engine: file → symphonia → channel mapping + rubato → ReplayGain → sink.

pub mod convert;
pub mod decoder;
pub mod error;

pub use error::AudioError;
