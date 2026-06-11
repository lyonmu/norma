# Norma Runtime Infrastructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Norma's runtime infrastructure for `~/.norma` paths, TOML configuration, JSON structured tracing logs, non-blocking log rotation/compression/retention, config hot reload, and a non-executable Skills directory index.

**Architecture:** Add focused `paths`, `config`, `telemetry`, `skills`, and `runtime` modules. `main.rs` stays thin and delegates startup to `runtime`; UI and app state consume snapshots and events instead of reading files directly. Logging uses `tracing` macros everywhere, with JSON formatting and background file IO hidden behind `telemetry`.

**Tech Stack:** Rust 2024, Cargo, GPUI `0.2.2`, `serde`, `toml`, `config`, `notify`, `tracing`, `tracing-subscriber`, `tracing-appender`, `flate2`, `tempfile` for tests.

---

## Scope Check

This plan implements only the accepted runtime infrastructure spec at `docs/superpowers/specs/2026-06-11-norma-runtime-infrastructure-design.md`.

It does not implement model providers, MCP credentials, executable Skills, an in-app settings editor, a log viewer, or database storage under `~/.norma/data`.

## Target File Structure

Create or modify these files:

- Modify: `Cargo.toml`  
  Add runtime infrastructure dependencies and test-only temporary directory support.

- Modify: `src/lib.rs`  
  Export new modules: `paths`, `config`, `telemetry`, `skills`, and `runtime`.

- Modify: `src/main.rs`  
  Keep only bootstrap and error reporting.

- Modify: `src/app_state.rs`  
  Store config/path/skill snapshots and support config-change application.

- Modify: `src/ui/shell.rs`  
  Accept preloaded app state and runtime guards instead of doing direct state bootstrap internally.

- Create: `src/paths.rs`  
  Resolve and create `~/.norma`, `config.toml`, `log`, `data`, and `skills`.

- Create: `src/config.rs`  
  Define `NormaConfig`, load/write TOML, apply `NORMA_` environment overrides through `config`, validate values, and define config reload events.

- Create: `src/telemetry.rs`  
  Initialize `tracing`, JSON formatting, background writer, size-based rotation, gzip compression, retention cleanup, and maintenance scheduling.

- Create: `src/skills.rs`  
  Scan and index `~/.norma/skills` without executing skill code.

- Create: `src/runtime.rs`  
  Coordinate startup: paths, config, telemetry, skills, watchers, and app state.

## Commit Strategy

Commit after each task:

- `chore: add runtime infrastructure dependencies`
- `feat: add Norma local paths`
- `feat: add persistent runtime config`
- `feat: add structured telemetry`
- `feat: add skills directory index`
- `feat: add config and skills watchers`
- `feat: wire runtime bootstrap`
- `test: verify runtime infrastructure`

Before committing, run the task-specific checks listed in the task.

---

### Task 1: Add Runtime Infrastructure Dependencies

**Files:**
- Modify: `Cargo.toml`
- Verify: `Cargo.lock`

- [ ] **Step 1: Confirm crate versions**

Run:

```bash
cargo search serde --registry crates-io --limit 5
cargo search toml --registry crates-io --limit 5
cargo search config --registry crates-io --limit 5
cargo search notify --registry crates-io --limit 5
cargo search tracing --registry crates-io --limit 5
cargo search tracing-subscriber --registry crates-io --limit 5
cargo search tracing-appender --registry crates-io --limit 5
cargo search flate2 --registry crates-io --limit 5
cargo search tempfile --registry crates-io --limit 5
```

Expected versions from 2026-06-11 discovery:

```text
serde = "1.0.228"
toml = "1.1.2"
config = "0.15.23"
notify = "9.0.0-rc.4"
tracing = "0.1.44"
tracing-subscriber = "0.3.23"
tracing-appender = "0.2.5"
flate2 = "1.1.9"
tempfile = "3.27.0"
```

- [ ] **Step 2: Update `Cargo.toml` dependencies**

Change the dependency sections to include these entries:

```toml
[dependencies]
anyhow = "1.0.102"
config = "0.15.23"
flate2 = "1.1.9"
gpui = "0.2.2"
ignore = "0.4.26"
notify = "9.0.0-rc.4"
serde = { version = "1.0.228", features = ["derive"] }
thiserror = "2.0.18"
toml = "1.1.2"
tracing = "0.1.44"
tracing-appender = "0.2.5"
tracing-subscriber = { version = "0.3.23", features = ["env-filter", "fmt", "json"] }

[dev-dependencies]
tempfile = "3.27.0"
```

