# Norma Module And Logging Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize Norma into scalable domain module directories and add structured tracing at runtime boundaries.

**Architecture:** First perform a mechanical source-layout migration so behavior stays stable and reviewable. Then add focused agent/provider/tool abstractions and structured logs around startup, config, logging, skills, workspace, Git, app state, watcher, and mock agent flows. Internal API compatibility is not required; update imports directly to the new module paths.

**Tech Stack:** Rust 2024, Cargo, GPUI `0.2.2`, `tracing`, `tracing-subscriber`, `tracing-appender`, `notify`, `config`, `serde`, `toml`, `thiserror`, `anyhow`, `tempfile`.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-06-11-norma-module-logging-restructure-design.md`.

It does not implement real LLM provider calls, real provider connection tests, executable tools, executable Skills, MCP runtime integration, ACP, destructive Git commands, UI redesign, or session persistence.

## Target File Structure

Create or modify these files:

- Modify: `src/lib.rs`  
  Export the new domain directories and remove the old `telemetry` export.

- Modify: `src/main.rs`  
  Keep bootstrap-only logic.

- Create directory: `src/logging/`  
  Files: `mod.rs`, `error.rs`, `init.rs`, `maintenance.rs`, `writer.rs`.

- Create directory: `src/config/`  
  Files: `mod.rs`, `app.rs`, `file.rs`, `model.rs`, `reload.rs`.

- Create directory: `src/paths/`  
  Files: `mod.rs`, `local.rs`.

- Create directory: `src/skills/`  
  Files: `mod.rs`, `index.rs`, `reload.rs`.

- Create directory: `src/workspace/`  
  Files: `mod.rs`, `file_tree.rs`, `project.rs`.

- Create directory: `src/git/`  
  Files: `mod.rs`, `status.rs`.

- Create directory: `src/session/`  
  Files: `mod.rs`, `event.rs`, `inspector.rs`, `state.rs`, `thread.rs`.

- Create directory: `src/agent/`  
  Files: `mod.rs`, `event.rs`, `mock.rs`, `runtime.rs`.

- Create directory: `src/agent/input/`  
  Files: `mod.rs`, `message.rs`, `request.rs`, `schema.rs`.

- Create directory: `src/agent/provider/`  
  Files: `mod.rs`, `capability.rs`, `model.rs`, `registry.rs`.

- Create directory: `src/agent/tools/`  
  Files: `mod.rs`, `definition.rs`, `invocation.rs`, `result.rs`.

- Create directory: `src/app/`  
  Files: `mod.rs`, `state.rs`.

- Create directory: `src/runtime/`  
  Files: `mod.rs`, `bootstrap.rs`, `update.rs`, `watchers.rs`.

- Modify: `src/ui/settings.rs`  
  Update imports from old module paths to new paths only.

- Modify: `src/ui/shell.rs`  
  Update imports from old module paths to new paths only.

- Modify: `src/ui/sidebar.rs`  
  Update imports from old module paths to new paths only.

- Modify: `src/ui/execution.rs`  
  Update imports from old module paths to new paths only.

- Modify: `src/ui/inspector.rs`  
  Update imports from old module paths to new paths only.

- Delete after migration: `src/telemetry.rs`, `src/config.rs`, `src/paths.rs`, `src/skills.rs`, `src/workspace.rs`, `src/git.rs`, `src/session.rs`, `src/agent.rs`, `src/app_state.rs`, `src/runtime.rs`.

## Commit Strategy

Commit after each major task:

- `refactor(logging): split telemetry into logging domain`
- `refactor(config): split settings and runtime config modules`
- `refactor(agent): add provider input and tool domains`
- `refactor(core): split workspace git skills and session modules`
- `refactor(runtime): split bootstrap and watcher modules`
- `feat(logging): add structured runtime diagnostics`

---

### Task 1: Capture Baseline And Prepare The Migration

**Files:**
- Read: `docs/superpowers/specs/2026-06-11-norma-module-logging-restructure-design.md`
- Read: `AGENTS.md`
- Verify: current repository state

- [ ] **Step 1: Confirm a clean starting state**

Run:

```bash
git status --short
```

Expected: no output, or only intentional plan/spec files already committed. If there are unrelated user changes, stop and inspect before editing.

- [ ] **Step 2: Run baseline verification**

Run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all commands exit 0. It is acceptable if Cargo prints the already-observed `block v0.1.6` future-incompatibility warning.

- [ ] **Step 3: Confirm current debug-output state**

Run:

```bash
rg -n "println!|eprintln!|dbg!" src tests
```

Expected: no matches. If there are matches, record them and remove them in the task that owns the affected module.

---

### Task 2: Split `telemetry.rs` Into The `logging` Domain

**Files:**
- Create: `src/logging/mod.rs`
- Create: `src/logging/error.rs`
- Create: `src/logging/init.rs`
- Create: `src/logging/maintenance.rs`
- Create: `src/logging/writer.rs`
- Modify: `src/lib.rs`
- Modify: `src/runtime.rs`
- Delete: `src/telemetry.rs`

- [ ] **Step 1: Create `src/logging/error.rs`**

Add:

```rust
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
```

- [ ] **Step 2: Create `src/logging/writer.rs`**

Move the rotating writer logic from `src/telemetry.rs` and make the writer visible within the logging domain:

```rust
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
```

- [ ] **Step 3: Create `src/logging/maintenance.rs`**

Move maintenance logic and add structured diagnostics:

```rust
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
            tracing::debug!(
                component = "logging",
                path = %path.display(),
                "expired log removed"
            );
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
    tracing::debug!(
        component = "logging",
        path = %path.display(),
        compressed_path = %compressed.display(),
        "log file compressed"
    );
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
```

- [ ] **Step 4: Create `src/logging/init.rs`**

Move tracing initialization:

```rust
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
    tracing::info!(
        component = "logging",
        log_dir = %log_dir.display(),
        level = %logging.level,
        format = %logging.format,
        "tracing initialized"
    );
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
```

- [ ] **Step 5: Create `src/logging/mod.rs`**

Add:

```rust
mod error;
mod init;
mod maintenance;
mod writer;

