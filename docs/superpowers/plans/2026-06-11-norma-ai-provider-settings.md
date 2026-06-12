# Norma AI Provider Settings Implementation Plan

> Historical note: this plan described the initial preview-only settings window. The later `rig-provider-abstraction` change supersedes the preview-only provider-test/save assumptions by adding save-gated real provider connection testing through the provider abstraction.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a separate Norma application settings window for AI provider configuration without interrupting the active workbench session. This original plan targeted preview-only configuration; later provider abstraction work adds real connection testing and save gating.

**Architecture:** Add a focused `config` module for settings data and validation, then thread `AppConfig` through `NormaAppState` so both the workbench and settings window can render from the same app-level state. Implement the settings UI as its own GPUI root view under `ui::settings`, opened from the top-right toolbar settings icon with a separate `open_window` call. This plan's preview-only test/save assumptions are historical; the provider abstraction change adds real low-cost provider connection tests before saving.

**Tech Stack:** Rust 2024, GPUI `0.2.2`, existing Norma UI helpers and theme module, standard Rust unit tests. No new dependencies are required for this plan.

---

## Scope Check

This plan implements `docs/superpowers/specs/2026-06-11-norma-ai-provider-settings-design.md`.

The scope was one subsystem: an app-level settings window with AI provider configuration. Rig integration and real provider connection tests are handled by the later provider abstraction change; real chat/session model calls, persistent secret storage, MCP, Skills, account management, billing, and right-inspector settings replacement remain outside this original plan.

## Current Repository Facts

- `src/lib.rs` currently exports `agent`, `app_state`, `git`, `session`, `ui`, and `workspace`.
- `src/app_state.rs` owns `NormaAppState` and currently does not include app settings.
- `src/ui/shell.rs` owns `AppShell`, `top_toolbar`, and GPUI window startup.
- `src/ui/components.rs` has simple static helpers such as `pill` and `icon_button`.
- `src/ui/theme.rs` centralizes the light workbench palette.
- Current toolbar icon buttons are visual divs; the settings icon needs a click handler.
- GPUI `div()` supports `.on_click(|event, window, cx| { ... })`.

## Target File Structure

- Modify: `src/lib.rs`  
  Export the new `config` module.

- Create: `src/config.rs`  
  Own settings sections, AI provider protocols, provider config state, validation, sample app config, and API key masking.

- Modify: `src/app_state.rs`  
  Add `config: AppConfig` to `NormaAppState` and preserve it across `no_project`, project-open, and project-open-error states.

- Modify: `src/ui/mod.rs`  
  Export `settings`.

- Create: `src/ui/settings.rs`  
  Own `SettingsWindow`, settings navigation, AI provider pane, provider list, provider editor, and preview-only action rows.

- Modify: `src/ui/components.rs`  
  Add small reusable helpers for interactive icon buttons and form-like rows if needed.

- Modify: `src/ui/shell.rs`  
  Open the separate settings window from the top-right settings icon without changing the active `AppShell` state.

- Create: `tests/settings_visual_contract.md`  
  Manual checklist for the separate settings window and non-interruption behavior.

## Commit Strategy

Commit after each task:

- `feat: add app settings config model`
- `feat: attach config to app state`
- `feat: render settings window shell`
- `feat: render AI provider settings`
- `feat: open settings from toolbar`
- `test: add settings visual checklist`

---

### Task 1: Add App Settings Config Model

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Export the config module**

Edit `src/lib.rs` so it contains:

```rust
pub mod agent;
pub mod app_state;
pub mod config;
pub mod git;
pub mod session;
pub mod ui;
pub mod workspace;
```

- [ ] **Step 2: Write the config model and tests**

