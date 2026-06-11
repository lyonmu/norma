use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::{ConfigReload, ConfigState, is_config_path_event};
use crate::runtime::RuntimeUpdate;
use crate::skills::{SkillsReload, SkillsState, is_skills_path_event};

pub struct RuntimeWatchers {
    _config_watcher: RecommendedWatcher,
    _skills_watcher: RecommendedWatcher,
    _config_thread: thread::JoinHandle<()>,
    _skills_thread: thread::JoinHandle<()>,
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

    tracing::info!(
        component = "runtime",
        config_path = %config_file.display(),
        skills_dir = %skills_dir.display(),
        "runtime watchers started"
    );

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
                tracing::debug!(component = "runtime", path = %config_file.display(), "config watcher event received");
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
                tracing::debug!(component = "runtime", path = %skills_dir.display(), "skills watcher event received");
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