pub use error::LoggingError;
pub use init::{LoggingGuard, init_tracing};
pub use maintenance::{maintain_logs, start_log_maintenance};

pub(crate) use writer::RotatingLogWriter;
```

- [ ] **Step 6: Update exports and runtime imports**

Change `src/lib.rs` from:

```rust
pub mod telemetry;
```

to:

```rust
pub mod logging;
```

In the current `src/runtime.rs`, replace:

```rust
use crate::telemetry::{TelemetryGuard, init_tracing, maintain_logs, start_log_maintenance};
```

with:

```rust
use crate::logging::{LoggingGuard, init_tracing, maintain_logs, start_log_maintenance};
```

Then replace the `RuntimeContext` field:

```rust
pub telemetry: TelemetryGuard,
```

with:

```rust
pub logging: LoggingGuard,
```

and in `bootstrap()` replace `telemetry` variable/field names with `logging`.

- [ ] **Step 7: Delete the old file and verify**

Run:

```bash
rm src/telemetry.rs
cargo fmt
cargo test logging -- --nocapture
cargo check
```

Expected: logging tests pass, and `cargo check` exits 0.

- [ ] **Step 8: Commit**

Run:

```bash
git add src/lib.rs src/runtime.rs src/logging src/telemetry.rs
git commit -m "refactor(logging): split telemetry into logging domain"
```

---

### Task 3: Split Runtime Config Into `config/`

**Files:**
- Create: `src/config/mod.rs`
- Create: `src/config/app.rs`
- Create: `src/config/model.rs`
- Create: `src/config/file.rs`
- Create: `src/config/reload.rs`
- Delete: `src/config.rs`

- [ ] **Step 1: Create `src/config/app.rs`**

Move these complete definitions from `src/config.rs` into `src/config/app.rs`: `SettingsSection`, `ProviderProtocol`, `ProviderConfigStatus`, `AiProviderConfig`, `AppConfig`, and `mask_secret`. Also move the current tests named `settings_sections_match_design_order`, `provider_protocols_are_limited_to_openai_and_anthropic`, `masks_api_key_by_default`, `masks_short_secrets_completely`, `validates_required_provider_fields_without_network_calls`, and `sample_config_selects_openai_provider` into this file.

The resulting file must define all copied public items with their current public variants, fields, methods, and tests. Do not create compatibility aliases.

- [ ] **Step 2: Create `src/config/model.rs`**

Move `NormaConfig`, `WindowConfig`, `PathsConfig`, `LoggingConfig`, `ConfigError`, `NormaConfig::default_for`, and `NormaConfig::validate` into `src/config/model.rs`.

Update path references inside `default_for` from:

```rust
pub fn default_for(paths: &crate::paths::NormaPaths) -> Self
```

to keep using:

```rust
pub fn default_for(paths: &crate::paths::NormaPaths) -> Self
```

Do not move file IO functions into this file.

- [ ] **Step 3: Create `src/config/file.rs`**

Move config persistence functions into `src/config/file.rs` and add structured logs:

```rust
use std::fs;
use std::path::Path;

