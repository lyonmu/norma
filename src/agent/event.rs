#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentEvent {
    Started { task: String },
    Completed { event_count: usize },
}