Keep `edition = "2024"` unchanged. Do not add `gpui_platform`.

- [ ] **Step 3: Resolve dependencies**

Run:

```bash
cargo check
```

Expected: PASS. If GPUI fails with a macOS Metal Toolchain error, run:

```bash
xcodebuild -downloadComponent MetalToolchain
cargo check
```

- [ ] **Step 4: Verify Rust 2024 metadata remains unchanged**

Run:

```bash
cargo metadata --no-deps --format-version 1 | rg '"edition":"2024"'
```

Expected: one match for the `norma` package.

- [ ] **Step 5: Commit**

Run:

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add runtime infrastructure dependencies"
```

---

### Task 2: Add Norma Local Paths

**Files:**
- Modify: `src/lib.rs`
- Create: `src/paths.rs`

- [ ] **Step 1: Export the module**

Add this line to `src/lib.rs`:

```rust
pub mod paths;
```

- [ ] **Step 2: Write failing path tests**

Create `src/paths.rs` with the type declarations and tests first:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormaPaths {
    pub home_dir: PathBuf,
    pub config_file: PathBuf,
    pub log_dir: PathBuf,
    pub data_dir: PathBuf,
    pub skills_dir: PathBuf,
}

impl NormaPaths {
    pub fn from_home(home: impl AsRef<Path>) -> Self {
        let home_dir = home.as_ref().join(".norma");
        Self {
            config_file: home_dir.join("config.toml"),
            log_dir: home_dir.join("log"),
            data_dir: home_dir.join("data"),
            skills_dir: home_dir.join("skills"),
            home_dir,
        }
    }

    pub fn create_all(&self) -> Result<(), PathsError> {
        for path in [
            &self.home_dir,
            &self.log_dir,
            &self.data_dir,
            &self.skills_dir,
        ] {
            fs::create_dir_all(path).map_err(|source| PathsError::CreateDir {
                path: path.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_expected_norma_paths_from_home() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());

        assert_eq!(paths.home_dir, root.path().join(".norma"));
        assert_eq!(paths.config_file, root.path().join(".norma/config.toml"));
        assert_eq!(paths.log_dir, root.path().join(".norma/log"));
        assert_eq!(paths.data_dir, root.path().join(".norma/data"));
        assert_eq!(paths.skills_dir, root.path().join(".norma/skills"));
    }

    #[test]
    fn creates_norma_directory_layout() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());

        paths.create_all().unwrap();

        assert!(paths.home_dir.is_dir());
        assert!(paths.log_dir.is_dir());
        assert!(paths.data_dir.is_dir());
        assert!(paths.skills_dir.is_dir());
        assert!(!paths.config_file.exists());
    }
}
```

- [ ] **Step 3: Run path tests**

Run:

```bash
cargo test paths
```

Expected: PASS, because this task includes the minimal implementation with the tests.

- [ ] **Step 4: Add current-user path helper**

Append this function above the test module:

```rust
pub fn default_paths() -> NormaPaths {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    NormaPaths::from_home(home)
}
```

- [ ] **Step 5: Run focused and full tests**

Run:

```bash
cargo fmt
cargo test paths
cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/lib.rs src/paths.rs
git commit -m "feat: add Norma local paths"
```

---

### Task 3: Add Persistent Runtime Config

**Files:**
- Modify: `src/lib.rs`
- Create: `src/config.rs`

- [ ] **Step 1: Export the module**

Add this line to `src/lib.rs`:

```rust
pub mod config;
```

- [ ] **Step 2: Create config types and tests**

