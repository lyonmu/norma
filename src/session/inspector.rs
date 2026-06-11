use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Inspector,
    Context,
    Output,
    Settings,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunkSummary {
    pub index: usize,
    pub line_range: String,
    pub added_lines: usize,
    pub deleted_lines: usize,
    pub expanded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangePreview {
    pub path: PathBuf,
    pub hunks: Vec<DiffHunkSummary>,
}