Create `src/config.rs` with this complete initial implementation:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    AiProviders,
    Runtime,
    Safety,
    Git,
    Appearance,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 6] = [
        SettingsSection::General,
        SettingsSection::AiProviders,
        SettingsSection::Runtime,
        SettingsSection::Safety,
        SettingsSection::Git,
        SettingsSection::Appearance,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::General => "通用",
            SettingsSection::AiProviders => "AI 提供商",
            SettingsSection::Runtime => "运行环境",
            SettingsSection::Safety => "安全",
            SettingsSection::Git => "Git",
            SettingsSection::Appearance => "外观",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProtocol {
    OpenAi,
    Anthropic,
}

impl ProviderProtocol {
    pub const ALL: [ProviderProtocol; 2] = [ProviderProtocol::OpenAi, ProviderProtocol::Anthropic];

    pub fn label(self) -> &'static str {
        match self {
            ProviderProtocol::OpenAi => "OpenAI",
            ProviderProtocol::Anthropic => "Anthropic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderConfigStatus {
    Complete,
    Incomplete,
    Invalid,
    PreviewUnvalidated,
}

impl ProviderConfigStatus {
    pub fn label(self) -> &'static str {
        match self {
            ProviderConfigStatus::Complete => "配置完整",
            ProviderConfigStatus::Incomplete => "待补全",
            ProviderConfigStatus::Invalid => "配置无效",
            ProviderConfigStatus::PreviewUnvalidated => "待测试",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiProviderConfig {
    pub id: String,
    pub name: String,
    pub protocol: ProviderProtocol,
    pub base_url: String,
    pub api_key_reference: String,
    pub model: String,
    pub status: ProviderConfigStatus,
}

impl AiProviderConfig {
    pub fn masked_api_key(&self) -> String {
        mask_secret(&self.api_key_reference)
    }

    pub fn required_field_errors(&self) -> Vec<&'static str> {
        let mut errors = Vec::new();
        if self.name.trim().is_empty() {
            errors.push("名称不能为空");
        }
        if self.base_url.trim().is_empty() {
            errors.push("Base URL 不能为空");
        }
        if self.api_key_reference.trim().is_empty() {
            errors.push("API Key 不能为空");
        }
        if self.model.trim().is_empty() {
            errors.push("模型不能为空");
        }
        errors
    }

    pub fn is_valid_for_preview(&self) -> bool {
        self.required_field_errors().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub active_settings_section: SettingsSection,
    pub selected_provider_id: Option<String>,
    pub providers: Vec<AiProviderConfig>,
}

impl AppConfig {
    pub fn sample() -> Self {
        Self {
            active_settings_section: SettingsSection::AiProviders,
            selected_provider_id: Some("openai-default".to_string()),
            providers: vec![
                AiProviderConfig {
                    id: "openai-default".to_string(),
                    name: "OpenAI 默认".to_string(),
                    protocol: ProviderProtocol::OpenAi,
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key_reference: "sk-preview-openai-default".to_string(),
                    model: "gpt-4o".to_string(),
                    status: ProviderConfigStatus::Complete,
                },
                AiProviderConfig {
                    id: "claude-proxy".to_string(),
                    name: "Claude 代理".to_string(),
                    protocol: ProviderProtocol::Anthropic,
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key_reference: "sk-preview-claude-proxy".to_string(),
                    model: "claude-3-5-sonnet".to_string(),
                    status: ProviderConfigStatus::PreviewUnvalidated,
                },
            ],
        }
    }

    pub fn selected_provider(&self) -> Option<&AiProviderConfig> {
        let selected_id = self.selected_provider_id.as_deref()?;
        self.providers.iter().find(|provider| provider.id == selected_id)
    }
}

pub fn mask_secret(secret: &str) -> String {
    if secret.trim().is_empty() {
        return String::new();
    }
    let visible_tail: String = secret
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("••••••••••••{visible_tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_sections_match_design_order() {
        let labels: Vec<&str> = SettingsSection::ALL
            .iter()
            .map(|section| section.label())
            .collect();
        assert_eq!(
            labels,
            vec!["通用", "AI 提供商", "运行环境", "安全", "Git", "外观"]
        );
    }

    #[test]
    fn provider_protocols_are_limited_to_openai_and_anthropic() {
        let labels: Vec<&str> = ProviderProtocol::ALL
            .iter()
            .map(|protocol| protocol.label())
            .collect();
        assert_eq!(labels, vec!["OpenAI", "Anthropic"]);
    }

    #[test]
    fn masks_api_key_by_default() {
        assert_eq!(mask_secret("sk-preview-openai-default"), "••••••••••••ault");
    }

    #[test]
    fn validates_required_provider_fields_without_network_calls() {
        let provider = AiProviderConfig {
            id: "empty".to_string(),
            name: "".to_string(),
            protocol: ProviderProtocol::OpenAi,
            base_url: "".to_string(),
            api_key_reference: "".to_string(),
            model: "".to_string(),
            status: ProviderConfigStatus::Incomplete,
        };

        assert_eq!(
            provider.required_field_errors(),
            vec![
                "名称不能为空",
                "Base URL 不能为空",
                "API Key 不能为空",
                "模型不能为空"
            ]
        );
        assert!(!provider.is_valid_for_preview());
    }

    #[test]
    fn sample_config_selects_openai_provider() {
        let config = AppConfig::sample();
        let selected = config.selected_provider().unwrap();
        assert_eq!(selected.name, "OpenAI 默认");
        assert_eq!(selected.protocol, ProviderProtocol::OpenAi);
    }
}
```

- [ ] **Step 3: Run the config tests**

Run:

```bash
cargo test config --lib
```

Expected: all `config` tests pass.

- [ ] **Step 4: Run formatting**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If it fails, run `cargo fmt`, then rerun `cargo fmt --check`.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/config.rs
git commit -m "feat: add app settings config model"
```

---

### Task 2: Attach Config To App State

**Files:**
- Modify: `src/app_state.rs`

- [ ] **Step 1: Update imports and state shape**

Modify the imports and `NormaAppState` definition in `src/app_state.rs`:

```rust
use std::env;

use crate::agent::{AgentRuntime, MockAgentRuntime};
use crate::config::AppConfig;
use crate::git::{GitStatusSummary, read_status};
use crate::session::{SessionState, sample_thread};
use crate::workspace::{FileNode, Project, load_file_tree, open_project, sample_file_tree};

#[derive(Debug, Clone)]
pub enum ProjectSelectionState {
    NoProject,
    ProjectOpen(Project),
    OpenError {
        attempted_path: String,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub struct NormaAppState {
    pub project_state: ProjectSelectionState,
    pub files: Vec<FileNode>,
    pub git: GitStatusSummary,
    pub session: SessionState,
    pub config: AppConfig,
}
```

- [ ] **Step 2: Initialize config in every constructor**

Update each `NormaAppState` constructor to include `config: AppConfig::sample()`:

```rust
pub fn no_project() -> Self {
    Self {
        project_state: ProjectSelectionState::NoProject,
        files: Vec::new(),
        git: GitStatusSummary::unavailable("no project open"),
        session: SessionState::new(sample_thread()),
        config: AppConfig::sample(),
    }
}
```

For the open-error return branch, use:

```rust
return Self {
    project_state: ProjectSelectionState::OpenError {
        attempted_path: root.display().to_string(),
        message: error.to_string(),
    },
    files: sample_file_tree(),
    git: GitStatusSummary::unavailable("project could not be opened"),
    session: SessionState::new(sample_thread()),
    config: AppConfig::sample(),
};
```

For the successful project-open return, use:

```rust
Self {
    project_state: ProjectSelectionState::ProjectOpen(project),
    files,
    git,
    session,
    config: AppConfig::sample(),
}
```

- [ ] **Step 3: Add app state tests for settings**

Add these tests to the existing `#[cfg(test)] mod tests` in `src/app_state.rs`:

```rust
#[test]
fn app_state_includes_preview_provider_config() {
    let state = NormaAppState::no_project();
    assert_eq!(state.config.providers.len(), 2);
    assert_eq!(
        state.config.selected_provider().unwrap().name,
        "OpenAI 默认"
    );
}

#[test]
fn settings_config_is_separate_from_session_events() {
    let state = NormaAppState::no_project();
    assert!(state.session.events.is_empty());
    assert!(!state.config.providers.is_empty());
}
```

- [ ] **Step 4: Run app state tests**

Run:

```bash
cargo test app_state --lib
```

Expected: all `app_state` tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app_state.rs
git commit -m "feat: attach config to app state"
```

---

### Task 3: Render The Separate Settings Window Shell

**Files:**
- Create: `src/ui/settings.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Export the settings UI module**

Edit `src/ui/mod.rs`:

```rust
pub mod components;
pub mod execution;
pub mod inspector;
pub mod settings;
pub mod shell;
pub mod sidebar;
pub mod theme;
```

- [ ] **Step 2: Create the settings window skeleton**

Create `src/ui/settings.rs`:

```rust
use gpui::{AnyElement, Context, IntoElement, ParentElement, Render, Styled, Window, div, prelude::*, px};

use crate::config::{AppConfig, SettingsSection};
use crate::ui::{components, theme};

pub struct SettingsWindow {
    config: AppConfig,
}

impl SettingsWindow {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .flex()
            .flex_col()
            .child(settings_header())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(settings_navigation(self.config.active_settings_section))
                    .child(
                        div()
                            .flex_1()
                            .p_6()
                            .child(settings_placeholder(self.config.active_settings_section)),
                    ),
            )
    }
}

fn settings_header() -> AnyElement {
    div()
        .h(px(56.))
        .px_5()
        .border_b_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(16.))
                .child("设置"),
        )
        .child(components::pill("应用级配置", false))
        .into_any_element()
}

fn settings_navigation(active: SettingsSection) -> AnyElement {
    div()
        .w(px(240.))
        .h_full()
        .bg(theme::surface())
        .border_r_1()
        .border_color(theme::border())
        .p_4()
        .flex()
        .flex_col()
        .gap_1()
        .children(SettingsSection::ALL.into_iter().map(|section| {
            let selected = section == active;
            div()
                .rounded(px(8.))
                .px_3()
                .py_2()
                .bg(if selected {
                    theme::surface_tint()
                } else {
                    theme::surface()
                })
                .text_color(if selected { theme::text() } else { theme::muted() })
                .font_weight(if selected {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .child(section.label())
        }))
        .into_any_element()
}

fn settings_placeholder(section: SettingsSection) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_5()
        .child(components::section_title(section.label()))
        .child(components::label("此配置分区为预览入口。"))
        .into_any_element()
}
```

- [ ] **Step 3: Run check**

Run:

```bash
cargo check
```

Expected: PASS, or the known GPUI/macOS Metal Toolchain failure. If the failure mentions Metal Toolchain, install it with `xcodebuild -downloadComponent MetalToolchain`, then rerun `cargo check`.

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs src/ui/settings.rs
git commit -m "feat: render settings window shell"
```

---

### Task 4: Render AI Provider Settings Content

**Files:**
- Modify: `src/ui/settings.rs`

- [ ] **Step 1: Replace the placeholder branch with the AI provider pane**

Update the content child in `SettingsWindow::render`:

```rust
.child(
    div()
        .flex_1()
        .p_6()
        .child(settings_content(&self.config)),
)
```

Add this function below `settings_navigation`:

```rust
fn settings_content(config: &AppConfig) -> AnyElement {
    match config.active_settings_section {
        SettingsSection::AiProviders => ai_provider_pane(config),
        section => settings_placeholder(section),
    }
}
```

- [ ] **Step 2: Add the AI provider pane**

Add these functions to `src/ui/settings.rs`:

```rust
fn ai_provider_pane(config: &AppConfig) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .gap_5()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(20.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child("AI 提供商"),
                        )
                        .child(components::label(
                            "为后续模型调用配置不同协议格式的提供商。",
                        )),
                )
                .child(components::pill("+ 新增提供商", true)),
        )
        .child(
            div()
                .flex()
                .gap_5()
                .child(provider_list(config))
                .child(provider_editor(config)),
        )
        .child(
            div()
                .rounded(px(9.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface_tint())
                .p_3()
                .child(components::label(
                    "模型调用将在后续通过 Rig + 自研 Provider 抽象层接入。当前仅保存配置预览。",
                )),
        )
        .into_any_element()
}

fn provider_list(config: &AppConfig) -> AnyElement {
    div()
        .w(px(360.))
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(theme::border())
                .child(components::section_title("提供商")),
        )
        .children(config.providers.iter().map(|provider| {
            let selected = config.selected_provider_id.as_deref() == Some(provider.id.as_str());
            div()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(theme::border())
                .bg(if selected {
                    theme::surface_tint()
                } else {
                    theme::surface()
                })
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(provider.name.clone()),
                        )
                        .child(components::pill(provider.protocol.label(), false)),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(components::label(provider.model.clone()))
                        .child(status_label(provider.status)),
                )
        }))
        .into_any_element()
}

