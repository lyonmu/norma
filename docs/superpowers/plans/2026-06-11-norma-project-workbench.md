# Norma Project Workbench Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Norma V1 as a GPUI desktop project workbench shell that matches the Review-First Codex visual target, supports no-project and project-open states, reads real local project files and Git status, and renders a mock agent execution stream without modifying project files.

**Architecture:** Keep `main.rs` small and split the app into focused modules: `workspace` for project/file context, `git` for read-only repository summaries, `session` for threads/events/inspector review state, `agent` for the mock runtime, `app_state` for no-project/project-open assembly, and `ui` for GPUI views. Build and test data/state modules first, then wire the UI to those states. V1 must display compare/revert/undo controls only as disabled or preview-only affordances.

**Tech Stack:** Rust 2024, GPUI `0.2.2`, `ignore` for file traversal with ignore-file support, `thiserror` for typed errors, `anyhow` for app bootstrap context, standard `std::process::Command` only for the read-only Git status query in V1.

---

## Scope Check

The spec includes future roadmap items for model providers, MCP, Skills, ACP, Sub-Agent, Multi-Agent, real patching, and destructive Git operations. This plan implements only V1 from `docs/superpowers/specs/2026-06-11-norma-project-workbench-design.md`: no-project and project-open shell states, visual contract, real file tree, basic read-only Git status, session/event state, session-derived mock change review state, mock runtime, and disabled/preview-only Git action UI.

## Dependency And Toolchain Facts

Current verification from `cargo search --registry crates-io` on 2026-06-11:

- `gpui = "0.2.2"`
- `ignore = "0.4.26"`
- `thiserror = "2.0.18"`
- `anyhow = "1.0.102"`

The crate must keep `edition = "2024"` in `Cargo.toml`. Do not add `gpui_platform`; a previous dependency attempt failed because that public crate name is not the correct GPUI dependency. If `cargo check` fails while compiling GPUI on macOS with a Metal toolchain error, install the Metal Toolchain via `xcodebuild -downloadComponent MetalToolchain` before changing Rust code.

## Target File Structure

Create or modify these files:

- Modify: `Cargo.toml`  
  Pin GPUI to the latest verified version and add focused dependencies.

- Modify: `src/main.rs`  
  Keep app bootstrap only: create the GPUI application, load fixture app state, open the main window.

- Create: `src/lib.rs`  
  Export app modules so unit tests can target them.

- Create: `src/workspace.rs`  
  Own `Project`, `FileNode`, workspace errors, file tree loading through `ignore::WalkBuilder`, hidden/internal directory filtering, and deterministic sample fallback data.

- Create: `src/git.rs`  
  Own read-only Git status models and parsing for `git status --porcelain=v1 --branch`. No mutating Git commands.

- Create: `src/session.rs`  
  Own `SessionThread`, `SessionEvent`, execution-step state, `InspectorTab`, session-derived review state, and derived `SessionState`.

- Create: `src/agent.rs`  
  Define `AgentRuntime` and `MockAgentRuntime` that emits deterministic V1 events for the visual contract.

- Create: `src/app_state.rs`  
  Build the in-memory `NormaAppState` used by UI components, including `ProjectSelectionState` for no-project and project-open states.

- Create: `src/ui/mod.rs`  
  Re-export UI modules.

- Create: `src/ui/theme.rs`  
  Centralize colors, spacing, and text sizes that match the visual target.

- Create: `src/ui/components.rs`  
  Small reusable GPUI helpers: pill, icon button, section header, metric tile, disabled action row.

- Create: `src/ui/shell.rs`  
  Own `AppShell`, top toolbar, three-column layout, and shell-level rendering.

- Create: `src/ui/sidebar.rs`  
  Project card, grouped thread list, file tree, and Git status card.

- Create: `src/ui/execution.rs`  
  Task header, task summary block, execution stream cards, timeline rail, and bottom composer.

- Create: `src/ui/inspector.rs`  
  Inspector tabs, change overview metrics, safety row, changed files list, file hunk preview, Git operation rows.

- Create: `tests/visual_contract.md`  
  Manual screenshot checklist for verifying the GPUI window against `docs/superpowers/specs/assets/norma-review-first-codex-workbench.png`.

## Commit Strategy

Commit after each task. Use short imperative commit messages:

- `chore: pin V1 dependencies`
- `feat: add workspace file tree model`
- `feat: add read-only git status model`
- `feat: add session event model`
- `feat: add mock agent runtime`
- `feat: add Norma app state`
- `feat: add GPUI app shell`
- `feat: add sidebar workbench UI`
- `feat: add execution stream UI`
- `feat: add dynamic inspector UI`
- `test: add visual verification checklist`

---

### Task 1: Pin Dependencies And Preserve Rust 2024

**Files:**
- Modify: `Cargo.toml`
- Verify: `Cargo.lock`

- [ ] **Step 1: Confirm latest compatible crate versions**

Run:

```bash
cargo search gpui --registry crates-io --limit 5
cargo search ignore --registry crates-io --limit 5
cargo search thiserror --registry crates-io --limit 5
cargo search anyhow --registry crates-io --limit 5
```

Expected:

```text
gpui = "0.2.2"
ignore = "0.4.26"
thiserror = "2.0.18"
anyhow = "1.0.102"
```

- [ ] **Step 2: Update dependencies**

Change the `[dependencies]` section in `Cargo.toml` to:

```toml
[dependencies]
anyhow = "1.0.102"
gpui = "0.2.2"
ignore = "0.4.26"
thiserror = "2.0.18"
```

Keep this line unchanged:

```toml
edition = "2024"
```

- [ ] **Step 3: Verify metadata still reports Rust 2024**

Run:

```bash
cargo metadata --no-deps --format-version 1 | rg '"edition":"2024"'
```

Expected: one match for the `norma` package.

- [ ] **Step 4: Check dependency resolution**

Run:

```bash
cargo check
```

Expected: either PASS, or a GPUI/macOS toolchain error. If the failure mentions Metal Toolchain, run:

```bash
xcodebuild -downloadComponent MetalToolchain
```

Then rerun:

```bash
cargo check
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: pin V1 dependencies"
```

---

### Task 2: Add Workspace File Tree Model

