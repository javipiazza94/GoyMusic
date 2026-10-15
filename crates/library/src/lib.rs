//! GoyMusic library: SQLite storage, folder scanning, playlists, search.

pub mod db;
pub mod error;
pub mod folders;
pub mod m3u;
pub mod playlists;
pub mod queries;
pub mod scanner;
pub mod search;
pub mod tags;

mod maintenance;
mod moves;