fn status_label(status: crate::config::ProviderConfigStatus) -> AnyElement {
    let color = match status {
        crate::config::ProviderConfigStatus::Complete => theme::green(),
        crate::config::ProviderConfigStatus::Invalid => theme::red(),
        crate::config::ProviderConfigStatus::Incomplete
        | crate::config::ProviderConfigStatus::PreviewUnvalidated => theme::muted(),
    };
    div()
        .text_size(px(13.))
        .text_color(color)
        .child(status.label())
        .into_any_element()
}
```

- [ ] **Step 3: Add the selected provider editor**

Add these functions to `src/ui/settings.rs`:

```rust
fn provider_editor(config: &AppConfig) -> AnyElement {
    let Some(provider) = config.selected_provider() else {
        return div()
            .flex_1()
            .rounded(px(10.))
            .border_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .p_5()
            .child(components::label("选择一个提供商进行配置。"))
            .into_any_element();
    };

    div()
        .flex_1()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_5()
        .flex()
        .flex_col()
        .gap_4()
        .child(components::section_title("提供商配置"))
        .child(form_row("名称", provider.name.clone()))
        .child(protocol_segment(provider.protocol))
        .child(form_row("Base URL", provider.base_url.clone()))
        .child(form_row("API Key", provider.masked_api_key()))
        .child(form_row("模型", provider.model.clone()))
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(components::pill("保存配置", true))
                        .child(components::pill("测试连接（预览）", false)),
                )
                .child(components::label("预览按钮不会发起网络请求")),
        )
        .into_any_element()
}

