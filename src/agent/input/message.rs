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
