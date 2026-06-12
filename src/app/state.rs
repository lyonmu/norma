use std::env;

use crate::agent::{AgentRuntime, MockAgentRuntime, RealAgentRuntime};
use crate::config::{AppConfig, NormaConfig};
use crate::git::{GitStatusSummary, read_status};
use crate::paths::NormaPaths;
use crate::runtime::RuntimeUpdate;
use crate::session::{SessionState, sample_thread};
use crate::skills::SkillIndex;
use crate::workspace::{FileNode, Project, load_file_tree, open_project, sample_file_tree};

#[derive(Debug, Clone)]
pub enum ProjectSelectionState {
    NoProject,
    ProjectOpen(Project),
    OpenError {
        attempted_path: String,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct NormaAppState {
    pub project_state: ProjectSelectionState,
    pub files: Vec<FileNode>,
    pub git: GitStatusSummary,
    pub session: SessionState,
    pub config: AppConfig,
    pub runtime_paths: Option<NormaPaths>,
    pub runtime_config: Option<NormaConfig>,
    pub runtime_skills: SkillIndex,
    pub runtime_error: Option<String>,
}

impl NormaAppState {
    pub fn project_name(&self) -> String {
        match &self.project_state {
            ProjectSelectionState::ProjectOpen(project) => project.name.clone(),
            ProjectSelectionState::NoProject => "未打开项目".to_string(),
            ProjectSelectionState::OpenError { .. } => "项目打开失败".to_string(),
        }
    }

    pub fn project_path_label(&self) -> String {
        match &self.project_state {
            ProjectSelectionState::ProjectOpen(project) => project.root.display().to_string(),
            ProjectSelectionState::NoProject => "选择一个本地项目目录开始".to_string(),
            ProjectSelectionState::OpenError { attempted_path, .. } => attempted_path.clone(),
        }
    }

    pub fn no_project() -> Self {
        Self {
            project_state: ProjectSelectionState::NoProject,
            files: Vec::new(),
            git: GitStatusSummary::unavailable("no project open"),
            session: SessionState::new(sample_thread()),
            config: AppConfig::empty(),
            runtime_paths: None,
            runtime_config: None,
            runtime_skills: SkillIndex::default(),
            runtime_error: None,
        }
    }

    pub fn load_current_project() -> Self {
        let root = env::current_dir().unwrap_or_else(|_| ".".into());
        Self::load_project(root)
    }

    pub fn load_project(root: impl Into<std::path::PathBuf>) -> Self {
        let root = root.into();
        tracing::info!(component = "app", root = %root.display(), "loading project state");
        let project = match open_project(&root) {
            Ok(project) => project,
            Err(error) => {
                return Self {
                    project_state: ProjectSelectionState::OpenError {
                        attempted_path: root.display().to_string(),
                        message: error.to_string(),
                    },
                    files: sample_file_tree(),
                    git: GitStatusSummary::unavailable("project could not be opened"),
                    session: SessionState::new(sample_thread()),
                    config: AppConfig::empty(),
                    runtime_paths: None,
                    runtime_config: None,
                    runtime_skills: SkillIndex::default(),
                    runtime_error: None,
                };
            }
        };

        let files = load_file_tree(&project.root, 80).unwrap_or_else(|error| {
            tracing::warn!(
                component = "app",
                root = %project.root.display(),
                error = %error,
                "file tree fallback used"
            );
            sample_file_tree()
        });
        let git = read_status(&project.root);

        let runtime = MockAgentRuntime;
        let mut session = SessionState::new(sample_thread());
        for event in runtime.run_mock_task("完善 Norma 项目设计") {
            session.push_event(event);
        }

        Self {
            project_state: ProjectSelectionState::ProjectOpen(project),
            files,
            git,
            session,
            config: AppConfig::empty(),
            runtime_paths: None,
            runtime_config: None,
            runtime_skills: SkillIndex::default(),
            runtime_error: None,
        }
    }

    pub fn load_current_project_with_runtime(
        paths: NormaPaths,
        config: NormaConfig,
        skills: SkillIndex,
    ) -> Self {
        let mut state = Self::load_current_project();
        state.runtime_paths = Some(paths);
        state.config = AppConfig::from_norma_config(&config);
        match RealAgentRuntime::new(config.clone()) {
            Ok(_runtime) => {
                state.runtime_config = Some(config);
                state.runtime_error = None;
            }
            Err(error) => {
                tracing::warn!(component = "app", error = %error, "real agent runtime unavailable");
                state.runtime_config = Some(config);
                state.runtime_error = Some(error.to_string());
            }
        }
        state.runtime_skills = skills;
        state
    }

    pub fn apply_runtime_update(&mut self, update: RuntimeUpdate) {
        match update {
            RuntimeUpdate::ConfigApplied(config) => {
                self.config = AppConfig::from_norma_config(&config);
                self.runtime_error = RealAgentRuntime::new(config.clone()).err().map(|error| {
                    tracing::warn!(component = "app", error = %error, "real agent runtime unavailable");
                    error.to_string()
                });
                self.runtime_config = Some(config);
                tracing::info!(component = "app", "runtime config update applied");
            }
            RuntimeUpdate::SkillsApplied(skills) => {
                self.runtime_skills = skills;
                tracing::info!(
                    component = "app",
                    skill_count = self.runtime_skills.entries.len(),
                    "runtime skills update applied"
                );
            }
            RuntimeUpdate::ConfigRejected(message) | RuntimeUpdate::SkillsRejected(message) => {
                tracing::warn!(error = %message, "runtime update rejected");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_contains_mock_session_events() {
        let state = NormaAppState::load_current_project();
        assert!(!state.session.events.is_empty());
        assert!(!state.session.changed_files.is_empty());
    }

    #[test]
    fn no_project_state_has_no_files_or_git_repository() {
        let state = NormaAppState::no_project();
        assert!(matches!(
            state.project_state,
            ProjectSelectionState::NoProject
        ));
        assert!(state.files.is_empty());
        assert!(!state.git.is_repository);
    }

    #[test]
    fn app_state_includes_preview_provider_config() {
        let state = NormaAppState::no_project();
        assert!(state.config.providers.is_empty());
        assert!(state.config.selected_provider().is_none());
    }

    #[test]
    fn settings_config_is_separate_from_session_events() {
        let state = NormaAppState::no_project();
        assert!(state.session.events.is_empty());
        assert!(state.config.providers.is_empty());
    }

    #[test]
    fn runtime_config_populates_settings_provider_rows_from_persisted_config() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut runtime_config = crate::config::NormaConfig::default_for(&paths);
        runtime_config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI 默认".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-openai-default".to_string(),
            is_default: true,
            models: vec![crate::config::AiModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o mini".to_string(),
                model_id: "gpt-4o-mini".to_string(),
                is_default: true,
            }],
        }];

        let state = NormaAppState::load_current_project_with_runtime(
            paths,
            runtime_config,
            SkillIndex::default(),
        );

        assert_eq!(state.config.providers.len(), 1);
        assert_eq!(
            state.config.selected_provider().unwrap().id,
            "openai-default"
        );
        assert!(state.config.selected_provider().unwrap().is_default);
    }

    #[test]
    fn runtime_config_reload_refreshes_settings_view_model() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut initial = crate::config::NormaConfig::default_for(&paths);
        initial.ai.providers = vec![crate::config::AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI 默认".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-openai-default".to_string(),
            is_default: true,
            models: vec![crate::config::AiModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o mini".to_string(),
                model_id: "gpt-4o-mini".to_string(),
                is_default: true,
            }],
        }];

        let mut state = NormaAppState::load_current_project_with_runtime(
            paths,
            initial.clone(),
            SkillIndex::default(),
        );

        let mut next = initial;
        next.ai.providers[0].name = "OpenAI 主提供商".to_string();

        state.apply_runtime_update(RuntimeUpdate::ConfigApplied(next.clone()));

        assert_eq!(state.runtime_config.as_ref().unwrap(), &next);
        assert_eq!(
            state.config.selected_provider().unwrap().name,
            "OpenAI 主提供商"
        );
    }

    #[test]
    fn runtime_config_with_valid_default_provider_keeps_startup_session_mock_only() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut runtime_config = crate::config::NormaConfig::default_for(&paths);
        runtime_config.ai.providers = vec![crate::config::AiProviderConfig {
            id: "openai-default".to_string(),
            name: "OpenAI 默认".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-openai-default".to_string(),
            is_default: true,
            models: vec![crate::config::AiModelConfig {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o mini".to_string(),
                model_id: "gpt-4o-mini".to_string(),
                is_default: true,
            }],
        }];

        let state = NormaAppState::load_current_project_with_runtime(
            paths,
            runtime_config,
            SkillIndex::default(),
        );

        assert!(state.runtime_error.is_none());
        assert!(
            state
                .session
                .events
                .iter()
                .any(|event| matches!(event, crate::session::SessionEvent::AgentPlan { .. }))
        );
        assert!(
            !state
                .session
                .events
                .iter()
                .any(|event| matches!(event, crate::session::SessionEvent::Error { .. }))
        );
    }

    #[test]
    fn runtime_config_reload_recomputes_runtime_error() {
        let root = tempfile::tempdir().unwrap();
        let paths = crate::paths::NormaPaths::from_home(root.path());
        let mut invalid = crate::config::NormaConfig::default_for(&paths);
        invalid.ai.providers = vec![crate::config::AiProviderConfig {
            id: "broken".to_string(),
            name: "Broken".to_string(),
            api_type: crate::config::ProviderApiType::OpenAi,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: "sk-test-broken".to_string(),
            is_default: true,
            models: vec![],
        }];
        let mut valid = invalid.clone();
        valid.ai.providers[0].models = vec![crate::config::AiModelConfig {
            id: "gpt-4o-mini".to_string(),
            name: "GPT-4o mini".to_string(),
            model_id: "gpt-4o-mini".to_string(),
            is_default: true,
        }];

        let mut state =
            NormaAppState::load_current_project_with_runtime(paths, invalid, SkillIndex::default());
        assert!(state.runtime_error.is_some());

        state.apply_runtime_update(RuntimeUpdate::ConfigApplied(valid));

        assert!(state.runtime_error.is_none());
    }
}
