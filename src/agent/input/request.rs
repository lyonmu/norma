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
