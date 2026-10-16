//! Log lokal: file harian di `<data_dir>/logs` (+ stdout di debug build). Tanpa telemetry.

use std::path::Path;

use anyhow::Context;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Guard harus dipegang selama app hidup agar log ter-flush.
pub fn init_logging(dir: &Path) -> anyhow::Result<WorkerGuard> {
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs)?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("nabungo")
        .filename_suffix("log")
        .max_log_files(14)
        .build(&logs)
        .context("gagal membuat file log")?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer().with_ansi(false).with_writer(writer);
    let stdout_layer = cfg!(debug_assertions).then(fmt::layer);
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
        .context("gagal memasang subscriber log")?;
    Ok(guard)
}