use crate::config::{ConfigError, NormaConfig};

pub fn ensure_config(paths: &crate::paths::NormaPaths) -> Result<NormaConfig, ConfigError> {
    if !paths.config_file.exists() {
        let config = NormaConfig::default_for(paths);
        write_config(&paths.config_file, &config)?;
        tracing::info!(
            component = "config",
            path = %paths.config_file.display(),
            "default config written"
        );
    }
    load_config_with_env(&paths.config_file)
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
    tracing::info!(
        component = "config",
        path = %path.display(),
        "config loaded"
    );
    Ok(config)
}

pub fn write_config(path: impl AsRef<Path>, config: &NormaConfig) -> Result<(), ConfigError> {
    let path = path.as_ref();
    let content = toml::to_string_pretty(config).map_err(ConfigError::Serialize)?;
    fs::write(path, content).map_err(|source| ConfigError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    tracing::debug!(
        component = "config",
        path = %path.display(),
        "config written"
    );
    Ok(())
}

pub fn load_config_with_env(path: impl AsRef<Path>) -> Result<NormaConfig, ConfigError> {
    let path = path.as_ref();
    tracing::debug!(
        component = "config",
        path = %path.display(),
        "loading config with environment overrides"
    );
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
    tracing::info!(
        component = "config",
        path = %path.display(),
        "config loaded"
    );
    Ok(config)
}
```

- [ ] **Step 4: Create `src/config/reload.rs`**

Move reload state into `src/config/reload.rs` and add reload logs:

```rust
use std::path::Path;

use notify;

use crate::config::{NormaConfig, load_config_with_env};

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
        let path = path.as_ref();
        match load_config_with_env(path) {
            Ok(config) => {
                self.active = config.clone();
                self.last_error = None;
                tracing::info!(
                    component = "config",
                    path = %path.display(),
                    "config reload applied"
                );
                ConfigReload::Applied(config)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
                tracing::warn!(
                    component = "config",
                    path = %path.display(),
                    error = %error,
                    "config reload rejected"
                );
                ConfigReload::Rejected(message)
            }
        }
    }
}

pub fn is_config_path_event(config_file: &Path, event: &notify::Event) -> bool {
    event.paths.iter().any(|path| path == config_file)
}
```

- [ ] **Step 5: Create `src/config/mod.rs`**

Add:

```rust
mod app;
mod file;
mod model;
mod reload;

pub use app::{
    AiProviderConfig, AppConfig, ProviderConfigStatus, ProviderProtocol, SettingsSection,
    mask_secret,
};
pub use file::{ensure_config, load_config, load_config_with_env, write_config};
pub use model::{ConfigError, LoggingConfig, NormaConfig, PathsConfig, WindowConfig};
pub use reload::{ConfigReload, ConfigState, is_config_path_event};
```

- [ ] **Step 6: Delete old file and verify**

Run:

```bash
rm src/config.rs
cargo fmt
cargo test config -- --nocapture
cargo check
```

Expected: config tests pass and `cargo check` exits 0.

- [ ] **Step 7: Commit**

Run:

```bash
git add src/config src/config.rs
git commit -m "refactor(config): split settings and runtime config modules"
```

---

### Task 4: Split Paths, Skills, Workspace, Git, And Session Domains

**Files:**
- Create: `src/paths/mod.rs`
- Create: `src/paths/local.rs`
- Create: `src/skills/mod.rs`
- Create: `src/skills/index.rs`
- Create: `src/skills/reload.rs`
- Create: `src/workspace/mod.rs`
- Create: `src/workspace/project.rs`
- Create: `src/workspace/file_tree.rs`
- Create: `src/git/mod.rs`
- Create: `src/git/status.rs`
- Create: `src/session/mod.rs`
- Create: `src/session/event.rs`
- Create: `src/session/inspector.rs`
- Create: `src/session/state.rs`
- Create: `src/session/thread.rs`
- Delete: old single files for these domains

- [ ] **Step 1: Split `paths`**

Move all code from `src/paths.rs` to `src/paths/local.rs`. Create `src/paths/mod.rs`:

```rust
mod local;

