use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use flate2::Compression;
use flate2::write::GzEncoder;
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use crate::config::LoggingConfig;

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("failed to create log directory {path}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },
    #[error("failed to open log file {path}: {source}")]
    OpenFile { path: PathBuf, source: io::Error },
    #[error("failed to write log file: {0}")]
    Write(io::Error),
    #[error("failed to initialize tracing subscriber: {0}")]
    Subscriber(String),
}

#[derive(Debug)]
pub struct TelemetryGuard {
    _worker: WorkerGuard,
}

#[derive(Debug)]
struct RotatingLogWriter {
    inner: Arc<Mutex<RotatingLogState>>,
}

#[derive(Debug)]
struct RotatingLogState {
    dir: PathBuf,
    active_path: PathBuf,
    active_file: File,
    active_bytes: u64,
    max_bytes: u64,
    roll_index: u64,
}

impl RotatingLogWriter {
    fn new(dir: impl AsRef<Path>, max_bytes: u64) -> Result<Self, TelemetryError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir).map_err(|source| TelemetryError::CreateDir {
            path: dir.clone(),
            source,
        })?;
        let active_path = dir.join("norma.log");
        let active_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_path)
            .map_err(|source| TelemetryError::OpenFile {
                path: active_path.clone(),
                source,
            })?;
        let active_bytes = active_file.metadata().map(|meta| meta.len()).unwrap_or(0);
        Ok(Self {
            inner: Arc::new(Mutex::new(RotatingLogState {
                dir,
                active_path,
                active_file,
                active_bytes,
                max_bytes,
                roll_index: 0,
            })),
        })
    }
}

impl Write for RotatingLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self.inner.lock().expect("log writer lock poisoned");
        if state.active_bytes + buf.len() as u64 > state.max_bytes && state.active_bytes > 0 {
            state.roll()?;
        }
        let written = state.active_file.write(buf)?;
        state.active_bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.inner.lock().expect("log writer lock poisoned");
        state.active_file.flush()
    }
}

impl RotatingLogState {
    fn roll(&mut self) -> io::Result<()> {
        self.active_file.flush()?;
        let rolled_path = self.dir.join(format!("norma.{}.log", self.roll_index));
        self.roll_index += 1;
        if rolled_path.exists() {
            fs::remove_file(&rolled_path)?;
        }
        fs::rename(&self.active_path, rolled_path)?;
        self.active_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.active_path)?;
        self.active_bytes = 0;
        Ok(())
    }
}

pub fn init_tracing(
    log_dir: impl AsRef<Path>,
    logging: &LoggingConfig,
) -> Result<TelemetryGuard, TelemetryError> {
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
        .map_err(|error| TelemetryError::Subscriber(error.to_string()))?;
    Ok(TelemetryGuard { _worker: guard })
}

pub fn start_log_maintenance(log_dir: PathBuf, logging: LoggingConfig) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(
                logging.maintenance_interval_hours.saturating_mul(60 * 60),
            ));
            let _ = maintain_logs(&log_dir, logging.retention_days, logging.compress_rotated);
        }
    })
}

pub fn maintain_logs(
    log_dir: impl AsRef<Path>,
    retention_days: u64,
    compress_rotated: bool,
) -> io::Result<()> {
    let log_dir = log_dir.as_ref();
    let retention = Duration::from_secs(retention_days.saturating_mul(24 * 60 * 60));
    let now = SystemTime::now();
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if file_name == "norma.log" {
            continue;
        }
        let metadata = entry.metadata()?;
        let age = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .unwrap_or_default();
        if age > retention {
            fs::remove_file(path)?;
            continue;
        }
        if compress_rotated && file_name.ends_with(".log") {
            compress_log_file(&path)?;
        }
    }
    Ok(())
}

fn compress_log_file(path: &Path) -> io::Result<()> {
    let compressed = path.with_extension("log.gz");
    let mut input = File::open(path)?;
    let output = File::create(&compressed)?;
    let mut encoder = GzEncoder::new(output, Compression::default());
    io::copy(&mut input, &mut encoder)?;
    encoder.finish()?;
    fs::remove_file(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn rotating_writer_rolls_when_size_limit_is_reached() {
        let root = tempfile::tempdir().unwrap();
        let mut writer = RotatingLogWriter::new(root.path(), 12).unwrap();

        writer.write_all(b"{\"a\":1}\n").unwrap();
        writer.write_all(b"{\"b\":2}\n").unwrap();
        writer.flush().unwrap();

        assert!(root.path().join("norma.log").is_file());
        assert!(root.path().join("norma.0.log").is_file());
    }

    #[test]
    fn maintenance_compresses_rolled_logs() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("norma.0.log"), "{\"a\":1}\n").unwrap();

        maintain_logs(root.path(), 7, true).unwrap();

        assert!(!root.path().join("norma.0.log").exists());
        assert!(root.path().join("norma.0.log.gz").is_file());
    }

    #[test]
    #[ignore = "global tracing subscriber can only be initialized once per process"]
    fn init_tracing_accepts_valid_logging_config() {
        let root = tempfile::tempdir().unwrap();
        let logging = test_logging();
        let guard = init_tracing(root.path(), &logging).unwrap();
        tracing::info!(event = "telemetry_test", "structured test log");
        drop(guard);

        assert!(root.path().join("norma.log").is_file());
    }
}