**Files:**
- Create: `src/lib.rs`
- Create: `src/workspace.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Expose modules from the library crate**

Create `src/lib.rs`:

```rust
pub mod workspace;
```

- [ ] **Step 2: Write failing workspace tests**

Create `src/workspace.rs` with these tests first:

```rust
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkspaceError {
    #[error("project path does not exist: {0}")]
    MissingPath(PathBuf),
    #[error("project path is not a directory: {0}")]
    NotDirectory(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub kind: FileKind,
    pub depth: usize,
}

pub fn sample_file_tree() -> Vec<FileNode> {
    [
        ("src", FileKind::Directory, 0),
        ("main.rs", FileKind::File, 1),
        ("README.md", FileKind::File, 0),
        ("Cargo.toml", FileKind::File, 0),
    ]
    .into_iter()
    .map(|(name, kind, depth)| FileNode {
        path: PathBuf::from(name),
        name: name.to_string(),
        kind,
        depth,
    })
    .collect()
}

pub fn open_project(path: impl AsRef<Path>) -> Result<Project, WorkspaceError> {
    let root = path.as_ref().to_path_buf();
    if !root.exists() {
        return Err(WorkspaceError::MissingPath(root));
    }
    if !root.is_dir() {
        return Err(WorkspaceError::NotDirectory(root));
    }
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Project")
        .to_string();
    Ok(Project { name, root })
}

pub fn load_file_tree(root: impl AsRef<Path>, max_entries: usize) -> Result<Vec<FileNode>, WorkspaceError> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(WorkspaceError::MissingPath(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(WorkspaceError::NotDirectory(root.to_path_buf()));
    }

    let mut nodes = Vec::new();
    for entry in WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .parents(true)
        .max_depth(Some(4))
        .build()
        .filter_map(Result::ok)
    {
        if entry.path() == root {
            continue;
        }
        if nodes.len() >= max_entries {
            break;
        }
        let path = entry.path().to_path_buf();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let depth = path
            .strip_prefix(root)
            .map(|relative| relative.components().count().saturating_sub(1))
            .unwrap_or(0);
        let kind = if path.is_dir() {
            FileKind::Directory
        } else {
            FileKind::File
        };
        nodes.push(FileNode {
            path,
            name,
            kind,
            depth,
        });
    }
    nodes.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("norma-workspace-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn opens_project_from_existing_directory() {
        let root = test_root("opens-project");
        let project = open_project(&root).unwrap();
        assert_eq!(project.name, root.file_name().unwrap().to_string_lossy());
        assert_eq!(project.root, root);
    }

    #[test]
    fn rejects_missing_project_path() {
        let root = std::env::temp_dir().join("norma-workspace-test-missing");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(open_project(&root), Err(WorkspaceError::MissingPath(root)));
    }

    #[test]
    fn loads_limited_file_tree_with_depth() {
        let root = test_root("file-tree");
        fs::create_dir_all(root.join("src/ui")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("README.md"), "# Norma\n").unwrap();

        let nodes = load_file_tree(&root, 10).unwrap();
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();

        assert!(names.contains(&"src"));
        assert!(names.contains(&"ui"));
        assert!(names.contains(&"main.rs"));
        assert!(names.contains(&"README.md"));
        assert!(nodes.iter().any(|node| node.name == "main.rs" && node.depth == 1));
    }

    #[test]
    fn hides_internal_dot_directories_from_visible_tree() {
        let root = test_root("hidden-dirs");
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(root.join("README.md"), "# Norma\n").unwrap();

        let nodes = load_file_tree(&root, 10).unwrap();
        let names: Vec<_> = nodes.iter().map(|node| node.name.as_str()).collect();

        assert!(names.contains(&"README.md"));
        assert!(!names.contains(&".git"));
        assert!(!names.contains(&"HEAD"));
    }

    #[test]
    fn sample_file_tree_is_deterministic() {
        let first = sample_file_tree();
        let second = sample_file_tree();
        assert_eq!(first, second);
        assert!(first.iter().any(|node| node.name == "README.md"));
    }
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test workspace -- --nocapture
```

Expected: PASS after adding the code above.

- [ ] **Step 4: Keep `main.rs` compiling**

Keep `src/main.rs` minimal until the UI shell task:

```rust
fn main() {
    println!("Norma workbench shell");
}
```

- [ ] **Step 5: Verify**

Run:

```bash
cargo fmt --check
cargo check
cargo test workspace
```

Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/workspace.rs src/main.rs
git commit -m "feat: add workspace file tree model"
```

---

### Task 3: Add Read-Only Git Status Model

**Files:**
- Modify: `src/lib.rs`
- Create: `src/git.rs`

- [ ] **Step 1: Export the Git module**

Update `src/lib.rs`:

```rust
pub mod git;
pub mod workspace;
```

- [ ] **Step 2: Create Git parser and tests**

Create `src/git.rs`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("git executable could not be started: {0}")]
    StartFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: PathBuf,
    pub kind: ChangeKind,
    pub added_lines: usize,
    pub deleted_lines: usize,
    pub hunk_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitStatusSummary {
    pub is_repository: bool,
    pub branch: Option<String>,
    pub ahead: usize,
    pub behind: usize,
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
    pub files: Vec<ChangedFile>,
    pub error: Option<String>,
}

impl GitStatusSummary {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            is_repository: false,
            branch: None,
            ahead: 0,
            behind: 0,
            modified: 0,
            added: 0,
            deleted: 0,
            untracked: 0,
            files: Vec::new(),
            error: Some(message.into()),
        }
    }
}

pub fn read_status(root: impl AsRef<Path>) -> GitStatusSummary {
    let output = Command::new("git")
        .arg("-C")
        .arg(root.as_ref())
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--branch")
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return GitStatusSummary::unavailable(format!("failed to start git: {error}"));
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return GitStatusSummary::unavailable(if stderr.is_empty() {
            "not a git repository".to_string()
        } else {
            stderr
        });
    }

    parse_status(&String::from_utf8_lossy(&output.stdout))
}

pub fn parse_status(output: &str) -> GitStatusSummary {
    let mut summary = GitStatusSummary {
        is_repository: true,
        branch: None,
        ahead: 0,
        behind: 0,
        modified: 0,
        added: 0,
        deleted: 0,
        untracked: 0,
        files: Vec::new(),
        error: None,
    };

    for line in output.lines() {
        if let Some(branch_line) = line.strip_prefix("## ") {
            parse_branch(branch_line, &mut summary);
            continue;
        }
        if line.len() < 3 {
            continue;
        }

        let status = &line[..2];
        let path = line[3..].split(" -> ").last().unwrap_or("").trim();
        if path.is_empty() {
            continue;
        }

        let kind = parse_change_kind(status);
        match kind {
            ChangeKind::Modified => summary.modified += 1,
            ChangeKind::Added => summary.added += 1,
            ChangeKind::Deleted => summary.deleted += 1,
            ChangeKind::Renamed => summary.modified += 1,
            ChangeKind::Untracked => summary.untracked += 1,
            ChangeKind::Other => summary.modified += 1,
        }

        summary.files.push(ChangedFile {
            path: PathBuf::from(path),
            kind,
            added_lines: mock_added_lines(kind),
            deleted_lines: mock_deleted_lines(kind),
            hunk_count: mock_hunk_count(kind),
        });
    }

    summary
}