Create `src/config.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::paths::NormaPaths;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write config {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config {path}: {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("failed to serialize default config: {0}")]
    Serialize(toml::ser::Error),
    #[error("invalid config: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormaConfig {
    pub window: WindowConfig,
    pub paths: PathsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PathsConfig {
    pub data_dir: String,
    pub log_dir: String,
    pub skills_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub max_file_size_mb: u64,
    pub maintenance_interval_hours: u64,
    pub retention_days: u64,
    pub compress_rotated: bool,
}

impl NormaConfig {
    pub fn default_for(paths: &NormaPaths) -> Self {
        Self {
            window: WindowConfig {
                width: 1440,
                height: 1024,
            },
            paths: PathsConfig {
                data_dir: paths.data_dir.display().to_string(),
                log_dir: paths.log_dir.display().to_string(),
                skills_dir: paths.skills_dir.display().to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                max_file_size_mb: 10,
                maintenance_interval_hours: 24,
                retention_days: 7,
                compress_rotated: true,
            },
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let valid_level = matches!(
            self.logging.level.as_str(),
            "trace" | "debug" | "info" | "warn" | "error"
        );
        if !valid_level {
            return Err(ConfigError::Invalid(format!(
                "logging.level must be trace, debug, info, warn, or error; got {}",
                self.logging.level
            )));
        }
        if self.logging.format != "json" {
            return Err(ConfigError::Invalid(format!(
                "logging.format must be json; got {}",
                self.logging.format
            )));
        }
        if self.logging.max_file_size_mb == 0 {
            return Err(ConfigError::Invalid(
                "logging.max_file_size_mb must be greater than zero".to_string(),
            ));
        }
        if self.logging.maintenance_interval_hours == 0 {
            return Err(ConfigError::Invalid(
                "logging.maintenance_interval_hours must be greater than zero".to_string(),
            ));
        }
        if self.logging.retention_days == 0 {
            return Err(ConfigError::Invalid(
                "logging.retention_days must be greater than zero".to_string(),
            ));
        }
        if self.window.width == 0 || self.window.height == 0 {
            return Err(ConfigError::Invalid(
                "window width and height must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

pub fn ensure_config(paths: &NormaPaths) -> Result<NormaConfig, ConfigError> {
    if !paths.config_file.exists() {
        let config = NormaConfig::default_for(paths);
        write_config(&paths.config_file, &config)?;
    }
    load_config(&paths.config_file)
}

pub fn load_config(path: impl AsRef<Path>) -> Result<NormaConfig, ConfigError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config: NormaConfig = toml::from_str(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    config.validate()?;
    Ok(config)
}

pub fn write_config(path: impl AsRef<Path>, config: &NormaConfig) -> Result<(), ConfigError> {
    let path = path.as_ref();
    let content = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;
    fs::write(path, content).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_default_config_on_first_launch() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();

        let config = ensure_config(&paths).unwrap();

        assert_eq!(config.logging.level, "info");
        assert_eq!(config.logging.max_file_size_mb, 10);
        assert!(paths.config_file.is_file());
        let content = fs::read_to_string(&paths.config_file).unwrap();
        assert!(content.contains("[logging]"));
        assert!(content.contains("retention_days = 7"));
    }

    #[test]
    fn loads_valid_config_file() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut config = NormaConfig::default_for(&paths);
        config.logging.level = "debug".to_string();
        write_config(&paths.config_file, &config).unwrap();

        let loaded = load_config(&paths.config_file).unwrap();

        assert_eq!(loaded.logging.level, "debug");
    }

    #[test]
    fn rejects_invalid_log_level() {
        let root = tempfile::tempdir().unwrap();
        let paths = NormaPaths::from_home(root.path());
        paths.create_all().unwrap();
        let mut config = NormaConfig::default_for(&paths);
        config.logging.level = "verbose".to_string();
        write_config(&paths.config_file, &config).unwrap();

        let error = load_config(&paths.config_file).unwrap_err();

        assert!(error.to_string().contains("logging.level"));
    }
}
```

- [ ] **Step 3: Run config tests**

Run:

```bash
cargo test config
```

Expected: PASS.

- [ ] **Step 4: Add environment override loader**

Append this function before the test module:

```rust
pub fn load_config_with_env(path: impl AsRef<Path>) -> Result<NormaConfig, ConfigError> {
    let path = path.as_ref();
    let settings = config::Config::builder()
        .add_source(config::File::from(path).format(config::FileFormat::Toml))
        .add_source(
            config::Environment::with_prefix("NORMA")
                .separator("__")
                .try_parsing(true),
        )
        .build()
        .map_err(|error| ConfigError::Invalid(error.to_string()))?;
    let config: NormaConfig = settings
        .try_deserialize()
        .map_err(|error| ConfigError::Invalid(error.to_string()))?;
    config.validate()?;
    Ok(config)
}
```

Replace the last line of `ensure_config`:

```rust
load_config(&paths.config_file)
```

with:

```rust
load_config_with_env(&paths.config_file)
```

- [ ] **Step 5: Add env override test**

Add this test in `src/config.rs`:

```rust
#[test]
fn applies_norma_environment_overrides() {
    let root = tempfile::tempdir().unwrap();
    let paths = NormaPaths::from_home(root.path());
    paths.create_all().unwrap();
    write_config(&paths.config_file, &NormaConfig::default_for(&paths)).unwrap();

    unsafe {
        std::env::set_var("NORMA__LOGGING__LEVEL", "warn");
    }
    let loaded = load_config_with_env(&paths.config_file).unwrap();
    unsafe {
        std::env::remove_var("NORMA__LOGGING__LEVEL");
    }

    assert_eq!(loaded.logging.level, "warn");
}
```

