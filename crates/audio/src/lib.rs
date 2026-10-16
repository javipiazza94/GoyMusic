//! GoyMusic audio engine: file → symphonia → channel mapping + rubato → ReplayGain → sink.

pub mod convert;
pub mod decoder;
pub mod engine;
pub mod error;
pub mod gain;
pub mod sink;

pub use error::AudioError;