fn parse_branch(branch_line: &str, summary: &mut GitStatusSummary) {
    let branch_name = branch_line
        .split("...")
        .next()
        .unwrap_or(branch_line)
        .trim()
        .to_string();
    summary.branch = Some(branch_name);

    if let Some(details) = branch_line.split('[').nth(1).and_then(|value| value.strip_suffix(']')) {
        for part in details.split(',') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix("ahead ") {
                summary.ahead = value.parse().unwrap_or(0);
            }
            if let Some(value) = part.strip_prefix("behind ") {
                summary.behind = value.parse().unwrap_or(0);
            }
        }
    }
}

fn parse_change_kind(status: &str) -> ChangeKind {
    if status == "??" {
        return ChangeKind::Untracked;
    }
    if status.contains('R') {
        return ChangeKind::Renamed;
    }
    if status.contains('A') {
        return ChangeKind::Added;
    }
    if status.contains('D') {
        return ChangeKind::Deleted;
    }
    if status.contains('M') {
        return ChangeKind::Modified;
    }
    ChangeKind::Other
}

fn mock_added_lines(kind: ChangeKind) -> usize {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => 24,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Other => 8,
        ChangeKind::Deleted => 0,
    }
}

fn mock_deleted_lines(kind: ChangeKind) -> usize {
    match kind {
        ChangeKind::Deleted => 18,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Other => 3,
        ChangeKind::Added | ChangeKind::Untracked => 0,
    }
}

fn mock_hunk_count(kind: ChangeKind) -> usize {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => 1,
        ChangeKind::Deleted => 1,
        ChangeKind::Modified | ChangeKind::Renamed | ChangeKind::Other => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_counts_and_changed_files() {
        let output = "\
## main...origin/main [ahead 2, behind 1]
 M src/main.rs
A  src/lib.rs
 D old.rs
R  old_name.rs -> new_name.rs
?? docs/spec.md
";

        let summary = parse_status(output);

        assert!(summary.is_repository);
        assert_eq!(summary.branch.as_deref(), Some("main"));
        assert_eq!(summary.ahead, 2);
        assert_eq!(summary.behind, 1);
        assert_eq!(summary.modified, 2);
        assert_eq!(summary.added, 1);
        assert_eq!(summary.deleted, 1);
        assert_eq!(summary.untracked, 1);
        assert_eq!(summary.files.len(), 5);
        assert_eq!(summary.files[3].path, PathBuf::from("new_name.rs"));
    }

    #[test]
    fn unavailable_summary_is_non_repository() {
        let summary = GitStatusSummary::unavailable("not a git repository");
        assert!(!summary.is_repository);
        assert_eq!(summary.error.as_deref(), Some("not a git repository"));
        assert!(summary.files.is_empty());
    }
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test git -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Verify destructive commands are absent**

Run:

```bash
rg -n 'reset|restore|checkout|clean|apply|commit|push' src/git.rs
```

Expected: no matches, except if one of these words appears in a test assertion message. If a command invocation includes one of those words, remove it from V1.

- [ ] **Step 5: Verify all tests**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/git.rs
git commit -m "feat: add read-only git status model"
```

---

### Task 4: Add Session Event And Inspector State Model

**Files:**
- Modify: `src/lib.rs`
- Create: `src/session.rs`

- [ ] **Step 1: Export the session module**

Update `src/lib.rs`:

```rust
pub mod git;
pub mod session;
pub mod workspace;
```

- [ ] **Step 2: Create session state types and tests**

Create `src/session.rs`:

```rust
use std::path::PathBuf;

use crate::git::ChangedFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Inspector,
    Context,
    Output,
    Settings,
    Approval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionThread {
    pub id: String,
    pub project_name: String,
    pub title: String,
    pub updated_label: String,
}

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
    UserTask { content: String },
    AgentPlan { goal: String, constraints: Vec<String> },
    StepUpdated(ExecutionStep),
    ChangeSummary { files: Vec<ChangedFile> },
    FinalResponse { content: String },
    Error { message: String },
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

pub fn sample_thread() -> SessionThread {
    SessionThread {
        id: "thread-design".to_string(),
        project_name: "norma".to_string(),
        title: "完善 Norma 项目设计".to_string(),
        updated_label: "14:32".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{ChangeKind, ChangedFile};

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
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test session -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Verify all tests**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/session.rs
git commit -m "feat: add session event model"
```

---

### Task 5: Add Mock Agent Runtime

**Files:**
- Modify: `src/lib.rs`
- Create: `src/agent.rs`

- [ ] **Step 1: Export the agent module**

Update `src/lib.rs`:

```rust
pub mod agent;
pub mod git;
pub mod session;
pub mod workspace;
```

- [ ] **Step 2: Create mock runtime and tests**

Create `src/agent.rs`:

```rust
use std::path::PathBuf;

use crate::git::{ChangeKind, ChangedFile};
use crate::session::{ChecklistItem, ExecutionStep, SessionEvent, StepStatus};

pub trait AgentRuntime {
    fn run_mock_task(&self, task: &str) -> Vec<SessionEvent>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MockAgentRuntime;

impl AgentRuntime for MockAgentRuntime {
    fn run_mock_task(&self, task: &str) -> Vec<SessionEvent> {
        vec![
            SessionEvent::UserTask {
                content: task.to_string(),
            },
            SessionEvent::AgentPlan {
                goal: "完善 Norma 的整体设计、架构、模块、数据流、UI、Git 交互".to_string(),
                constraints: vec![
                    "不内置代码编辑器".to_string(),
                    "专注项目上下文、执行流、变更审查与回滚".to_string(),
                    "V1 不执行破坏性 Git 操作".to_string(),
                ],
            },
            SessionEvent::StepUpdated(completed_step(
                "读取 README",
                "读取并解析 README.md 与 README.zh.md，提取产品定位与目标。",
                "18.4s",
            )),
            SessionEvent::StepUpdated(completed_step(
                "确认产品边界",
                "确认 Norma 不包含代码编辑器，聚焦执行流与变更审查。",
                "15.7s",
            )),
            SessionEvent::StepUpdated(running_step()),
            SessionEvent::ChangeSummary {
                files: vec![
                    changed("src/ui/inspector.rs", 86, 10, 4),
                    changed("src/ui/execution_item.rs", 42, 3, 2),
                    changed("src/agent/runner.rs", 38, 2, 2),
                    changed("src/git/repository.rs", 29, 5, 2),
                    changed("src/config/settings.rs", 24, 0, 1),
                ],
            },
            SessionEvent::StepUpdated(waiting_step()),
        ]
    }
}

fn completed_step(title: &str, description: &str, duration: &str) -> ExecutionStep {
    ExecutionStep {
        title: title.to_string(),
        description: description.to_string(),
        status: StepStatus::Completed,
        duration_label: Some(duration.to_string()),
        checklist: Vec::new(),
    }
}

fn running_step() -> ExecutionStep {
    ExecutionStep {
        title: "生成 Codex 风格 UI".to_string(),
        description: "设计三栏工作台：线程侧栏、执行流、动态检查器。".to_string(),
        status: StepStatus::Running,
        duration_label: Some("32.1s".to_string()),
        checklist: vec![
            ChecklistItem { label: "分析 Codex 设计语言".to_string(), status: StepStatus::Completed },
            ChecklistItem { label: "制定三栏布局与信息层级".to_string(), status: StepStatus::Completed },
            ChecklistItem { label: "设计右侧检查器（Diff + Git）".to_string(), status: StepStatus::Completed },
            ChecklistItem { label: "制作高保真界面草图".to_string(), status: StepStatus::Running },
            ChecklistItem { label: "评审与优化".to_string(), status: StepStatus::Waiting },
            ChecklistItem { label: "输出交互规范".to_string(), status: StepStatus::Waiting },
        ],
    }
}

fn waiting_step() -> ExecutionStep {
    ExecutionStep {
        title: "检查变更摘要".to_string(),
        description: "汇总变更文件与行数，生成审查摘要。".to_string(),
        status: StepStatus::Waiting,
        duration_label: None,
        checklist: Vec::new(),
    }
}

fn changed(path: &str, added_lines: usize, deleted_lines: usize, hunk_count: usize) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        kind: ChangeKind::Modified,
        added_lines,
        deleted_lines,
        hunk_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_runtime_emits_visual_contract_states() {
        let runtime = MockAgentRuntime;
        let events = runtime.run_mock_task("完善 Norma 项目设计");

        assert!(matches!(events.first(), Some(SessionEvent::UserTask { .. })));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::AgentPlan { .. })));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::ChangeSummary { files } if files.len() == 5)));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::StepUpdated(step) if step.status == StepStatus::Running && step.checklist.len() == 6)));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::StepUpdated(step) if step.status == StepStatus::Waiting)));
    }
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test agent -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Verify all tests**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/agent.rs
git commit -m "feat: add mock agent runtime"
```

---

### Task 6: Add App State Assembly

**Files:**
- Modify: `src/lib.rs`
- Create: `src/app_state.rs`

- [ ] **Step 1: Export app state**

Update `src/lib.rs`:

```rust
pub mod agent;
pub mod app_state;
pub mod git;
pub mod session;
pub mod workspace;
```

- [ ] **Step 2: Create app state and tests**

Create `src/app_state.rs`:

```rust
use std::env;

