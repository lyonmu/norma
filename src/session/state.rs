use crate::git::ChangedFile;

use super::event::SessionEvent;
use super::inspector::{DiffHunkSummary, FileChangePreview, InspectorTab};
use super::thread::SessionThread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub thread: SessionThread,
    pub events: Vec<SessionEvent>,
    pub active_tab: InspectorTab,
    pub changed_files: Vec<ChangedFile>,
    pub selected_change: Option<FileChangePreview>,
}

impl SessionState {
    pub fn new(thread: SessionThread) -> Self {
        Self {
            thread,
            events: Vec::new(),
            active_tab: InspectorTab::Context,
            changed_files: Vec::new(),
            selected_change: None,
        }
    }

    pub fn push_event(&mut self, event: SessionEvent) {
        if let SessionEvent::ChangeSummary { files } = &event {
            self.active_tab = InspectorTab::Inspector;
            self.changed_files = files.clone();
            self.selected_change = files.first().map(|file| FileChangePreview {
                path: file.path.clone(),
                hunks: (1..=file.hunk_count.max(1))
                    .map(|index| DiffHunkSummary {
                        index,
                        line_range: format!("行 {}-{}", 10 * index, 10 * index + 8),
                        added_lines: file.added_lines / file.hunk_count.max(1),
                        deleted_lines: file.deleted_lines / file.hunk_count.max(1),
                        expanded: false,
                    })
                    .collect(),
            });
        }
        if matches!(event, SessionEvent::Error { .. }) {
            self.active_tab = InspectorTab::Approval;
        }
        self.events.push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::super::thread::sample_thread;
    use super::*;
    use crate::git::ChangeKind;
    use std::path::PathBuf;

    #[test]
    fn starts_in_context_mode() {
        let state = SessionState::new(sample_thread());
        assert_eq!(state.active_tab, InspectorTab::Context);
        assert!(state.events.is_empty());
        assert!(state.changed_files.is_empty());
    }

    #[test]
    fn change_summary_switches_to_inspector_tab_and_sets_review_state() {
        let mut state = SessionState::new(sample_thread());
        state.push_event(SessionEvent::ChangeSummary {
            files: vec![ChangedFile {
                path: PathBuf::from("src/ui/inspector.rs"),
                kind: ChangeKind::Modified,
                added_lines: 86,
                deleted_lines: 10,
                hunk_count: 4,
            }],
        });

        assert_eq!(state.active_tab, InspectorTab::Inspector);
        assert_eq!(state.changed_files.len(), 1);
        assert_eq!(state.selected_change.as_ref().unwrap().hunks.len(), 4);
    }

    #[test]
    fn error_switches_to_approval_mode() {
        let mut state = SessionState::new(sample_thread());
        state.push_event(SessionEvent::Error {
            message: "需要人工确认".to_string(),
        });
        assert_eq!(state.active_tab, InspectorTab::Approval);
    }
}
