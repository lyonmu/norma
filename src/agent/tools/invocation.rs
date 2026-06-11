#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub input_json: String,
}