fn form_row(label: &str, value: impl Into<String>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::label(label))
        .child(
            div()
                .h(px(38.))
                .rounded(px(8.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface())
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .child(value.into()),
        )
        .into_any_element()
}

fn protocol_segment(active: crate::config::ProviderProtocol) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::label("协议类型"))
        .child(
            div()
                .rounded(px(8.))
                .border_1()
                .border_color(theme::border())
                .bg(theme::app_bg())
                .p_1()
                .flex()
                .gap_1()
                .children(crate::config::ProviderProtocol::ALL.into_iter().map(|protocol| {
                    let selected = protocol == active;
                    div()
                        .flex_1()
                        .rounded(px(6.))
                        .px_3()
                        .py_2()
                        .bg(if selected {
                            theme::surface()
                        } else {
                            theme::app_bg()
                        })
                        .text_color(if selected { theme::text() } else { theme::muted() })
                        .font_weight(if selected {
                            gpui::FontWeight::SEMIBOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .child(protocol.label())
                })),
        )
        .into_any_element()
}
```

- [ ] **Step 4: Remove unused imports or functions**

Run:

```bash
cargo check
```

Expected: PASS. If Rust reports unused imports in `src/ui/settings.rs`, remove only those unused imports and rerun `cargo check`.

- [ ] **Step 5: Commit**

```bash
git add src/ui/settings.rs
git commit -m "feat: render AI provider settings"
```

---

### Task 5: Open Settings From The Toolbar

**Files:**
- Modify: `src/ui/shell.rs`

- [ ] **Step 1: Import the settings window type**

Update the `src/ui/shell.rs` UI imports:

```rust
use crate::ui::{components, execution, inspector, settings::SettingsWindow, sidebar, theme};
```

- [ ] **Step 2: Pass app state into the toolbar**

Change the toolbar call in `AppShell::render`:

```rust
.child(top_toolbar(&self.state))
```

Change the function signature:

```rust
fn top_toolbar(state: &NormaAppState) -> impl IntoElement {
```

- [ ] **Step 3: Add an interactive settings button**

Replace the final toolbar settings icon child:

```rust
.child(settings_button(state.config.clone()))
```

Add this function below `top_toolbar`:

```rust
fn settings_button(config: crate::config::AppConfig) -> impl IntoElement {
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
        .child("⚙")
        .on_click(move |_, _, cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(180.), px(120.)),
                    size(px(960.), px(720.)),
                ))),
                ..WindowOptions::default()
            };
            let config = config.clone();
            cx.open_window(options, |_, cx| cx.new(|_| SettingsWindow::new(config)))
                .expect("failed to open Norma settings window");
        })
}
```

Keep the notification icon as a non-interactive visual button:

```rust
.child(components::icon_button("🔔"))
```

- [ ] **Step 4: Run shell build verification**

Run:

```bash
cargo check
```

Expected: PASS. If GPUI requires an explicit trait import for `.on_click`, add `InteractiveElement` to the existing GPUI import list in `src/ui/shell.rs` and rerun.

- [ ] **Step 5: Verify session state is not mutated by settings setup**

Run:

```bash
cargo test app_state config --lib
```

Expected: all app state and config tests pass. This verifies the settings config is separate from session events at the state level.

- [ ] **Step 6: Commit**

```bash
git add src/ui/shell.rs
git commit -m "feat: open settings from toolbar"
```

---

### Task 6: Add Manual Visual And Behavior Checklist

**Files:**
- Create: `tests/settings_visual_contract.md`

- [ ] **Step 1: Add the checklist**

Create `tests/settings_visual_contract.md`:

```markdown
# Settings Visual Contract

