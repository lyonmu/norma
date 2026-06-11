use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("failed to create log directory {path}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },
    #[error("failed to open log file {path}: {source}")]
    OpenFile { path: PathBuf, source: io::Error },
    #[error("failed to write log file: {0}")]
    Write(io::Error),
    #[error("failed to initialize tracing subscriber: {0}")]
    Subscriber(String),
}
