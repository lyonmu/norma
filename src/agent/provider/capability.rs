#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub tool_use: bool,
    pub structured_output: bool,
}