use crate::agent::{AgentRuntime, MockAgentRuntime};
use crate::git::{GitStatusSummary, read_status};
use crate::session::{SessionState, sample_thread};
use crate::workspace::{FileNode, Project, load_file_tree, open_project, sample_file_tree};

#[derive(Debug, Clone)]
pub enum ProjectSelectionState {
    NoProject,
    ProjectOpen(Project),
    OpenError { attempted_path: String, message: String },
}

#[derive(Debug, Clone)]
pub struct NormaAppState {
    pub project_state: ProjectSelectionState,
    pub files: Vec<FileNode>,
    pub git: GitStatusSummary,
    pub session: SessionState,
}

impl NormaAppState {
    pub fn project_name(&self) -> String {
        match &self.project_state {
            ProjectSelectionState::ProjectOpen(project) => project.name.clone(),
            ProjectSelectionState::NoProject => "未打开项目".to_string(),
            ProjectSelectionState::OpenError { .. } => "项目打开失败".to_string(),
        }
    }

    pub fn project_path_label(&self) -> String {
        match &self.project_state {
            ProjectSelectionState::ProjectOpen(project) => project.root.display().to_string(),
            ProjectSelectionState::NoProject => "选择一个本地项目目录开始".to_string(),
            ProjectSelectionState::OpenError { attempted_path, .. } => attempted_path.clone(),
        }
    }

    pub fn no_project() -> Self {
        Self {
            project_state: ProjectSelectionState::NoProject,
            files: Vec::new(),
            git: GitStatusSummary::unavailable("no project open"),
            session: SessionState::new(sample_thread()),
        }
    }

    pub fn load_current_project() -> Self {
        let root = env::current_dir().unwrap_or_else(|_| ".".into());
        Self::load_project(root)
    }