pub use local::{NormaPaths, PathsError, default_paths};
```

Delete `src/paths.rs`.

- [ ] **Step 2: Split `skills`**

Move `SkillsError`, `SkillEntry`, `SkillIndex`, and `scan_skills` to `src/skills/index.rs`. Add structured logs to `scan_skills`:

```rust
tracing::debug!(component = "skills", root = %root.display(), "skill scan started");
```

before reading the directory, and:

```rust
tracing::info!(
    component = "skills",
    root = %root.display(),
    entry_count = entries.len(),
    "skill scan completed"
);
```

before returning.

Move `SkillsReload`, `SkillsState`, and `is_skills_path_event` to `src/skills/reload.rs`. Add logs in `reload_from`:

```rust
tracing::info!(component = "skills", "skills reload applied");
tracing::warn!(component = "skills", error = %error, "skills reload rejected");
```

Create `src/skills/mod.rs`:

```rust
mod index;
mod reload;

pub use index::{SkillEntry, SkillIndex, SkillsError, scan_skills};
pub use reload::{SkillsReload, SkillsState, is_skills_path_event};
```

Delete `src/skills.rs`.

- [ ] **Step 3: Split `workspace`**

Move `WorkspaceError` and `Project` plus `open_project` to `src/workspace/project.rs`. Add warning logs before returning `MissingPath` and `NotDirectory`.

Move `FileKind`, `FileNode`, `sample_file_tree`, and `load_file_tree` to `src/workspace/file_tree.rs`. Add:

```rust
tracing::debug!(
    component = "workspace",
    root = %root.display(),
    max_entries,
    "file tree load started"
);
tracing::debug!(
    component = "workspace",
    root = %root.display(),
    file_count = nodes.len(),
    "file tree load completed"
);
```

Create `src/workspace/mod.rs`:

```rust
mod file_tree;
mod project;

pub use file_tree::{FileKind, FileNode, load_file_tree, sample_file_tree};
pub use project::{Project, WorkspaceError, open_project};
```

Delete `src/workspace.rs`.

- [ ] **Step 4: Split `git`**

Move all code from `src/git.rs` to `src/git/status.rs`. Add logs in `read_status`:

```rust
tracing::debug!(component = "git", root = %root.as_ref().display(), "git status command started");
tracing::warn!(component = "git", root = %root.as_ref().display(), error = %error, "git status failed to start");
tracing::warn!(component = "git", root = %root.as_ref().display(), stderr = %stderr, "git status returned non-zero");
```

After parsing, log summary counts:

```rust
tracing::debug!(
    component = "git",
    changed_files = summary.files.len(),
    modified = summary.modified,
    added = summary.added,
    deleted = summary.deleted,
    untracked = summary.untracked,
    "git status parsed"
);
```

Create `src/git/mod.rs`:

```rust
mod status;

pub use status::{ChangeKind, ChangedFile, GitError, GitStatusSummary, parse_status, read_status};
```

Delete `src/git.rs`.

- [ ] **Step 5: Split `session`**

Move:

- `InspectorTab`, `DiffHunkSummary`, `FileChangePreview` to `src/session/inspector.rs`.
- `SessionThread` and `sample_thread` to `src/session/thread.rs`.
- `StepStatus`, `ExecutionStep`, `ChecklistItem`, `SessionEvent` to `src/session/event.rs`.
- `SessionState` and its tests to `src/session/state.rs`.

Create `src/session/mod.rs`:

```rust
mod event;
mod inspector;
mod state;
mod thread;

pub use event::{ChecklistItem, ExecutionStep, SessionEvent, StepStatus};
pub use inspector::{DiffHunkSummary, FileChangePreview, InspectorTab};
pub use state::SessionState;
pub use thread::{SessionThread, sample_thread};
```

Delete `src/session.rs`.

- [ ] **Step 6: Verify and commit**

Run:

```bash
cargo fmt
cargo test paths skills workspace git session -- --nocapture
cargo check
```

Expected: tests pass and `cargo check` exits 0.

Commit:

```bash
git add src/paths src/skills src/workspace src/git src/session src/paths.rs src/skills.rs src/workspace.rs src/git.rs src/session.rs
git commit -m "refactor(core): split workspace git skills and session modules"
```

---

### Task 5: Build The Agent Domain With Provider, Input, And Tool Submodules

**Files:**
- Create: `src/agent/mod.rs`
- Create: `src/agent/runtime.rs`
- Create: `src/agent/mock.rs`
- Create: `src/agent/event.rs`
- Create: `src/agent/input/mod.rs`
- Create: `src/agent/input/message.rs`
- Create: `src/agent/input/request.rs`
- Create: `src/agent/input/schema.rs`
- Create: `src/agent/provider/mod.rs`
- Create: `src/agent/provider/capability.rs`
- Create: `src/agent/provider/model.rs`
- Create: `src/agent/provider/registry.rs`
- Create: `src/agent/tools/mod.rs`
- Create: `src/agent/tools/definition.rs`
- Create: `src/agent/tools/invocation.rs`
- Create: `src/agent/tools/result.rs`
- Delete: `src/agent.rs`

- [ ] **Step 1: Create provider model abstractions**

Create `src/agent/provider/model.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    OpenAi,
    Anthropic,
    OpenAiCompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModel {
    pub id: ProviderId,
    pub name: String,
    pub protocol: ProviderProtocol,
    pub default_model: String,
}
```

Create `src/agent/provider/capability.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub tool_use: bool,
    pub structured_output: bool,
}
```

Create `src/agent/provider/registry.rs`:

```rust
use crate::agent::provider::{ProviderId, ProviderModel};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderRegistry {
    providers: Vec<ProviderModel>,
}

