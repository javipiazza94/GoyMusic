//! GoyMusic library: SQLite storage, folder scanning, playlists, search.

pub mod art;
pub mod db;
pub mod error;
pub mod folders;
pub mod history;
pub mod m3u;
pub mod playlists;
pub mod queries;
pub mod scanner;
pub mod search;
pub mod state;
pub mod tags;

mod maintenance;
mod moves;
