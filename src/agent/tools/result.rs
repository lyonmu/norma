#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolInvocationResult {
    Succeeded { output_json: String },
    Failed { message: String },
}