impl ProviderRegistry {
    pub fn new(providers: Vec<ProviderModel>) -> Self {
        Self { providers }
    }

    pub fn providers(&self) -> &[ProviderModel] {
        &self.providers
    }

    pub fn find(&self, id: &ProviderId) -> Option<&ProviderModel> {
        self.providers.iter().find(|provider| provider.id == *id)
    }
}
```

Create `src/agent/provider/mod.rs`:

```rust
mod capability;
mod model;
mod registry;

pub use capability::ProviderCapabilities;
pub use model::{ProviderId, ProviderModel, ProviderProtocol};
pub use registry::ProviderRegistry;
```

- [ ] **Step 2: Create structured input abstractions**

Create `src/agent/input/message.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMessageRole {
    User,
    System,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub role: AgentMessageRole,
    pub content: String,
}
```

Create `src/agent/input/schema.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredInputSchema {
    pub name: String,
    pub description: String,
}
```

Create `src/agent/input/request.rs`:

```rust
use std::path::PathBuf;

use crate::agent::input::{AgentMessage, StructuredInputSchema};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    pub task: String,
    pub messages: Vec<AgentMessage>,
    pub project_root: Option<PathBuf>,
    pub schema: Option<StructuredInputSchema>,
}

impl AgentRequest {
    pub fn from_task(task: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            messages: Vec::new(),
            project_root: None,
            schema: None,
        }
    }
}
```

Create `src/agent/input/mod.rs`:

```rust
mod message;
mod request;
mod schema;

pub use message::{AgentMessage, AgentMessageRole};
pub use request::AgentRequest;
pub use schema::StructuredInputSchema;
```

- [ ] **Step 3: Create tool use abstractions**

Create `src/agent/tools/definition.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
}
```

Create `src/agent/tools/invocation.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub input_json: String,
}
```

Create `src/agent/tools/result.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolInvocationResult {
    Succeeded { output_json: String },
    Failed { message: String },
}
```

Create `src/agent/tools/mod.rs`:

```rust
mod definition;
mod invocation;
mod result;

pub use definition::ToolDefinition;
pub use invocation::ToolInvocation;
pub use result::ToolInvocationResult;
```

- [ ] **Step 4: Split runtime and mock implementation**

Create `src/agent/runtime.rs`:

```rust
use crate::agent::input::AgentRequest;
use crate::session::SessionEvent;

pub trait AgentRuntime {
    fn run_mock_task(&self, task: &str) -> Vec<SessionEvent> {
        self.run(AgentRequest::from_task(task))
    }

