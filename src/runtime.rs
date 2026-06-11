use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::app_state::NormaAppState;
use crate::config::ensure_config;
use crate::config::{ConfigReload, ConfigState, NormaConfig, is_config_path_event};
use crate::paths::{NormaPaths, default_paths};
use crate::skills::{SkillIndex, SkillsReload, SkillsState, is_skills_path_event, scan_skills};
use crate::telemetry::{TelemetryGuard, init_tracing, maintain_logs, start_log_maintenance};

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
        debounce_config_events(config_file, config_rx, config_state, config_updates)
    });
    let skills_thread =
        thread::spawn(move || debounce_skills_events(skills_dir, skills_rx, skills_state, updates));

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
    paths
        .create_all()
        .context("failed to create Norma directories")?;
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
