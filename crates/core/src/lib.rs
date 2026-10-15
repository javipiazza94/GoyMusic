//! GoyMusic core: domain types, playback queue and event bus. Pure logic, no I/O.

pub mod events;
pub mod ids;
pub mod model;
pub mod queue;

pub use ids::*;