    pub fn load_project(root: impl Into<std::path::PathBuf>) -> Self {
        let root = root.into();
        let project = match open_project(&root) {
            Ok(project) => project,
            Err(error) => {
                return Self {
                    project_state: ProjectSelectionState::OpenError {
                        attempted_path: root.display().to_string(),
                        message: error.to_string(),
                    },
                    files: sample_file_tree(),
                    git: GitStatusSummary::unavailable("project could not be opened"),
                    session: SessionState::new(sample_thread()),
                };
            }
        };

        let files = load_file_tree(&project.root, 80).unwrap_or_else(|_| sample_file_tree());
        let git = read_status(&project.root);

        let runtime = MockAgentRuntime;
        let mut session = SessionState::new(sample_thread());
        for event in runtime.run_mock_task("完善 Norma 项目设计") {
            session.push_event(event);
        }

        Self {
            project_state: ProjectSelectionState::ProjectOpen(project),
            files,
            git,
            session,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_contains_mock_session_events() {
        let state = NormaAppState::load_current_project();
        assert!(!state.session.events.is_empty());
        assert!(!state.session.changed_files.is_empty());
    }

    #[test]
    fn no_project_state_has_no_files_or_git_repository() {
        let state = NormaAppState::no_project();
        assert!(matches!(state.project_state, ProjectSelectionState::NoProject));
        assert!(state.files.is_empty());
        assert!(!state.git.is_repository);
    }
}
```

- [ ] **Step 3: Run tests**

Run:

```bash
cargo test app_state -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Verify**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/app_state.rs
git commit -m "feat: add Norma app state"
```

---

### Task 7: Build GPUI App Shell And Theme

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Create: `src/ui/mod.rs`
- Create: `src/ui/theme.rs`
- Create: `src/ui/components.rs`
- Create: `src/ui/shell.rs`

- [ ] **Step 1: Export UI module**

Update `src/lib.rs`:

```rust
pub mod agent;
pub mod app_state;
pub mod git;
pub mod session;
pub mod ui;
pub mod workspace;
```

Create `src/ui/mod.rs`:

```rust
pub mod components;
pub mod shell;
pub mod theme;
```

- [ ] **Step 2: Create theme constants**

Create `src/ui/theme.rs`:

```rust
use gpui::{Hsla, hsla, px};

pub const TOOLBAR_HEIGHT: gpui::Pixels = px(56.);
pub const SIDEBAR_WIDTH: gpui::Pixels = px(320.);
pub const INSPECTOR_WIDTH: gpui::Pixels = px(410.);

pub fn app_bg() -> Hsla {
    hsla(220. / 360., 0.16, 0.97, 1.)
}

pub fn surface() -> Hsla {
    hsla(0., 0., 1., 1.)
}

pub fn surface_tint() -> Hsla {
    hsla(218. / 360., 0.35, 0.96, 1.)
}

pub fn border() -> Hsla {
    hsla(220. / 360., 0.16, 0.88, 1.)
}

pub fn text() -> Hsla {
    hsla(222. / 360., 0.25, 0.13, 1.)
}

pub fn muted() -> Hsla {
    hsla(220. / 360., 0.08, 0.46, 1.)
}

pub fn blue() -> Hsla {
    hsla(218. / 360., 0.88, 0.56, 1.)
}

pub fn green() -> Hsla {
    hsla(145. / 360., 0.54, 0.40, 1.)
}

pub fn red() -> Hsla {
    hsla(356. / 360., 0.71, 0.52, 1.)
}
```

- [ ] **Step 3: Create reusable UI helpers**

Create `src/ui/components.rs`:

```rust
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, prelude::*, px};

use crate::ui::theme;

pub fn label(text: impl Into<String>) -> AnyElement {
    div()
        .text_size(px(13.))
        .text_color(theme::muted())
        .child(text.into())
        .into_any_element()
}

pub fn section_title(text: impl Into<String>) -> AnyElement {
    div()
        .text_size(px(13.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme::text())
        .child(text.into())
        .into_any_element()
}

pub fn pill(text: impl Into<String>, active: bool) -> AnyElement {
    let bg = if active { theme::blue() } else { theme::surface_tint() };
    let fg = if active { theme::surface() } else { theme::muted() };
    div()
        .px_2()
        .py_1()
        .rounded(px(6.))
        .bg(bg)
        .text_color(fg)
        .text_size(px(12.))
        .child(text.into())
        .into_any_element()
}

pub fn icon_button(text: impl Into<String>) -> AnyElement {
    div()
        .w(px(32.))
        .h(px(32.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme::border())
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.))
        .text_color(theme::text())
        .child(text.into())
        .into_any_element()
}
```

- [ ] **Step 4: Create shell view**

Create `src/ui/shell.rs`:

```rust
use gpui::{
    App, Application, Context, IntoElement, ParentElement, Render, Styled, Window, WindowBounds,
    WindowOptions, div, point, prelude::*, px, size,
};

use crate::app_state::NormaAppState;
use crate::ui::{components, theme};

pub struct AppShell {
    state: NormaAppState,
}

impl AppShell {
    pub fn new(state: NormaAppState) -> Self {
        Self { state }
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .flex()
            .flex_col()
            .child(top_toolbar())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .w(theme::SIDEBAR_WIDTH)
                            .h_full()
                            .border_r_1()
                            .border_color(theme::border())
                            .child(components::section_title(format!("{} sidebar", self.state.project_name()))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .p_6()
                            .child(components::section_title(self.state.session.thread.title.clone())),
                    )
                    .child(
                        div()
                            .w(theme::INSPECTOR_WIDTH)
                            .h_full()
                            .border_l_1()
                            .border_color(theme::border())
                            .p_5()
                            .child(components::section_title("检查器")),
                    ),
            )
    }
}

