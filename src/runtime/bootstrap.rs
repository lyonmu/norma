use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use anyhow::Context;

use crate::app::NormaAppState;
use crate::config::{ConfigState, ensure_config};
use crate::logging::{LoggingGuard, init_tracing, maintain_logs, start_log_maintenance};
use crate::paths::{NormaPaths, default_paths};
use crate::runtime::{RuntimeUpdate, RuntimeWatchers, runtime_update_channel, start_watchers};
use crate::skills::{SkillIndex, SkillsState, scan_skills};

pub struct RuntimeContext {
    pub paths: NormaPaths,
    pub config: Arc<Mutex<ConfigState>>,
    pub skills: Arc<Mutex<SkillsState>>,
    pub logging: LoggingGuard,
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
    let logging = init_tracing(&paths.log_dir, &config.logging)
        .context("failed to initialize Norma logging")?;
    tracing::info!(component = "runtime", "runtime bootstrap started");
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
        logging,
        watchers,
        updates: update_rx,
        app_state,
    })
}
