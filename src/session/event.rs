use crate::git::ChangedFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Completed,
    Running,
    Waiting,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionStep {
    pub title: String,
    pub description: String,
    pub status: StepStatus,
    pub duration_label: Option<String>,
    pub checklist: Vec<ChecklistItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    pub label: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    UserTask {
        content: String,
    },
    AgentPlan {
        goal: String,
        constraints: Vec<String>,
    },
    StepUpdated(ExecutionStep),
    ChangeSummary {
        files: Vec<ChangedFile>,
    },
    FinalResponse {
        content: String,
    },
    Error {
        message: String,
    },
}