    fn run(&self, request: AgentRequest) -> Vec<SessionEvent>;
}
```

Create `src/agent/event.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    Started { task: String },
    Completed { event_count: usize },
}
```

Move the current mock implementation from `src/agent.rs` to `src/agent/mock.rs`, change the trait implementation to:

```rust
impl AgentRuntime for MockAgentRuntime {
    fn run(&self, request: AgentRequest) -> Vec<SessionEvent> {
        tracing::info!(
            component = "agent",
            runtime = "mock",
            task = %request.task,
            "agent task started"
        );
        let events = vec![
            SessionEvent::UserTask {
                content: request.task.clone(),
            },
            SessionEvent::AgentPlan {
                goal: "完善 Norma 的整体设计、架构、模块、数据流、UI、Git 交互".to_string(),
                constraints: vec![
                    "不内置代码编辑器".to_string(),
                    "专注项目上下文、执行流、变更审查与回滚".to_string(),
                    "V1 不执行破坏性 Git 操作".to_string(),
                ],
            },
            SessionEvent::StepUpdated(completed_step(
                "读取 README",
                "读取并解析 README.md 与 README.zh.md，提取产品定位与目标。",
                "18.4s",
            )),
            SessionEvent::StepUpdated(completed_step(
                "确认产品边界",
                "确认 Norma 不包含代码编辑器，聚焦执行流与变更审查。",
                "15.7s",
            )),
            SessionEvent::StepUpdated(running_step()),
            SessionEvent::ChangeSummary {
                files: vec![
                    changed("src/ui/inspector.rs", 86, 10, 4),
                    changed("src/ui/execution_item.rs", 42, 3, 2),
                    changed("src/agent/runner.rs", 38, 2, 2),
                    changed("src/git/repository.rs", 29, 5, 2),
                    changed("src/config/settings.rs", 24, 0, 1),
                ],
            },
            SessionEvent::StepUpdated(waiting_step()),
        ];
        tracing::info!(
            component = "agent",
            runtime = "mock",
            task = %request.task,
            event_count = events.len(),
            "agent task completed"
        );
        events
    }
}
```

Move the helper functions `completed_step`, `running_step`, `waiting_step`, and `changed` into `mock.rs`. Move the current `mock_runtime_emits_visual_contract_states` test into `mock.rs` and update it to call `runtime.run_mock_task("完善 Norma 项目设计")`.

- [ ] **Step 5: Create `src/agent/mod.rs`**

Add:

```rust
pub mod input;
pub mod provider;
pub mod tools;

mod event;
mod mock;
mod runtime;

pub use event::AgentEvent;
pub use mock::MockAgentRuntime;
pub use runtime::AgentRuntime;
```

- [ ] **Step 6: Verify and commit**

Run:

```bash
rm src/agent.rs
cargo fmt
cargo test agent -- --nocapture
cargo check
```

Expected: agent tests pass and `cargo check` exits 0.

Commit:

```bash
git add src/agent src/agent.rs
git commit -m "refactor(agent): add provider input and tool domains"
```

---

### Task 6: Move App State Into `app/`

**Files:**
- Create: `src/app/mod.rs`
- Create: `src/app/state.rs`
- Modify: `src/lib.rs`
- Modify: `src/runtime/bootstrap.rs` or current `src/runtime.rs`
- Modify: `src/ui/shell.rs`
- Delete: `src/app_state.rs`

- [ ] **Step 1: Move app state code**

Move all code from `src/app_state.rs` to `src/app/state.rs`.

Update imports inside `state.rs`:

```rust
use crate::agent::{AgentRuntime, MockAgentRuntime};
use crate::config::{AppConfig, NormaConfig};
use crate::git::{GitStatusSummary, read_status};
use crate::paths::NormaPaths;
use crate::runtime::RuntimeUpdate;
use crate::session::{SessionState, sample_thread};
use crate::skills::SkillIndex;
use crate::workspace::{FileNode, Project, load_file_tree, open_project, sample_file_tree};
```

Add logs in `load_project`:

```rust
tracing::info!(component = "app", root = %root.display(), "loading project state");
```

Add warn log when falling back to sample file tree:

```rust
let files = load_file_tree(&project.root, 80).unwrap_or_else(|error| {
    tracing::warn!(
        component = "app",
        root = %project.root.display(),
        error = %error,
        "file tree fallback used"
    );
    sample_file_tree()
});
```

Add logs in `apply_runtime_update` for applied updates:

```rust
tracing::info!(component = "app", "runtime config update applied");
tracing::info!(
    component = "app",
    skill_count = skills.entries.len(),
    "runtime skills update applied"
);
```

- [ ] **Step 2: Create `src/app/mod.rs`**

Add:

```rust
mod state;

pub use state::{NormaAppState, ProjectSelectionState};
```

- [ ] **Step 3: Update exports**

In `src/lib.rs`, replace:

```rust
pub mod app_state;
```

with:

```rust
pub mod app;
```

Update all imports:

```rust
crate::app_state::NormaAppState
```

to:

```rust
crate::app::NormaAppState
```

- [ ] **Step 4: Verify and commit**

Run:

```bash
rm src/app_state.rs
cargo fmt
cargo test app -- --nocapture
cargo check
```

Expected: app state tests pass and `cargo check` exits 0.

Commit:

```bash
git add src/app src/app_state.rs src/lib.rs src/runtime.rs src/ui
git commit -m "refactor(app): move aggregate state into app domain"
```

---

### Task 7: Split Runtime Into Bootstrap, Updates, And Watchers

**Files:**
- Create: `src/runtime/mod.rs`
- Create: `src/runtime/bootstrap.rs`
- Create: `src/runtime/update.rs`
- Create: `src/runtime/watchers.rs`
- Modify: `src/main.rs`
- Delete: `src/runtime.rs`

- [ ] **Step 1: Create `src/runtime/update.rs`**

Move `RuntimeUpdate` and `runtime_update_channel` into `src/runtime/update.rs`:

```rust
use std::sync::mpsc::{self, Receiver, Sender};

