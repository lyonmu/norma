use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::logging::LoggingError;

#[derive(Debug)]
pub(crate) struct RotatingLogWriter {
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
    pub(crate) fn new(dir: impl AsRef<Path>, max_bytes: u64) -> Result<Self, LoggingError> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir).map_err(|source| LoggingError::CreateDir {
            path: dir.clone(),
            source,
        })?;
        let active_path = dir.join("norma.log");
        let active_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_path)
            .map_err(|source| LoggingError::OpenFile {
                path: active_path.clone(),
                source,
            })?;
        let active_bytes = active_file.metadata().map(|meta| meta.len()).unwrap_or(0);
        tracing::debug!(
            component = "logging",
            path = %active_path.display(),
            active_bytes,
            max_bytes,
            "active log opened"
        );
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
        fs::rename(&self.active_path, &rolled_path)?;
        self.active_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.active_path)?;
        self.active_bytes = 0;
        tracing::debug!(
            component = "logging",
            rolled_path = %rolled_path.display(),
            active_path = %self.active_path.display(),
            "log file rolled"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