Use this checklist when verifying `docs/superpowers/specs/2026-06-11-norma-ai-provider-settings-design.md`.

## Workbench Entry

- [ ] The main workbench still opens at about 1440x1024.
- [ ] The top-right settings icon is compact and visually aligned with the notification/run controls.
- [ ] Clicking the settings icon opens a separate `设置` window.
- [ ] Opening settings does not replace the workbench content.
- [ ] Opening settings does not clear the center composer.
- [ ] Opening settings does not switch the selected thread.
- [ ] Opening settings does not add a session event to the execution stream.

## Settings Window

- [ ] The settings window is light, compact, and consistent with the workbench visual language.
- [ ] The left navigation includes `通用`, `AI 提供商`, `运行环境`, `安全`, `Git`, and `外观`.
- [ ] `AI 提供商` is selected with a soft blue tint.
- [ ] The right pane title is `AI 提供商`.
- [ ] The helper text says `为后续模型调用配置不同协议格式的提供商。`.
- [ ] Provider rows are grouped with separators rather than separate cards.
- [ ] The selected provider row is lightly tinted.

## Provider Editor

- [ ] The editor shows `名称`, `协议类型`, `Base URL`, `API Key`, and `模型`.
- [ ] `协议类型` has exactly `OpenAI` and `Anthropic`.
- [ ] `API Key` is masked by default.
- [ ] `保存配置` is the primary action.
- [ ] `测试连接（预览）` is disabled or clearly preview-only.
- [ ] The pane includes `模型调用将在后续通过 Rig + 自研 Provider 抽象层接入。当前仅保存配置预览。`.
- [ ] No real API key, account UI, avatar, billing, MCP, Skills, automation, or tool-calling controls are visible.
```

- [ ] **Step 2: Run final verification commands**

Run:

```bash
cargo fmt --check
cargo test
cargo check
cargo clippy --all-targets -- -D warnings
```

Expected:

- `cargo fmt --check`: PASS
- `cargo test`: PASS
- `cargo check`: PASS
- `cargo clippy --all-targets -- -D warnings`: PASS when Clippy is installed

If `cargo clippy` is unavailable because the Rust component is not installed, record the exact component error in the handoff.

- [ ] **Step 3: Run the app for manual verification**

Run:

```bash
cargo run
```

Expected:

- the main workbench opens
- clicking the top-right settings icon opens a separate `设置` window
- the settings window shows the AI provider pane
- no model calls or network provider tests are performed

- [ ] **Step 4: Commit**

```bash
git add tests/settings_visual_contract.md
git commit -m "test: add settings visual checklist"
```

---

## Self-Review Notes

- Spec coverage: The plan covers the separate settings entry point, non-interruption behavior, left navigation, AI provider list, selected provider editor, OpenAI/Anthropic protocol limit, masked API keys, preview-only test action, and Rig plus custom Provider abstraction note.
- Placeholder scan: The plan contains no undefined implementation gaps; each code-changing task includes the exact code shape to add or modify.
- Type consistency: `SettingsSection`, `ProviderProtocol`, `ProviderConfigStatus`, `AiProviderConfig`, and `AppConfig` are introduced in Task 1 and reused consistently in later tasks.