use crate::config::NormaConfig;
use crate::skills::SkillIndex;

#[derive(Debug, Clone)]
pub enum RuntimeUpdate {
    ConfigApplied(NormaConfig),
    ConfigRejected(String),
    SkillsApplied(SkillIndex),
    SkillsRejected(String),
}

pub fn runtime_update_channel() -> (Sender<RuntimeUpdate>, Receiver<RuntimeUpdate>) {
    mpsc::channel()
}
```

- [ ] **Step 2: Create `src/runtime/watchers.rs`**

Move `RuntimeWatchers`, `start_watchers`, `debounce_config_events`, and `debounce_skills_events` into this file. Update imports to:

```rust
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::{ConfigReload, ConfigState, is_config_path_event};
use crate::runtime::RuntimeUpdate;
use crate::skills::{SkillsReload, SkillsState, is_skills_path_event};
```

Add logs:

```rust
tracing::info!(
    component = "runtime",
    config_path = %config_file.display(),
    skills_dir = %skills_dir.display(),
    "runtime watchers started"
);
```

inside `start_watchers`, and:

```rust
tracing::debug!(component = "runtime", path = %config_file.display(), "config watcher event received");
tracing::debug!(component = "runtime", path = %skills_dir.display(), "skills watcher event received");
```

inside the debounce loops when relevant events arrive.

- [ ] **Step 3: Create `src/runtime/bootstrap.rs`**

Move `RuntimeContext` and `bootstrap` into this file. Update imports:

```rust
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;

use anyhow::Context;

use crate::app::NormaAppState;
use crate::config::{ConfigState, ensure_config};
use crate::logging::{LoggingGuard, init_tracing, maintain_logs, start_log_maintenance};
use crate::paths::{NormaPaths, default_paths};
use crate::runtime::{RuntimeUpdate, RuntimeWatchers, runtime_update_channel, start_watchers};
use crate::skills::{SkillIndex, SkillsState, scan_skills};
```

Add startup logs after logging is initialized:

```rust
tracing::info!(component = "runtime", "runtime bootstrap started");
```

Keep the final runtime initialized log shown in Step 2, and ensure the `RuntimeContext` field is:

```rust
pub logging: LoggingGuard,
```

- [ ] **Step 4: Create `src/runtime/mod.rs`**

Add:

```rust
mod bootstrap;
mod update;
mod watchers;

pub use bootstrap::{RuntimeContext, bootstrap};
pub use update::{RuntimeUpdate, runtime_update_channel};
pub use watchers::{RuntimeWatchers, start_watchers};
```

- [ ] **Step 5: Delete old runtime file and verify**

Run:

```bash
rm src/runtime.rs
cargo fmt
cargo test runtime -- --nocapture
cargo check
```

Expected: runtime-related compilation succeeds and full `cargo check` exits 0.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/runtime src/runtime.rs src/main.rs src/lib.rs
git commit -m "refactor(runtime): split bootstrap and watcher modules"
```

---

### Task 8: Update UI Imports And Preserve Visual Behavior

**Files:**
- Modify: `src/ui/settings.rs`
- Modify: `src/ui/shell.rs`
- Modify: `src/ui/sidebar.rs`
- Modify: `src/ui/execution.rs`
- Modify: `src/ui/inspector.rs`
- Modify: `src/ui/components.rs`
- Modify: `src/ui/theme.rs`

- [ ] **Step 1: Search for stale imports**

Run:

```bash
rg -n "crate::(app_state|telemetry)|crate::config::|crate::agent::|crate::session::|crate::git::|crate::workspace::" src/ui src/main.rs src/lib.rs
```

Expected: matches are normal for domain imports except `crate::app_state` and `crate::telemetry`, which must be changed.

- [ ] **Step 2: Update stale imports**

Replace:

```rust
use crate::app_state::NormaAppState;
```

with:

```rust
use crate::app::NormaAppState;
```

Remove any `crate::telemetry` import. Keep `crate::config`, `crate::session`, `crate::git`, and `crate::workspace` imports if their public re-exports preserve the current type names.

- [ ] **Step 3: Verify UI compilation**

Run:

```bash
cargo check
```

