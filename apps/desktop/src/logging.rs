//! Logs to `~/.local/state/goymusic/logs/`, one file per day, the last 7 kept (spec §9).

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::EnvFilter;

/// Keep the guard alive until exit: dropping it flushes the log.
pub fn init() -> Option<WorkerGuard> {
    let dir = dirs::state_dir()?.join("goymusic/logs");
    // the appender prunes old files at start-up and complains if the folder is missing
    std::fs::create_dir_all(&dir).ok()?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("goymusic")
        .max_log_files(7)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .try_init()
        .ok()?;
    Some(guard)
}