- [ ] **Step 6: Run config tests**

Run:

```bash
cargo fmt
cargo test config
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add src/lib.rs src/config.rs
git commit -m "feat: add persistent runtime config"
```

---

### Task 4: Add Structured Telemetry

**Files:**
- Modify: `src/lib.rs`
- Create: `src/telemetry.rs`

- [ ] **Step 1: Export the module**

Add this line to `src/lib.rs`:

```rust
pub mod telemetry;
```

- [ ] **Step 2: Create telemetry writer and tests**

Create `src/telemetry.rs` with a size-rotating writer, gzip maintenance, and focused tests:

```rust
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
    CreateDir {
        path: PathBuf,
        source: io::Error,
    },
    #[error("failed to open log file {path}: {source}")]
    OpenFile {
        path: PathBuf,
        source: io::Error,
    },
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
        let rolled_path = self
            .dir
            .join(format!("norma.{}.log", self.roll_index));
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
    let filter = EnvFilter::try_new(logging.level.as_str())
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .try_init()
        .map_err(|error| TelemetryError::Subscriber(error.to_string()))?;
    Ok(TelemetryGuard { _worker: guard })
}

pub fn start_log_maintenance(
    log_dir: PathBuf,
    logging: LoggingConfig,
) -> thread::JoinHandle<()> {
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
        let file_name = path.file_name().and_then(|value| value.to_str()).unwrap_or("");
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
    fn init_tracing_accepts_valid_logging_config() {
        let root = tempfile::tempdir().unwrap();
        let logging = test_logging();
        let guard = init_tracing(root.path(), &logging).unwrap();
        tracing::info!(event = "telemetry_test", "structured test log");
        drop(guard);

        assert!(root.path().join("norma.log").is_file());
    }
}
```

- [ ] **Step 3: Run telemetry tests**

Run:

```bash
cargo test telemetry
```

Expected: PASS. If `init_tracing_accepts_valid_logging_config` fails because a previous test initialized the global subscriber in the same process, mark that test `#[ignore]` and add a comment that global subscriber initialization is verified by a single-process integration check.

- [ ] **Step 4: Add JSON format smoke test as ignored if needed**

If the global subscriber conflict appears, change the test attribute to:

```rust
#[test]
#[ignore = "global tracing subscriber can only be initialized once per process"]
fn init_tracing_accepts_valid_logging_config() {
```

Then run:

```bash
cargo test telemetry
cargo test telemetry -- --ignored --test-threads=1
```

