use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;
use crate::logging::{LoggingError, RotatingLogWriter};

#[derive(Debug)]
pub struct LoggingGuard {
    _worker: WorkerGuard,
}

pub fn init_tracing(
    log_dir: impl AsRef<Path>,
    logging: &LoggingConfig,
) -> Result<LoggingGuard, LoggingError> {
    let log_dir = log_dir.as_ref();
    let max_bytes = logging.max_file_size_mb.saturating_mul(1024 * 1024).max(1);
    let writer = RotatingLogWriter::new(log_dir, max_bytes)?;
    let (non_blocking, guard) = tracing_appender::non_blocking(writer);
    let filter =
        EnvFilter::try_new(logging.level.as_str()).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .try_init()
        .map_err(|error| LoggingError::Subscriber(error.to_string()))?;
    tracing::info!(component = "logging", log_dir = %log_dir.display(), level = %logging.level, format = %logging.format, "tracing initialized");
    Ok(LoggingGuard { _worker: guard })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LoggingConfig;

    fn test_logging() -> LoggingConfig {
        LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            max_file_size_mb: 1,
            maintenance_interval_hours: 24,
            retention_days: 7,
            compress_rotated: true,
        }
    }

    #[test]
    #[ignore = "global tracing subscriber can only be initialized once per process"]
    fn init_tracing_accepts_valid_logging_config() {
        let root = tempfile::tempdir().unwrap();
        let logging = test_logging();
        let guard = init_tracing(root.path(), &logging).unwrap();
        tracing::info!(event = "logging_test", "structured test log");
        drop(guard);

        assert!(root.path().join("norma.log").is_file());
    }
}