fn top_toolbar() -> impl IntoElement {
    div()
        .h(theme::TOOLBAR_HEIGHT)
        .w_full()
        .px_5()
        .border_b_1()
        .border_color(theme::border())
        .flex()
        .items_center()
        .justify_between()
        .bg(theme::surface())
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(components::icon_button("N"))
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Norma"))
                .child(components::icon_button("←"))
                .child(components::icon_button("→")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(components::pill("模型 GPT-4.1", false))
                .child(components::pill("运行环境 本地", false))
                .child(components::pill("安全级别 标准", false)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(components::icon_button("▶"))
                .child(components::icon_button("🔔"))
                .child(components::icon_button("⚙")),
        )
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let state = NormaAppState::load_current_project();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(gpui::Bounds::new(
                point(px(80.), px(80.)),
                size(px(1440.), px(1024.)),
            ))),
            ..WindowOptions::default()
        };
        cx.open_window(options, |_, cx| cx.new(|_| AppShell::new(state)))
            .expect("failed to open Norma window");
    });
}
```

- [ ] **Step 5: Update binary entrypoint**

Replace `src/main.rs` with:

```rust
fn main() {
    norma::ui::shell::run();
}
```

- [ ] **Step 6: Verify compilation**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS. If GPUI method names differ, inspect local examples under `~/.cargo/registry/src/*/gpui-0.2.2/examples/` and adjust only the UI builder calls, preserving module boundaries and visual contract.

- [ ] **Step 7: Commit**

```bash
git add src/lib.rs src/main.rs src/ui
git commit -m "feat: add GPUI app shell"
```

---

### Task 8: Implement Left Sidebar UI

**Files:**
- Modify: `src/ui/mod.rs`
- Create: `src/ui/sidebar.rs`
- Modify: `src/ui/shell.rs`

- [ ] **Step 1: Export sidebar module**

Update `src/ui/mod.rs`:

```rust
pub mod components;
pub mod shell;
pub mod sidebar;
pub mod theme;
```

- [ ] **Step 2: Create sidebar view helpers**

Create `src/ui/sidebar.rs`:

```rust
use gpui::{AnyElement, ParentElement, Styled, div, prelude::*, px};

use crate::app_state::NormaAppState;
use crate::git::ChangeKind;
use crate::ui::{components, theme};
use crate::workspace::FileKind;

pub fn render_sidebar(state: &NormaAppState) -> AnyElement {
    div()
        .size_full()
        .bg(theme::surface())
        .p_4()
        .flex()
        .flex_col()
        .gap_5()
        .child(project_card(state))
        .child(thread_list(state))
        .child(file_tree(state))
        .child(git_card(state))
        .into_any_element()
}

fn project_card(state: &NormaAppState) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .p_3()
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(state.project_name()))
        .child(components::label(state.project_path_label()))
        .into_any_element()
}

fn thread_list(state: &NormaAppState) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("线程"))
        .child(components::label("今天"))
        .child(
            div()
                .rounded(px(8.))
                .bg(theme::surface_tint())
                .px_3()
                .py_2()
                .flex()
                .justify_between()
                .child(state.session.thread.title.clone())
                .child(components::label(state.session.thread.updated_label.clone())),
        )
        .child(components::label("昨天"))
        .child(components::label("搭建 GPUI 窗口框架"))
        .child(components::label("接入配置管理模块"))
        .into_any_element()
}

fn file_tree(state: &NormaAppState) -> AnyElement {
    let rows = state.files.iter().take(18).map(|node| {
        let indent = px((node.depth as f32) * 14.);
        let icon = match node.kind {
            FileKind::Directory => "▸",
            FileKind::File => "◇",
        };
        div()
            .pl(indent)
            .h(px(24.))
            .flex()
            .items_center()
            .gap_2()
            .text_size(px(13.))
            .child(icon)
            .child(node.name.clone())
    });

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("项目文件"))
        .children(rows)
        .into_any_element()
}

fn git_card(state: &NormaAppState) -> AnyElement {
    let branch = state.git.branch.clone().unwrap_or_else(|| "非 Git 仓库".to_string());
    div()
        .mt_auto()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("Git 状态"))
        .child(components::label(branch))
        .child(
            div()
                .flex()
                .gap_3()
                .child(metric("已修改", state.git.modified, theme::red()))
                .child(metric("已添加", state.git.added + state.git.untracked, theme::green()))
                .child(metric("已删除", state.git.deleted, theme::muted())),
        )
        .child(components::label(format!(
            "↑ {} ahead / ↓ {} behind",
            state.git.ahead, state.git.behind
        )))
        .into_any_element()
}

fn metric(label: &str, value: usize, color: gpui::Hsla) -> AnyElement {
    div()
        .flex()
        .gap_1()
        .text_size(px(12.))
        .child(div().text_color(color).child(value.to_string()))
        .child(components::label(label))
        .into_any_element()
}
```

- [ ] **Step 3: Replace temporary shell sidebar content**

In `src/ui/shell.rs`, add:

```rust
use crate::ui::sidebar;
```

Replace the left column child with:

```rust
div()
    .w(theme::SIDEBAR_WIDTH)
    .h_full()
    .border_r_1()
    .border_color(theme::border())
    .child(sidebar::render_sidebar(&self.state))
```

- [ ] **Step 4: Verify**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 5: Manual run**

Run:

```bash
cargo run
```

Expected: window opens with left sidebar containing project card, thread rows, file tree, and Git card. Close the window before continuing.

- [ ] **Step 6: Commit**

```bash
git add src/ui/mod.rs src/ui/sidebar.rs src/ui/shell.rs
git commit -m "feat: add sidebar workbench UI"
```

---

### Task 9: Implement Center Execution Stream UI

**Files:**
- Modify: `src/ui/mod.rs`
- Create: `src/ui/execution.rs`
- Modify: `src/ui/shell.rs`

- [ ] **Step 1: Export execution module**

Ensure `src/ui/mod.rs` includes:

```rust
pub mod components;
pub mod execution;
pub mod shell;
pub mod sidebar;
pub mod theme;
```

- [ ] **Step 2: Create execution stream renderer**

Create `src/ui/execution.rs`:

```rust
use gpui::{AnyElement, ParentElement, Styled, div, prelude::*, px};

use crate::session::{ExecutionStep, SessionEvent, SessionState, StepStatus};
use crate::ui::{components, theme};

pub fn render_execution(session: &SessionState) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_4()
        .child(task_header(session))
        .child(task_summary(session))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .border_l_1()
                .border_color(theme::border())
                .pl_4()
                .children(session.events.iter().filter_map(render_event)),
        )
        .child(composer())
        .into_any_element()
}

fn task_header(session: &SessionState) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(18.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(session.thread.title.clone())
                .child(" ✎"),
        )
        .child(components::pill("继续任务", false))
        .into_any_element()
}

fn task_summary(session: &SessionState) -> AnyElement {
    let mut goal = "完善 Norma 的整体设计、架构、模块、数据流、UI、Git 交互".to_string();
    let mut constraints = "不内置代码编辑器，专注项目上下文、执行流、变更审查与回滚".to_string();
    for event in &session.events {
        if let SessionEvent::AgentPlan { goal: event_goal, constraints: event_constraints } = event {
            goal = event_goal.clone();
            constraints = event_constraints.join("，");
        }
    }
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(gpui::hsla(218. / 360., 0.8, 0.88, 1.))
        .bg(theme::surface_tint())
        .p_4()
        .flex()
        .flex_col()
        .gap_2()
        .child(row("目标", goal))
        .child(row("约束", constraints))
        .child(row("状态", "进行中"))
        .into_any_element()
}

fn render_event(event: &SessionEvent) -> Option<AnyElement> {
    match event {
        SessionEvent::StepUpdated(step) => Some(step_card(step)),
        SessionEvent::FinalResponse { content } => Some(message_card("完成", content, theme::green())),
        SessionEvent::Error { message } => Some(message_card("需要确认", message, theme::red())),
        SessionEvent::UserTask { .. } | SessionEvent::AgentPlan { .. } | SessionEvent::ChangeSummary { .. } => None,
    }
}

fn step_card(step: &ExecutionStep) -> AnyElement {
    let color = match step.status {
        StepStatus::Completed => theme::green(),
        StepStatus::Running => theme::blue(),
        StepStatus::Waiting => theme::muted(),
        StepStatus::Failed => theme::red(),
    };
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(if step.status == StepStatus::Running { theme::blue() } else { theme::border() })
        .bg(theme::surface())
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(div().text_color(color).child(status_icon(step.status)))
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(step.title.clone())),
                )
                .child(components::label(step.duration_label.clone().unwrap_or_else(|| "等待中".to_string()))),
        )
        .child(components::label(step.description.clone()))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(step.checklist.iter().map(|item| {
                    div()
                        .flex()
                        .gap_2()
                        .text_size(px(13.))
                        .child(status_icon(item.status))
                        .child(item.label.clone())
                })),
        )
        .into_any_element()
}

fn message_card(title: &str, content: &str, color: gpui::Hsla) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(color)
        .bg(theme::surface())
        .p_4()
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title.to_string()))
        .child(components::label(content.to_string()))
        .into_any_element()
}

fn row(label: &str, value: impl Into<String>) -> AnyElement {
    div()
        .flex()
        .gap_2()
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(format!("{label}:")))
        .child(value.into())
        .into_any_element()
}

fn status_icon(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Completed => "✓",
        StepStatus::Running => "◉",
        StepStatus::Waiting => "○",
        StepStatus::Failed => "!",
    }
}

fn composer() -> AnyElement {
    div()
        .mt_auto()
        .rounded(px(12.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(components::label("描述你的下一步需求..."))
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(components::pill("添加上下文", false))
                        .child(components::pill("使用工具", false)),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(components::pill("自动执行", false))
                        .child(components::icon_button("↵")),
                ),
        )
        .into_any_element()
}
```

- [ ] **Step 3: Replace temporary center content**

In `src/ui/shell.rs`, add:

```rust
use crate::ui::execution;
```

Replace the center column child with:

```rust
div()
    .flex_1()
    .h_full()
    .p_6()
    .child(execution::render_execution(&self.state.session))
```

- [ ] **Step 4: Verify**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 5: Manual visual check**

Run:

```bash
cargo run
```

Expected: center stream shows task header, goal/constraint/status block, completed cards, active card with checklist, pending card, and composer. Close the window before continuing.

- [ ] **Step 6: Commit**

```bash
git add src/ui/mod.rs src/ui/execution.rs src/ui/shell.rs
git commit -m "feat: add execution stream UI"
```

---

### Task 10: Implement Right Dynamic Inspector UI

**Files:**
- Modify: `src/ui/mod.rs`
- Create: `src/ui/inspector.rs`
- Modify: `src/ui/shell.rs`

- [ ] **Step 1: Export inspector module**

Ensure `src/ui/mod.rs` includes:

```rust
pub mod components;
pub mod execution;
pub mod inspector;
pub mod shell;
pub mod sidebar;
pub mod theme;
```

- [ ] **Step 2: Create inspector renderer**

Create `src/ui/inspector.rs`:

```rust
use gpui::{AnyElement, ParentElement, Styled, div, prelude::*, px};

use crate::app_state::NormaAppState;
use crate::session::InspectorTab;
use crate::ui::{components, theme};

pub fn render_inspector(state: &NormaAppState) -> AnyElement {
    let review_files = &state.session.changed_files;
    let added: usize = review_files.iter().map(|file| file.added_lines).sum();
    let deleted: usize = review_files.iter().map(|file| file.deleted_lines).sum();
    let changed_count = review_files.len();

    div()
        .size_full()
        .bg(theme::surface())
        .flex()
        .flex_col()
        .child(tabs(state.session.active_tab))
        .child(
            div()
                .p_5()
                .flex()
                .flex_col()
                .gap_5()
                .child(components::section_title("变更概览"))
                .child(
                    div()
                        .grid()
                        .grid_cols(4)
                        .gap_2()
                        .child(metric_tile(changed_count.to_string(), "变更文件", theme::text()))
                        .child(metric_tile(format!("+{added}"), "新增行", theme::green()))
                        .child(metric_tile(format!("-{deleted}"), "删除行", theme::red()))
                        .child(metric_tile("92%", "信心度", theme::blue())),
                )
                .child(safety_row())
                .child(changed_files(state))
                .child(file_preview(state))
                .child(git_operations()),
        )
        .into_any_element()
}

fn tabs(active: InspectorTab) -> AnyElement {
    let names = [
        ("检查器", InspectorTab::Inspector),
        ("上下文", InspectorTab::Context),
        ("输出", InspectorTab::Output),
        ("设置", InspectorTab::Settings),
    ];
    div()
        .h(px(50.))
        .border_b_1()
        .border_color(theme::border())
        .flex()
        .items_end()
        .gap_6()
        .px_5()
        .children(names.into_iter().map(|(name, mode)| {
            let is_active = active == mode;
            div()
                .pb_3()
                .border_b_2()
                .border_color(if is_active { theme::blue() } else { gpui::hsla(0., 0., 0., 0.) })
                .text_color(if is_active { theme::text() } else { theme::muted() })
                .font_weight(if is_active { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                .child(name)
        }))
        .into_any_element()
}

fn metric_tile(value: String, label: &str, color: gpui::Hsla) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_3()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(div().text_color(color).font_weight(gpui::FontWeight::BOLD).text_size(px(18.)).child(value))
        .child(components::label(label))
        .into_any_element()
}

fn safety_row() -> AnyElement {
    div()
        .rounded(px(9.))
        .bg(gpui::hsla(145. / 360., 0.42, 0.95, 1.))
        .px_3()
        .py_2()
        .flex()
        .justify_between()
        .child(div().text_color(theme::green()).child("安全检查  通过"))
        .child(components::label("无高风险操作"))
        .into_any_element()
}

fn changed_files(state: &NormaAppState) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .justify_between()
                .child(components::section_title(format!("变更文件 ({})", state.session.changed_files.len())))
                .child(components::pill("全部", false)),
        )
        .children(state.session.changed_files.iter().take(8).map(|file| {
            div()
                .rounded(px(7.))
                .px_3()
                .py_2()
                .bg(if Some(&file.path) == state.session.selected_change.as_ref().map(|change| &change.path) {
                    theme::surface_tint()
                } else {
                    theme::surface()
                })
                .flex()
                .justify_between()
                .child(file.path.display().to_string())
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(div().text_color(theme::green()).child(format!("+{}", file.added_lines)))
                        .child(div().text_color(theme::red()).child(format!("-{}", file.deleted_lines))),
                )
        }))
        .into_any_element()
}

fn file_preview(state: &NormaAppState) -> AnyElement {
    let Some(change) = &state.session.selected_change else {
        return div().child(components::label("暂无文件变更预览")).into_any_element();
    };

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("文件变更预览"))
        .child(
            div()
                .rounded(px(10.))
                .border_1()
                .border_color(theme::border())
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(change.path.display().to_string()))
                        .child(components::pill("预览对比", false)),
                )
                .children(change.hunks.iter().map(|hunk| {
                    div()
                        .border_t_1()
                        .border_color(theme::border())
                        .pt_2()
                        .flex()
                        .justify_between()
                        .child(format!("Hunk {}  {}", hunk.index, hunk.line_range))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(div().text_color(theme::green()).child(format!("+{}", hunk.added_lines)))
                                .child(div().text_color(theme::red()).child(format!("-{}", hunk.deleted_lines))),
                        )
                })),
        )
        .into_any_element()
}

fn git_operations() -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("Git 操作"))
        .child(disabled_action("撤销本次 Agent 变更", "V1 仅展示入口，不执行破坏性操作"))
        .child(disabled_action("丢弃所选变更", "V1 禁用，避免误删手动修改"))
        .child(disabled_action("在外部编辑器中打开", "后续接入系统打开行为"))
        .into_any_element()
}

fn disabled_action(title: &str, description: &str) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .p_3()
        .opacity(0.65)
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title.to_string()))
        .child(components::label(description.to_string()))
        .into_any_element()
}
```

- [ ] **Step 3: Replace temporary inspector content**

In `src/ui/shell.rs`, add:

```rust
use crate::ui::inspector;
```

Replace the right column child with:

```rust
div()
    .w(theme::INSPECTOR_WIDTH)
    .h_full()
    .border_l_1()
    .border_color(theme::border())
    .child(inspector::render_inspector(&self.state))
```

- [ ] **Step 4: Verify no destructive Git operation or stale inspector data source is introduced**

Run:

```bash
rg -n 'reset|restore|checkout|clean|apply|commit|push' src
rg -n 'InspectorMode|inspector_mode|state\.git\.files|state\.project\.|\.hidden\(false\)' src
```

Expected: first command shows no command execution using destructive words. UI labels may contain non-command descriptions only if they are disabled or preview-only. Second command shows no matches.

- [ ] **Step 5: Verify compilation and tests**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 6: Manual visual check**

Run:

```bash
cargo run
```

Expected: right inspector shows tabs, metric tiles, safety row, changed file rows, selected hunk preview, and disabled Git operation rows. Close the window before continuing.

- [ ] **Step 7: Commit**

```bash
git add src/ui/mod.rs src/ui/inspector.rs src/ui/shell.rs
git commit -m "feat: add dynamic inspector UI"
```

---

### Task 11: Add Visual Verification Checklist

**Files:**
- Create: `tests/visual_contract.md`

- [ ] **Step 1: Create manual verification checklist**

Create `tests/visual_contract.md`:

```markdown
# Norma V1 Visual Contract Checklist

Reference: `docs/superpowers/specs/assets/norma-review-first-codex-workbench.png`

Run:

```bash
cargo run
```

Verify at a desktop window near `1440x1024`:

- [ ] Top toolbar is about 56px tall.
- [ ] Left sidebar is about 320px wide.
- [ ] Right inspector is about 400-420px wide.
- [ ] App uses a light native desktop surface.
- [ ] Top toolbar includes Norma brand, back/forward controls, model selector, local runtime, safety level, run button, notification, and settings controls.
- [ ] Top toolbar does not include a user avatar, account menu, profile control, or sign-in state.
- [ ] No-project state keeps the three-column shell, shows an open-project affordance, and keeps the inspector inactive.
- [ ] Project-open state uses a selected local project root; current working directory loading is documented as a development fallback only.
- [ ] Left sidebar includes project card, grouped thread list, file tree, and Git status card.
- [ ] Center stream includes task title with edit affordance, goal/constraint/status block, timeline rail, completed step cards, active step card, pending step card, and composer with send control.
- [ ] Right inspector includes tabs, change metrics, safety row, changed files, hunk preview, and disabled Git operation rows.
- [ ] Right inspector metrics, changed file list, and selected-file preview come from the same session `ChangeSummary`.
- [ ] No embedded code editor or large code pane is visible.
- [ ] Compare/revert/undo/discard operations are disabled, preview-only, or marked mock-safe.
- [ ] Text is not clipped or overlapping.
- [ ] The screen reads as close to the reference image, not as a generic dashboard.
```

- [ ] **Step 2: Verify markdown renders as plain text**

Run:

```bash
sed -n '1,200p' tests/visual_contract.md
```

Expected: checklist is readable and references the visual target.

- [ ] **Step 3: Run final automated checks**

Run:

```bash
cargo fmt --check
cargo check
cargo test
```

Expected: all PASS.

- [ ] **Step 4: Commit**

```bash
git add tests/visual_contract.md
git commit -m "test: add visual verification checklist"
```

---

### Task 12: Final V1 Plan Verification

**Files:**
- Review all touched files.

- [ ] **Step 1: Check final git state**

Run:

```bash
git status --short
```

Expected: no unstaged or uncommitted changes.

- [ ] **Step 2: Run complete validation**

Run:

```bash
cargo fmt --check
cargo check
cargo test
rg -n 'reset|restore|checkout|clean|apply|commit|push' src
```

Expected:

- formatting passes
- type checking passes
- tests pass
- no destructive Git command execution appears in `src`
- no stale inspector mode names or mixed inspector data sources remain

- [ ] **Step 3: Manual visual verification**

Run:

```bash
cargo run
```

Use `tests/visual_contract.md` and compare against:

```text
docs/superpowers/specs/assets/norma-review-first-codex-workbench.png
```

Expected: every checklist item passes or has a documented reason tied to GPUI constraints. Any documented exception must not remove no-project state, session-derived review data, disabled destructive actions, or the core three-column visual structure.

- [ ] **Step 4: Summarize implementation state**

Add a short note to the final implementation response:

```text
Implemented V1 project workbench shell: real file tree, read-only Git status, mock runtime events, Codex-inspired three-column GPUI UI, and disabled Git action affordances. V1 does not call real models, edit files, or run destructive Git actions.
```

Do not commit this response text to the repository.