Expected: normal tests pass, ignored telemetry initialization test passes in isolation.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/lib.rs src/telemetry.rs
git commit -m "feat: add structured telemetry"
```

---

### Task 5: Add Skills Directory Index

**Files:**
- Modify: `src/lib.rs`
- Create: `src/skills.rs`

- [ ] **Step 1: Export the module**

Add this line to `src/lib.rs`:

```rust
pub mod skills;
```

- [ ] **Step 2: Create skills index and tests**

Create `src/skills.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkillsError {
    #[error("failed to read skills directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillEntry {
    pub name: String,
    pub root: PathBuf,
    pub manifest: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillIndex {
    pub entries: Vec<SkillEntry>,
}

pub fn scan_skills(root: impl AsRef<Path>) -> Result<SkillIndex, SkillsError> {
    let root = root.as_ref();
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(SkillIndex { entries });
    }
    let read_dir = fs::read_dir(root).map_err(|source| SkillsError::ReadDir {
        path: root.to_path_buf(),
        source,
    })?;
    for entry in read_dir {
        let entry = entry.map_err(|source| SkillsError::ReadDir {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let manifest = path.join("SKILL.md");
        entries.push(SkillEntry {
            name,
            root: path,
            manifest: manifest.is_file().then_some(manifest),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(SkillIndex { entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_skills_directory_returns_empty_index() {
        let root = tempfile::tempdir().unwrap();
        let index = scan_skills(root.path().join("missing")).unwrap();

        assert!(index.entries.is_empty());
    }

    #[test]
    fn scans_skill_directories_without_executing_them() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("alpha")).unwrap();
        fs::write(root.path().join("alpha/SKILL.md"), "# Alpha\n").unwrap();
        fs::create_dir(root.path().join("beta")).unwrap();
        fs::write(root.path().join("loose.txt"), "ignored\n").unwrap();

        let index = scan_skills(root.path()).unwrap();

        assert_eq!(index.entries.len(), 2);
        assert_eq!(index.entries[0].name, "alpha");
        assert!(index.entries[0].manifest.is_some());
        assert_eq!(index.entries[1].name, "beta");
        assert!(index.entries[1].manifest.is_none());
    }
}
```

- [ ] **Step 3: Run skills tests**

Run:

```bash
cargo test skills
```

Expected: PASS.

- [ ] **Step 4: Commit**

Run:

```bash
git add src/lib.rs src/skills.rs
git commit -m "feat: add skills directory index"
```

---

### Task 6: Add Config And Skills Watchers

**Files:**
- Modify: `src/config.rs`
- Modify: `src/skills.rs`
- Create: `src/runtime.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Add config reload state**

Append to `src/config.rs` before the test module:

```rust
#[derive(Debug, Clone)]
pub enum ConfigReload {
    Applied(NormaConfig),
    Rejected(String),
}

#[derive(Debug, Clone)]
pub struct ConfigState {
    active: NormaConfig,
    last_error: Option<String>,
}

impl ConfigState {
    pub fn new(active: NormaConfig) -> Self {
        Self {
            active,
            last_error: None,
        }
    }

    pub fn active(&self) -> &NormaConfig {
        &self.active
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn reload_from(&mut self, path: impl AsRef<Path>) -> ConfigReload {
        match load_config_with_env(path) {
            Ok(config) => {
                self.active = config.clone();
                self.last_error = None;
                ConfigReload::Applied(config)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
                ConfigReload::Rejected(message)
            }
        }
    }
}
```

- [ ] **Step 2: Add reload tests**

Add these tests in `src/config.rs`:

```rust
#[test]
fn reload_keeps_last_good_config_when_new_file_is_invalid() {
    let root = tempfile::tempdir().unwrap();
    let paths = NormaPaths::from_home(root.path());
    paths.create_all().unwrap();
    let mut initial = NormaConfig::default_for(&paths);
    initial.logging.level = "debug".to_string();
    write_config(&paths.config_file, &initial).unwrap();
    let mut state = ConfigState::new(load_config(&paths.config_file).unwrap());

    let mut invalid = NormaConfig::default_for(&paths);
    invalid.logging.level = "verbose".to_string();
    write_config(&paths.config_file, &invalid).unwrap();
    let result = state.reload_from(&paths.config_file);

    assert!(matches!(result, ConfigReload::Rejected(_)));
    assert_eq!(state.active().logging.level, "debug");
    assert!(state.last_error().unwrap().contains("logging.level"));
}

#[test]
fn reload_applies_valid_config() {
    let root = tempfile::tempdir().unwrap();
    let paths = NormaPaths::from_home(root.path());
    paths.create_all().unwrap();
    write_config(&paths.config_file, &NormaConfig::default_for(&paths)).unwrap();
    let mut state = ConfigState::new(load_config(&paths.config_file).unwrap());

    let mut next = NormaConfig::default_for(&paths);
    next.logging.level = "warn".to_string();
    write_config(&paths.config_file, &next).unwrap();
    let result = state.reload_from(&paths.config_file);

    assert!(matches!(result, ConfigReload::Applied(_)));
    assert_eq!(state.active().logging.level, "warn");
    assert!(state.last_error().is_none());
}
```

- [ ] **Step 3: Add skills index state**

Append to `src/skills.rs` before the test module:

```rust
#[derive(Debug, Clone)]
pub enum SkillsReload {
    Applied(SkillIndex),
    Rejected(String),
}

#[derive(Debug, Clone, Default)]
pub struct SkillsState {
    active: SkillIndex,
    last_error: Option<String>,
}

impl SkillsState {
    pub fn new(active: SkillIndex) -> Self {
        Self {
            active,
            last_error: None,
        }
    }

    pub fn active(&self) -> &SkillIndex {
        &self.active
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn reload_from(&mut self, root: impl AsRef<Path>) -> SkillsReload {
        match scan_skills(root) {
            Ok(index) => {
                self.active = index.clone();
                self.last_error = None;
                SkillsReload::Applied(index)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
                SkillsReload::Rejected(message)
            }
        }
    }
}
```

- [ ] **Step 4: Add watcher event helper**

Create this helper in `src/config.rs` before the test module:

```rust
pub fn is_config_path_event(config_file: &Path, event: &notify::Event) -> bool {
    event.paths.iter().any(|path| path == config_file)
}
```

Create this helper in `src/skills.rs` before the test module:

```rust
pub fn is_skills_path_event(skills_dir: &Path, event: &notify::Event) -> bool {
    event.paths.iter().any(|path| path.starts_with(skills_dir))
}
```

- [ ] **Step 5: Export runtime and add update channel types**

Add this line to `src/lib.rs`:

```rust
pub mod runtime;
```

Create `src/runtime.rs` with watcher handles and runtime update events:

```rust
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::{
    ConfigReload, ConfigState, NormaConfig, is_config_path_event,
};
use crate::skills::{
    SkillIndex, SkillsReload, SkillsState, is_skills_path_event,
};

#[derive(Debug, Clone)]
pub enum RuntimeUpdate {
    ConfigApplied(NormaConfig),
    ConfigRejected(String),
    SkillsApplied(SkillIndex),
    SkillsRejected(String),
}

pub struct RuntimeWatchers {
    _config_watcher: RecommendedWatcher,
    _skills_watcher: RecommendedWatcher,
    _config_thread: thread::JoinHandle<()>,
    _skills_thread: thread::JoinHandle<()>,
}

pub fn runtime_update_channel() -> (Sender<RuntimeUpdate>, Receiver<RuntimeUpdate>) {
    mpsc::channel()
}

pub fn start_watchers(
    config_file: PathBuf,
    skills_dir: PathBuf,
    config_state: Arc<Mutex<ConfigState>>,
    skills_state: Arc<Mutex<SkillsState>>,
    updates: Sender<RuntimeUpdate>,
) -> notify::Result<RuntimeWatchers> {
    let (config_tx, config_rx) = mpsc::channel();
    let mut config_watcher = notify::recommended_watcher(config_tx)?;
    config_watcher.watch(&config_file, RecursiveMode::NonRecursive)?;

    let (skills_tx, skills_rx) = mpsc::channel();
    let mut skills_watcher = notify::recommended_watcher(skills_tx)?;
    skills_watcher.watch(&skills_dir, RecursiveMode::Recursive)?;

    let config_updates = updates.clone();
    let config_thread = thread::spawn(move || {
        debounce_config_events(config_file, config_rx, config_state, config_updates);
    });

    let skills_thread = thread::spawn(move || {
        debounce_skills_events(skills_dir, skills_rx, skills_state, updates);
    });

    Ok(RuntimeWatchers {
        _config_watcher: config_watcher,
        _skills_watcher: skills_watcher,
        _config_thread: config_thread,
        _skills_thread: skills_thread,
    })
}

fn debounce_config_events(
    config_file: PathBuf,
    events: Receiver<notify::Result<notify::Event>>,
    state: Arc<Mutex<ConfigState>>,
    updates: Sender<RuntimeUpdate>,
) {
    let debounce = Duration::from_millis(250);
    let mut pending = false;
    let mut last_event = Instant::now();
    loop {
        match events.recv_timeout(debounce) {
            Ok(Ok(event)) if is_config_path_event(&config_file, &event) => {
                pending = true;
                last_event = Instant::now();
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                let _ = updates.send(RuntimeUpdate::ConfigRejected(error.to_string()));
            }
            Err(RecvTimeoutError::Timeout) if pending && last_event.elapsed() >= debounce => {
                pending = false;
                let reload = state
                    .lock()
                    .expect("config state lock poisoned")
                    .reload_from(&config_file);
                let update = match reload {
                    ConfigReload::Applied(config) => RuntimeUpdate::ConfigApplied(config),
                    ConfigReload::Rejected(message) => RuntimeUpdate::ConfigRejected(message),
                };
                let _ = updates.send(update);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn debounce_skills_events(
    skills_dir: PathBuf,
    events: Receiver<notify::Result<notify::Event>>,
    state: Arc<Mutex<SkillsState>>,
    updates: Sender<RuntimeUpdate>,
) {
    let debounce = Duration::from_millis(250);
    let mut pending = false;
    let mut last_event = Instant::now();
    loop {
        match events.recv_timeout(debounce) {
            Ok(Ok(event)) if is_skills_path_event(&skills_dir, &event) => {
                pending = true;
                last_event = Instant::now();
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                let _ = updates.send(RuntimeUpdate::SkillsRejected(error.to_string()));
            }
            Err(RecvTimeoutError::Timeout) if pending && last_event.elapsed() >= debounce => {
                pending = false;
                let reload = state
                    .lock()
                    .expect("skills state lock poisoned")
                    .reload_from(&skills_dir);
                let update = match reload {
                    SkillsReload::Applied(index) => RuntimeUpdate::SkillsApplied(index),
                    SkillsReload::Rejected(message) => RuntimeUpdate::SkillsRejected(message),
                };
                let _ = updates.send(update);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
```

- [ ] **Step 6: Run reload and watcher compile checks**

Run:

```bash
cargo fmt
cargo test config
cargo test skills
cargo check
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add src/lib.rs src/config.rs src/skills.rs src/runtime.rs
git commit -m "feat: add config and skills watchers"
```

---

### Task 7: Wire Runtime Bootstrap

**Files:**
- Modify: `src/runtime.rs`
- Modify: `src/app_state.rs`
- Modify: `src/ui/shell.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add runtime context**

Add these imports to the top of `src/runtime.rs`:

```rust
use anyhow::Context;

use crate::app_state::NormaAppState;
use crate::paths::{NormaPaths, default_paths};
use crate::telemetry::{TelemetryGuard, init_tracing, maintain_logs, start_log_maintenance};
```

Then update the existing config and skills imports in `src/runtime.rs` to include bootstrap helpers:

```rust
use crate::config::{
    ConfigReload, ConfigState, NormaConfig, ensure_config, is_config_path_event,
};
use crate::skills::{
    SkillIndex, SkillsReload, SkillsState, is_skills_path_event, scan_skills,
};
```

Then append bootstrap types and functions to `src/runtime.rs`:

```rust
pub struct RuntimeContext {
    pub paths: NormaPaths,
    pub config: Arc<Mutex<ConfigState>>,
    pub skills: Arc<Mutex<SkillsState>>,
    pub telemetry: TelemetryGuard,
    pub watchers: RuntimeWatchers,
    pub updates: Receiver<RuntimeUpdate>,
    pub app_state: NormaAppState,
}

pub fn bootstrap() -> anyhow::Result<RuntimeContext> {
    let paths = default_paths();
    paths.create_all().context("failed to create Norma directories")?;
    let config = ensure_config(&paths).context("failed to load Norma config")?;
    let telemetry = init_tracing(&paths.log_dir, &config.logging)
        .context("failed to initialize Norma telemetry")?;
    maintain_logs(
        &paths.log_dir,
        config.logging.retention_days,
        config.logging.compress_rotated,
    )
    .context("failed to run startup log maintenance")?;
    let _maintenance = start_log_maintenance(paths.log_dir.clone(), config.logging.clone());
    let skill_index = scan_skills(&paths.skills_dir).unwrap_or_else(|error| {
        tracing::warn!(error = %error, "failed to scan skills directory");
        SkillIndex::default()
    });
    let config_state = Arc::new(Mutex::new(ConfigState::new(config.clone())));
    let skills_state = Arc::new(Mutex::new(SkillsState::new(skill_index.clone())));
    let (update_tx, update_rx) = runtime_update_channel();
    let watchers = start_watchers(
        paths.config_file.clone(),
        paths.skills_dir.clone(),
        config_state.clone(),
        skills_state.clone(),
        update_tx,
    )
    .context("failed to start Norma runtime watchers")?;
    tracing::info!(
        config_path = %paths.config_file.display(),
        log_dir = %paths.log_dir.display(),
        data_dir = %paths.data_dir.display(),
        skills_dir = %paths.skills_dir.display(),
        "norma runtime initialized"
    );
    let app_state = NormaAppState::load_current_project_with_runtime(
        paths.clone(),
        config.clone(),
        skill_index.clone(),
    );
    Ok(RuntimeContext {
        paths,
        config: config_state,
        skills: skills_state,
        telemetry,
        watchers,
        updates: update_rx,
        app_state,
    })
}
```

- [ ] **Step 2: Update app state with runtime snapshots and updates**

In `src/app_state.rs`, add imports:

```rust
use crate::config::NormaConfig;
use crate::paths::NormaPaths;
use crate::skills::SkillIndex;
use crate::runtime::RuntimeUpdate;
```

Add fields to `NormaAppState`:

```rust
    pub paths: Option<NormaPaths>,
    pub config: Option<NormaConfig>,
    pub skills: SkillIndex,
```

Update every existing constructor to fill these fields:

```rust
            paths: None,
            config: None,
            skills: SkillIndex::default(),
```

Add this method to `impl NormaAppState`:

```rust
    pub fn load_current_project_with_runtime(
        paths: NormaPaths,
        config: NormaConfig,
        skills: SkillIndex,
    ) -> Self {
        let mut state = Self::load_current_project();
        state.paths = Some(paths);
        state.config = Some(config);
        state.skills = skills;
        state
    }

    pub fn apply_runtime_update(&mut self, update: RuntimeUpdate) {
        match update {
            RuntimeUpdate::ConfigApplied(config) => {
                self.config = Some(config);
            }
            RuntimeUpdate::SkillsApplied(skills) => {
                self.skills = skills;
            }
            RuntimeUpdate::ConfigRejected(message) | RuntimeUpdate::SkillsRejected(message) => {
                tracing::warn!(error = %message, "runtime update rejected");
            }
        }
    }
```

- [ ] **Step 3: Update shell run entrypoint**

In `src/ui/shell.rs`, add an import:

```rust
use std::sync::mpsc::Receiver;

use crate::runtime::RuntimeUpdate;
```

Change `AppShell` to hold runtime updates:

```rust
pub struct AppShell {
    state: NormaAppState,
    updates: Receiver<RuntimeUpdate>,
}
```

Replace the constructor:

```rust
pub fn new(state: NormaAppState, updates: Receiver<RuntimeUpdate>) -> Self {
    Self { state, updates }
}
```

At the top of `render`, drain pending updates before building elements:

```rust
        while let Ok(update) = self.updates.try_recv() {
            self.state.apply_runtime_update(update);
        }
```

Replace:

```rust
pub fn run() {
    Application::new().run(|cx: &mut App| {
        let state = NormaAppState::load_current_project();
```

with:

```rust
pub fn run(state: NormaAppState, updates: Receiver<RuntimeUpdate>) {
    Application::new().run(|cx: &mut App| {
```

Update the window creation closure:

```rust
        cx.open_window(options, |_, cx| cx.new(|_| AppShell::new(state, updates)))
            .expect("failed to open Norma window");
```

- [ ] **Step 4: Update `src/main.rs`**

Replace the file with:

```rust
fn main() -> anyhow::Result<()> {
    let runtime = norma::runtime::bootstrap()?;
    norma::ui::shell::run(runtime.app_state, runtime.updates);
    Ok(())
}
```

- [ ] **Step 5: Run bootstrap checks**

Run:

```bash
cargo fmt
cargo check
cargo test app_state
cargo test runtime
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/runtime.rs src/app_state.rs src/ui/shell.rs src/main.rs
git commit -m "feat: wire runtime bootstrap"
```

---

### Task 8: Verify Runtime Infrastructure End To End

**Files:**
- Modify: `src/workspace.rs`
- Modify: `src/git.rs`
- Modify: `src/agent.rs`
- Modify: `tests/visual_contract.md`

- [ ] **Step 1: Add structured tracing usage in existing modules**

In `src/workspace.rs`, add logs in `open_project` after validation succeeds:

```rust
tracing::info!(
    project = %name,
    root = %root.display(),
    "project opened"
);
```

In `src/git.rs`, add logs in `read_status`:

```rust
tracing::debug!(
    root = %root.as_ref().display(),
    "reading git status"
);
```

In `src/agent.rs`, add a log at the start of `run_mock_task`:

```rust
tracing::info!(
    task,
    runtime = "mock",
    "agent task started"
);
```

- [ ] **Step 2: Update manual visual checklist with runtime notes**

Append to `tests/visual_contract.md`:

```markdown

## Runtime Infrastructure Checks

- [ ] First launch creates `~/.norma/config.toml`.
- [ ] First launch creates `~/.norma/log`.
- [ ] First launch creates `~/.norma/data`.
- [ ] First launch creates `~/.norma/skills`.
- [ ] `~/.norma/log/norma.log` contains one JSON object per line.
- [ ] Editing `~/.norma/config.toml` to a valid logging level keeps the app running.
- [ ] Editing `~/.norma/config.toml` to an invalid logging level keeps the last good config active and records an error.
```

- [ ] **Step 3: Run full verification**

Run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all commands pass. If Clippy is not installed, record the exact message and run:

```bash
rustup component add clippy
cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 4: Manual runtime smoke test**

Run:

```bash
rm -rf ~/.norma
cargo run
```

Expected after the app starts:

```bash
test -f ~/.norma/config.toml
test -d ~/.norma/log
test -d ~/.norma/data
test -d ~/.norma/skills
head -n 1 ~/.norma/log/norma.log | jq .
```

Expected: all `test` commands exit 0 and `jq` parses the first log line.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/workspace.rs src/git.rs src/agent.rs tests/visual_contract.md
git commit -m "test: verify runtime infrastructure"
```

---

## Final Verification

Run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all pass, or Clippy installation failure is documented with the exact reason.

Check repository status:

```bash
git status --short
```

Expected: only pre-existing unrelated files may remain, such as `docs/superpowers/specs/2026-06-11-norma-ai-provider-settings-design.md` if it was not part of this implementation.

Do not stage or commit unrelated files.
