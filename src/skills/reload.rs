use std::path::Path;

use notify;

use super::index::{SkillIndex, scan_skills};

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
                tracing::info!(component = "skills", "skills reload applied");
                SkillsReload::Applied(index)
            }
            Err(error) => {
                let message = error.to_string();
                self.last_error = Some(message.clone());
                tracing::warn!(component = "skills", error = %error, "skills reload rejected");
                SkillsReload::Rejected(message)
            }
        }
    }
}

pub fn is_skills_path_event(skills_dir: &Path, event: &notify::Event) -> bool {
    event.paths.iter().any(|path| path.starts_with(skills_dir))
}