Expected: the GPUI UI compiles with the new imports.

- [ ] **Step 4: Commit if UI imports changed separately**

Run:

```bash
git add src/ui src/main.rs src/lib.rs
git commit -m "refactor(ui): update imports for domain modules"
```

If Task 6 or Task 7 already included all UI import changes, skip this commit.

---

### Task 9: Add Final Structured Logging Coverage

**Files:**
- Modify: `src/runtime/bootstrap.rs`
- Modify: `src/runtime/watchers.rs`
- Modify: `src/config/file.rs`
- Modify: `src/config/reload.rs`
- Modify: `src/logging/init.rs`
- Modify: `src/logging/maintenance.rs`
- Modify: `src/logging/writer.rs`
- Modify: `src/skills/index.rs`
- Modify: `src/skills/reload.rs`
- Modify: `src/workspace/project.rs`
- Modify: `src/workspace/file_tree.rs`
- Modify: `src/git/status.rs`
- Modify: `src/app/state.rs`
- Modify: `src/agent/mock.rs`

- [ ] **Step 1: Audit current logs**

Run:

```bash
rg -n "tracing::(trace|debug|info|warn|error)!" src
```

Expected: logs exist in runtime, config, logging, skills, workspace, git, app, and agent domains.

- [ ] **Step 2: Add missing runtime boundary logs**

Ensure `runtime/bootstrap.rs` logs:

```rust
tracing::info!(component = "runtime", "runtime bootstrap started");
tracing::info!(
    component = "runtime",
    config_path = %paths.config_file.display(),
    log_dir = %paths.log_dir.display(),
    data_dir = %paths.data_dir.display(),
    skills_dir = %paths.skills_dir.display(),
    "norma runtime initialized"
);
```

Ensure watcher send failures use `warn!`:

```rust
if updates.send(update).is_err() {
    tracing::warn!(component = "runtime", "runtime update receiver dropped");
}
```

- [ ] **Step 3: Add missing fallback logs**

Ensure project-open and file-tree fallbacks emit `warn!` with `component`, `path` or `root`, and `error` fields. For example:

```rust
tracing::warn!(
    component = "workspace",
    root = %root.display(),
    error = %error,
    "project open failed"
);
```

- [ ] **Step 4: Verify no secrets are logged**

Run:

```bash
rg -n "api_key|api key|secret|token|credential|base_url" src
```

Expected: matches may exist in config/UI data models and validation, but no `tracing::` call should log secret values, API keys, raw tokens, or provider base URLs.

- [ ] **Step 5: Verify no debug-output macros exist**

Run:

```bash
rg -n "println!|eprintln!|dbg!" src tests
```

Expected: no matches.

- [ ] **Step 6: Run full verification and commit**

Run:

```bash
cargo fmt
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all commands exit 0. Existing future-incompat warning from dependencies is acceptable.

Commit:

```bash
git add src
git commit -m "feat(logging): add structured runtime diagnostics"
```

---

### Task 10: Final Repository Verification

**Files:**
- Verify: all modified source files
- Verify: `docs/superpowers/specs/2026-06-11-norma-module-logging-restructure-design.md`

- [ ] **Step 1: Confirm target source layout**

Run:

```bash
find src -maxdepth 3 -type f | sort
```

Expected: output includes domain directories for `app`, `agent`, `config`, `git`, `logging`, `paths`, `runtime`, `session`, `skills`, `ui`, and `workspace`.

- [ ] **Step 2: Confirm old single-file modules are gone**

Run:

```bash
test ! -f src/telemetry.rs
test ! -f src/config.rs
test ! -f src/paths.rs
test ! -f src/skills.rs
test ! -f src/workspace.rs
test ! -f src/git.rs
test ! -f src/session.rs
test ! -f src/agent.rs
test ! -f src/app_state.rs
test ! -f src/runtime.rs
```

Expected: all `test ! -f` commands exit 0.

- [ ] **Step 3: Confirm no stale telemetry code import remains**

Run:

```bash
rg -n "crate::telemetry|pub mod telemetry|telemetry::" src
```

Expected: no matches.

- [ ] **Step 4: Confirm no disallowed debug output remains**

Run:

```bash
rg -n "println!|eprintln!|dbg!" src tests
```

Expected: no matches.

- [ ] **Step 5: Run final verification**

Run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all commands exit 0.

- [ ] **Step 6: Review diff**

Run:

```bash
git diff --stat HEAD
git status --short
```

Expected: only source files changed by the implementation remain unstaged or staged, depending on the execution workflow. No generated `target/` files should appear.
