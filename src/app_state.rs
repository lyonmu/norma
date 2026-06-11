use std::env;

use crate::agent::{AgentRuntime, MockAgentRuntime};
use crate::config::AppConfig;
use crate::config::NormaConfig;
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
            config: AppConfig::sample(),
            runtime_paths: None,
            runtime_config: None,
            runtime_skills: SkillIndex::default(),
        }
    }

    pub fn load_current_project() -> Self {
        let root = env::current_dir().unwrap_or_else(|_| ".".into());
        Self::load_project(root)
    }

    pub fn load_project(root: impl Into<std::path::PathBuf>) -> Self {
        let root = root.into();
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
                    config: AppConfig::sample(),
                    runtime_paths: None,
                    runtime_config: None,
                    runtime_skills: SkillIndex::default(),
                };
            }
        };

        let files = load_file_tree(&project.root, 80).unwrap_or_else(|_| sample_file_tree());
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
            config: AppConfig::sample(),
            runtime_paths: None,
            runtime_config: None,
            runtime_skills: SkillIndex::default(),
        }
    }

    pub fn load_current_project_with_runtime(
        paths: NormaPaths,
        config: NormaConfig,
        skills: SkillIndex,
    ) -> Self {
        let mut state = Self::load_current_project();
        state.runtime_paths = Some(paths);
        state.runtime_config = Some(config);
        state.runtime_skills = skills;
        state
    }

    pub fn apply_runtime_update(&mut self, update: RuntimeUpdate) {
        match update {
            RuntimeUpdate::ConfigApplied(config) => {
                self.runtime_config = Some(config);
            }
            RuntimeUpdate::SkillsApplied(skills) => {
                self.runtime_skills = skills;
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
        assert_eq!(state.config.providers.len(), 2);
        assert_eq!(
            state.config.selected_provider().unwrap().name,
            "OpenAI 默认"
        );
    }

    #[test]
    fn settings_config_is_separate_from_session_events() {
        let state = NormaAppState::no_project();
        assert!(state.session.events.is_empty());
        assert!(!state.config.providers.is_empty());
    }
}
