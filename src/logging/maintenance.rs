use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use flate2::Compression;
use flate2::write::GzEncoder;

use crate::config::LoggingConfig;

pub fn start_log_maintenance(log_dir: PathBuf, logging: LoggingConfig) -> thread::JoinHandle<()> {
    tracing::info!(
        component = "logging",
        log_dir = %log_dir.display(),
        interval_hours = logging.maintenance_interval_hours,
        "log maintenance worker started"
    );
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(
                logging.maintenance_interval_hours.saturating_mul(60 * 60),
            ));
            if let Err(error) =
                maintain_logs(&log_dir, logging.retention_days, logging.compress_rotated)
            {
                tracing::warn!(
                    component = "logging",
                    log_dir = %log_dir.display(),
                    error = %error,
                    "log maintenance failed"
                );
            }
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
    let mut removed = 0usize;
    let mut compressed = 0usize;

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
            fs::remove_file(&path)?;
            removed += 1;
            tracing::debug!(component = "logging", path = %path.display(), "expired log removed");
            continue;
        }
        if compress_rotated && file_name.ends_with(".log") {
            compress_log_file(&path)?;
            compressed += 1;
        }
    }

    tracing::info!(
        component = "logging",
        log_dir = %log_dir.display(),
        removed,
        compressed,
        "log maintenance completed"
    );
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
    tracing::debug!(component = "logging", path = %path.display(), compressed_path = %compressed.display(), "log file compressed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maintenance_compresses_rolled_logs() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("norma.0.log"), "{\"a\":1}\n").unwrap();

        maintain_logs(root.path(), 7, true).unwrap();

        assert!(!root.path().join("norma.0.log").exists());
        assert!(root.path().join("norma.0.log.gz").is_file());
    }
}
