# Text Input System Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the reviewed text input behavior gaps: composer submit, provider persistence, TextArea clipboard, and minimum visible caret/selection states.

**Architecture:** Keep the existing `src/ui/input/` structure. Add pure model helpers for clipboard and display segments, reuse those helpers in GPUI field rendering, wire `ComposerInput` to own submit commands, and make settings save build a persisted config from the current provider candidate.

**Tech Stack:** Rust 2024, GPUI 0.2.2, existing Norma config and UI modules, Rust unit tests, manual GPUI verification.

---

## Scope Check

This plan repairs the existing input-system implementation. It does not add mouse character hit-testing, drag selection, autocomplete, slash commands, `@` references, or caret-following scroll.

## File Structure

- Modify `src/ui/input/model.rs`: add pure clipboard helpers and display segment support.
- Modify `src/ui/input/field.rs`: use model clipboard helpers for `TextField`, `SecureTextField`, and `TextArea`; render caret and selection segments.
- Modify `src/ui/input/composer.rs`: add composer command handling, submit callback invocation, successful-submit clearing, send button wiring, and focused key handling.
- Modify `src/ui/settings.rs`: save current provider candidate into `NormaConfig` instead of writing stale persisted config.

---

### Task 1: Add TextBuffer Clipboard And Display Segment Helpers

**Files:**
- Modify: `src/ui/input/model.rs`

- [ ] **Step 1: Write failing model tests**

Add these tests to `#[cfg(test)] mod tests` in `src/ui/input/model.rs`:

```rust
#[test]
fn cut_selection_returns_text_and_removes_selection() {
    let mut buffer = TextBuffer::new(InputMode::MultiLine, "hello\nworld");
    buffer.set_selection(Selection::range(6, 11));

    let cut = buffer.cut_selection();

    assert_eq!(cut, Some("world".to_string()));
    assert_eq!(buffer.text(), "hello\n");
    assert_eq!(buffer.selection(), Selection::caret(6));

    assert!(buffer.undo());
    assert_eq!(buffer.text(), "hello\nworld");
}

#[test]
fn paste_text_inserts_multiline_text_at_caret() {
    let mut buffer = TextBuffer::new(InputMode::MultiLine, "hello");

    buffer.move_to_end(false);
    buffer.paste_text("\nworld").unwrap();

    assert_eq!(buffer.text(), "hello\nworld");
    assert_eq!(buffer.selection(), Selection::caret(11));
}

#[test]
fn display_segments_show_caret_in_middle() {
    let mut buffer = TextBuffer::new(InputMode::SingleLine, "hello");
    buffer.move_to_start(false);
    buffer.move_right(false);
    buffer.move_right(false);

    assert_eq!(
        buffer.display_segments(DisplayMode::Plain, true),
        vec![
            DisplaySegment::Text("he".to_string()),
            DisplaySegment::Caret,
            DisplaySegment::Text("llo".to_string()),
        ]
    );
}

#[test]
fn display_segments_show_reversed_selection() {
    let mut buffer = TextBuffer::new(InputMode::SingleLine, "hello");
    buffer.set_selection(Selection::range(4, 1));

    assert_eq!(
        buffer.display_segments(DisplayMode::Plain, true),
        vec![
            DisplaySegment::Text("h".to_string()),
            DisplaySegment::Selection("ell".to_string()),
            DisplaySegment::Text("o".to_string()),
        ]
    );
}

#[test]
fn secure_display_segments_do_not_expose_raw_secret() {
    let mut buffer = TextBuffer::new(InputMode::SingleLine, "sk-test-secret");
    buffer.set_selection(Selection::range(0, 2));

    let segments = buffer.display_segments(DisplayMode::Secure, true);

    assert_eq!(
        segments,
        vec![
            DisplaySegment::Selection("••".to_string()),
            DisplaySegment::Text("••••••••cret".to_string()),
        ]
    );
}

#[test]
fn display_segments_respect_chinese_character_boundaries() {
    let mut buffer = TextBuffer::new(InputMode::SingleLine, "你好world");
    buffer.set_selection(Selection::range("你".len(), "你好".len()));

    assert_eq!(
        buffer.display_segments(DisplayMode::Plain, true),
        vec![
            DisplaySegment::Text("你".to_string()),
            DisplaySegment::Selection("好".to_string()),
            DisplaySegment::Text("world".to_string()),
        ]
    );
}
```

- [ ] **Step 2: Run failing model tests**

Run:

```bash
cargo test ui::input::model --lib
```

Expected: FAIL because `DisplaySegment`, `cut_selection`, `paste_text`, and `display_segments` are missing.

- [ ] **Step 3: Implement display segment and clipboard helpers**

Add this enum near `DisplayMode` in `src/ui/input/model.rs`:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplaySegment {
    Text(String),
    Selection(String),
    Caret,
}
```

Add these methods to `impl TextBuffer`:

```rust
pub fn cut_selection(&mut self) -> Option<String> {
    if self.selection.is_caret() {
        return None;
    }
    let selected = self.selected_text().to_string();
    self.push_undo();
    self.replace_selection_raw("");
    self.redo_stack.clear();
    Some(selected)
}

pub fn paste_text(&mut self, text: &str) -> Result<EditOutcome, TextEditError> {
    self.insert_text(text)
}

pub fn clear(&mut self) {
    self.push_undo();
    self.text.clear();
    self.selection = Selection::caret(0);
    self.redo_stack.clear();
}

pub fn display_segments(&self, mode: DisplayMode, focused: bool) -> Vec<DisplaySegment> {
    let display_text = self.display_text(mode);
    let selection = self.selection.ordered();
    if !focused {
        return vec![DisplaySegment::Text(display_text)];
    }

    if self.selection.is_caret() {
        let caret = display_offset_for_raw_offset(&self.text, self.selection.head, &display_text);
        let mut segments = Vec::new();
        if caret > 0 {
            segments.push(DisplaySegment::Text(display_text[..caret].to_string()));
        }
        segments.push(DisplaySegment::Caret);
        if caret < display_text.len() {
            segments.push(DisplaySegment::Text(display_text[caret..].to_string()));
        }
        return segments;
    }

    let start = display_offset_for_raw_offset(&self.text, selection.start, &display_text);
    let end = display_offset_for_raw_offset(&self.text, selection.end, &display_text);
    let mut segments = Vec::new();
    if start > 0 {
        segments.push(DisplaySegment::Text(display_text[..start].to_string()));
    }
    segments.push(DisplaySegment::Selection(display_text[start..end].to_string()));
    if end < display_text.len() {
        segments.push(DisplaySegment::Text(display_text[end..].to_string()));
    }
    segments
}
```

Add this helper below `mask_secret`:

```rust
fn display_offset_for_raw_offset(raw: &str, raw_offset: usize, display: &str) -> usize {
    let char_offset = raw[..raw_offset].chars().count();
    display
        .char_indices()
        .nth(char_offset)
        .map(|(idx, _)| idx)
        .unwrap_or(display.len())
}
```

Update `src/ui/input/mod.rs` export:

```rust
pub use model::{
    DisplayMode, DisplaySegment, EditOutcome, InputMode, Selection, TextBuffer, TextEditError,
    TextSnapshot,
};
```

- [ ] **Step 4: Run model tests**

Run:

```bash
cargo test ui::input::model --lib
```

Expected: PASS.

- [ ] **Step 5: Commit model repair**

Run:

```bash
git add src/ui/input/model.rs src/ui/input/mod.rs
git commit -m "fix(ui): add input clipboard and display segments"
```

---

### Task 2: Render Caret And Selection And Repair TextArea Clipboard

**Files:**
- Modify: `src/ui/input/field.rs`

- [ ] **Step 1: Refactor field rendering to segment helpers**

Modify imports in `src/ui/input/field.rs`:

```rust
use crate::ui::{
    input::{
        DisplayMode, DisplaySegment, InputCommand, InputMode, KeyBindingContext, TextBuffer,
        key_to_command,
    },
    theme,
};
```

Add helper functions near `render_input_shell`:

```rust
fn render_segments(segments: Vec<DisplaySegment>, placeholder: String, empty: bool) -> gpui::Div {
    let mut row = div().flex().items_center();
    if empty {
        return row
            .text_color(theme::muted())
            .child(placeholder);
    }
    for segment in segments {
        row = match segment {
            DisplaySegment::Text(text) => row.child(div().child(text)),
            DisplaySegment::Selection(text) => row.child(
                div()
                    .rounded(px(3.))
                    .bg(theme::surface_tint())
                    .text_color(theme::text())
                    .child(text),
            ),
            DisplaySegment::Caret => row.child(
                div()
                    .w(px(1.))
                    .h(px(18.))
                    .bg(theme::blue()),
            ),
        };
    }
    row
}
```

Change `render_input_shell` signature:

```rust
fn render_input_shell(
    focused: bool,
    disabled: bool,
    has_error: bool,
    empty: bool,
    segments: Vec<DisplaySegment>,
    placeholder: String,
) -> gpui::Div
```

Replace its last line:

```rust
.child(render_segments(segments, placeholder, empty))
```

Update `TextField::render`:

```rust
let focused = self.focus_handle.is_focused(window);
render_input_shell(
    focused,
    self.disabled,
    self.error_text.is_some(),
    self.buffer.text().is_empty(),
    self.buffer.display_segments(DisplayMode::Plain, focused),
    self.placeholder.clone(),
)
```

Update `SecureTextField::render` the same way, using `display_mode`.

- [ ] **Step 2: Add shared clipboard command helper**

Add this method to `impl TextField`:

```rust
fn apply_editing_command(&mut self, command: InputCommand, cx: &mut Context<Self>) -> bool {
    match command {
        InputCommand::Copy => {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                self.buffer.selected_text().to_string(),
            ));
            false
        }
        InputCommand::Cut if !self.read_only => {
            if let Some(text) = self.buffer.cut_selection() {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                self.emit_change();
                true
            } else {
                false
            }
        }
        InputCommand::Paste if !self.read_only => {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                && self
                    .buffer
                    .paste_text(&text)
                    .map(|outcome| outcome.changed)
                    .unwrap_or(false)
            {
                self.emit_change();
                true
            } else {
                false
            }
        }
        command if !self.read_only => {
            if self
                .buffer
                .apply_command(command)
                .map(|outcome| outcome.changed)
                .unwrap_or(false)
            {
                self.emit_change();
                true
            } else {
                false
            }
        }
        _ => false,
    }
}
```

Then replace the editing branches in `TextField::apply_command` with:

```rust
command => {
    self.apply_editing_command(command, cx);
}
```

Keep `Submit` and `Blur` branches before this branch.

- [ ] **Step 3: Apply equivalent clipboard behavior to SecureTextField**

Inside the `SecureTextField` key listener, replace the `Copy`, `Cut`, and `Paste` branches with logic using `this.field.buffer.cut_selection()` and `this.field.buffer.paste_text(&text)`. Keep the final branch calling `this.field.buffer.apply_command(command)`.

Use this exact replacement for clipboard branches:

```rust
InputCommand::Copy => cx.write_to_clipboard(gpui::ClipboardItem::new_string(
    this.field.buffer.selected_text().to_string(),
)),
InputCommand::Cut if !this.field.read_only => {
    if let Some(text) = this.field.buffer.cut_selection() {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
        if let Some(on_change) = &this.field.on_change {
            on_change(this.field.buffer.text().to_string());
        }
    }
}
InputCommand::Paste if !this.field.read_only => {
    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
        && this
            .field
            .buffer
            .paste_text(&text)
            .map(|outcome| outcome.changed)
            .unwrap_or(false)
        && let Some(on_change) = &this.field.on_change
    {
        on_change(this.field.buffer.text().to_string());
    }
}
```

- [ ] **Step 4: Repair TextArea clipboard and visual segments**

In `TextArea::render`, replace text rendering with:

```rust
let focused = self.focus_handle.is_focused(window);
let segments = self.buffer.display_segments(DisplayMode::Plain, focused);
```

Replace `.child(text)` with:

```rust
.child(render_segments(segments, self.placeholder.clone(), is_empty))
```

In the key listener, handle clipboard commands before `apply_command`:

```rust
match command {
    InputCommand::Copy => {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
            this.buffer.selected_text().to_string(),
        ));
    }
    InputCommand::Cut => {
        if let Some(text) = this.buffer.cut_selection() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
            if let Some(on_change) = &this.on_change {
                on_change(this.buffer.text().to_string());
            }
        }
    }
    InputCommand::Paste => {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
            && this
                .buffer
                .paste_text(&text)
                .map(|outcome| outcome.changed)
                .unwrap_or(false)
            && let Some(on_change) = &this.on_change
        {
            on_change(this.buffer.text().to_string());
        }
    }
    command => {
        if this
            .buffer
            .apply_command(command)
            .map(|outcome| outcome.changed)
            .unwrap_or(false)
            && let Some(on_change) = &this.on_change
        {
            on_change(this.buffer.text().to_string());
        }
    }
}
```

- [ ] **Step 5: Run targeted checks**

Run:

```bash
cargo check
cargo test ui::input --lib
```

Expected: PASS.

- [ ] **Step 6: Commit field repair**

Run:

```bash
git add src/ui/input/field.rs
git commit -m "fix(ui): render input caret and repair text area clipboard"
```

---

### Task 3: Wire Composer Submit Through Keys And Button

**Files:**
- Modify: `src/ui/input/composer.rs`
- Modify: `src/ui/input/field.rs`

- [ ] **Step 1: Write failing composer tests**

Replace the existing composer tests in `src/ui/input/composer.rs` with:

```rust
#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::ui::input::InputCommand;

    #[test]
    fn empty_content_cannot_submit() {
        assert_eq!(ComposerState::new("  ").submit(), None);
    }

    #[test]
    fn successful_submit_returns_trimmed_content_and_clears_text() {
        let mut state = ComposerState::new(" summarize project ");

        assert_eq!(state.submit(), Some("summarize project".to_string()));
        assert_eq!(state.text(), "");
    }

    #[test]
    fn sending_state_blocks_submit_without_clearing() {
        let mut state = ComposerState::new("summarize project");
        state.set_sending(true);

        assert_eq!(state.submit(), None);
        assert_eq!(state.text(), "summarize project");
    }

    #[test]
    fn submit_command_invokes_callback_once() {
        let submitted = Arc::new(Mutex::new(Vec::new()));
        let submitted_callback = Arc::clone(&submitted);
        let mut state = ComposerState::new(" run tests ");

        handle_composer_command(
            &mut state,
            InputCommand::Submit,
            Some(&Box::new(move |content| {
                submitted_callback.lock().unwrap().push(content);
            })),
        );

        assert_eq!(submitted.lock().unwrap().as_slice(), ["run tests"]);
        assert_eq!(state.text(), "");
    }

    #[test]
    fn shift_enter_command_inserts_newline_without_submit() {
        let submitted = Arc::new(Mutex::new(Vec::new()));
        let submitted_callback = Arc::clone(&submitted);
        let mut state = ComposerState::new("hello");

        handle_composer_command(
            &mut state,
            InputCommand::InsertNewline,
            Some(&Box::new(move |content| {
                submitted_callback.lock().unwrap().push(content);
            })),
        );

        assert_eq!(state.text(), "hello\n");
        assert!(submitted.lock().unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Run failing composer tests**

Run:

```bash
cargo test ui::input::composer --lib
```

Expected: FAIL because `submit()` does not clear text and `handle_composer_command` does not exist.

- [ ] **Step 3: Implement composer command helper**

Modify `ComposerState::submit`:

```rust
pub fn submit(&mut self) -> Option<String> {
    if self.sending {
        return None;
    }
    let trimmed = self.text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let submitted = trimmed.to_string();
    self.text.clear();
    Some(submitted)
}
```

Add this function below `impl ComposerState`:

```rust
pub fn handle_composer_command(
    state: &mut ComposerState,
    command: crate::ui::input::InputCommand,
    on_submit: Option<&InputSubmit>,
) -> bool {
    match command {
        crate::ui::input::InputCommand::Submit => {
            if let Some(content) = state.submit() {
                if let Some(on_submit) = on_submit {
                    on_submit(content);
                }
                true
            } else {
                false
            }
        }
        crate::ui::input::InputCommand::InsertNewline => {
            state.set_text(format!("{}\n", state.text()));
            true
        }
        _ => false,
    }
}
```

Add this method to `impl TextArea` in `src/ui/input/field.rs` so `ComposerInput` can keep the visible text area in sync after submit and newline commands:

```rust
pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
    self.buffer.set_text(content);
    cx.notify();
}
```

- [ ] **Step 4: Wire key handling and send button**

Update imports in `src/ui/input/composer.rs`:

```rust
use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::ui::{
    components,
    input::{InputCommand, KeyBindingContext, TextArea, field::InputSubmit, key_to_command},
    theme,
};
```

Add method to `impl ComposerInput`:

```rust
fn apply_command(&mut self, command: InputCommand, cx: &mut Context<Self>) {
    let changed = {
        let mut state = self.state.lock().unwrap();
        handle_composer_command(&mut state, command, self.on_submit.as_ref())
    };
    if changed {
        let text = self.state.lock().unwrap().text().to_string();
        self.text_area.update(cx, |text_area, cx| {
            text_area.set_content(text, cx);
        });
    }
    cx.notify();
}

fn handle_key_down(
    &mut self,
    event: &KeyDownEvent,
    _window: &mut Window,
    cx: &mut Context<Self>,
) {
    let Some(command) = key_to_command(&event.keystroke, KeyBindingContext::Composer) else {
        return;
    };
    self.apply_command(command, cx);
}
```

In `render`, add `.on_key_down(cx.listener(Self::handle_key_down))` to the root `div()` and change the send icon child:

```rust
.child(
    div()
        .id(SharedString::from("composer-send"))
        .child(components::icon_button("↵"))
        .on_click(cx.listener(|this, _, _, cx| {
            this.apply_command(InputCommand::Submit, cx);
        })),
)
```

Rename `_cx` in render signature to `cx`.

Then update the `TextArea` key listener in `src/ui/input/field.rs` to use a configurable key context. Add this field to `TextArea`:

```rust
key_context: KeyBindingContext,
```

Set it in `TextArea::new`:

```rust
key_context: KeyBindingContext::TextArea,
```

Add this constructor:

```rust
pub fn new_with_context(
    cx: &mut App,
    placeholder: impl Into<String>,
    initial: &str,
    on_change: Option<InputChange>,
    key_context: KeyBindingContext,
) -> Entity<Self> {
    cx.new(|cx| Self {
        buffer: TextBuffer::new(InputMode::MultiLine, initial),
        placeholder: placeholder.into(),
        focus_handle: cx.focus_handle(),
        on_change,
        error_text: None,
        disabled: false,
        read_only: false,
        max_height: px(160.),
        key_context,
    })
}
```

Change the `TextArea` key listener mapping from:

```rust
key_to_command(&event.keystroke, KeyBindingContext::TextArea)
```

to:

```rust
key_to_command(&event.keystroke, this.key_context)
```

Finally, create the composer text area with composer key context:

```rust
let text_area = TextArea::new_with_context(
    cx,
    "描述你的下一步需求...",
    "",
    Some(Box::new(move |content| {
        callback_state.lock().unwrap().set_text(content);
    })),
    KeyBindingContext::Composer,
);
```

- [ ] **Step 5: Run composer tests**

Run:

```bash
cargo test ui::input::composer --lib
cargo check
```

Expected: PASS.

- [ ] **Step 6: Commit composer repair**

Run:

```bash
git add src/ui/input/composer.rs
git commit -m "fix(ui): wire composer submit"
```

---

### Task 4: Save Current Provider Candidate Values

**Files:**
- Modify: `src/ui/settings.rs`

- [ ] **Step 1: Write failing settings persistence tests**

Add these tests to `#[cfg(test)] mod tests` in `src/ui/settings.rs`:

```rust
#[test]
fn save_action_writes_edited_base_url_to_runtime_config() {
    let runtime_config = sample_runtime_config();
    let mut config = AppConfig::from_norma_config(&runtime_config);
    config
        .selected_provider_mut()
        .unwrap()
        .base_url = "https://proxy.example.com/v1".to_string();
    config.selected_provider_mut().unwrap().mark_tested();
    let state = SettingsWindowState {
        config,
        persisted_config: Some(runtime_config),
        config_file: Some(std::env::temp_dir().join("norma-settings-test.toml")),
    };

    let mut saved_config = None;
    state
        .save_selected_provider(|_, config| {
            saved_config = Some(config.clone());
            Ok(())
        })
        .unwrap();

    assert_eq!(
        saved_config.unwrap().ai.providers[0].base_url,
        "https://proxy.example.com/v1"
    );
}

#[test]
fn save_action_writes_edited_api_key_to_runtime_config() {
    let runtime_config = sample_runtime_config();
    let mut config = AppConfig::from_norma_config(&runtime_config);
    config
        .selected_provider_mut()
        .unwrap()
        .api_key_reference = "sk-new-secret".to_string();
    config.selected_provider_mut().unwrap().mark_tested();
    let state = SettingsWindowState {
        config,
        persisted_config: Some(runtime_config),
        config_file: Some(std::env::temp_dir().join("norma-settings-test.toml")),
    };

    let mut saved_config = None;
    state
        .save_selected_provider(|_, config| {
            saved_config = Some(config.clone());
            Ok(())
        })
        .unwrap();

    assert_eq!(saved_config.unwrap().ai.providers[0].api_key, "sk-new-secret");
}
```

- [ ] **Step 2: Run failing settings tests**

Run:

```bash
cargo test ui::settings::tests::save_action_writes_edited_base_url_to_runtime_config ui::settings::tests::save_action_writes_edited_api_key_to_runtime_config --lib
```

Expected: FAIL because `save_selected_provider` writes stale `persisted_config`.

- [ ] **Step 3: Build current config before writing**

Replace the last lines of `save_selected_provider`:

```rust
let Some(config) = self.persisted_config.as_ref() else {
    return Err("缺少可写入的配置".to_string());
};

write(path, config).map_err(|error| error.to_string())
```

with:

```rust
let Some(config) = self.persisted_config.as_ref() else {
    return Err("缺少可写入的配置".to_string());
};
let mut next_config = config.clone();
let next_provider = crate::config::AiProviderConfig {
    id: provider.id.clone(),
    name: provider.name.clone(),
    api_type: match provider.protocol {
        crate::config::ProviderProtocol::OpenAi => ProviderApiType::OpenAi,
        crate::config::ProviderProtocol::Anthropic => ProviderApiType::Anthropic,
    },
    base_url: provider.base_url.clone(),
    api_key: provider.api_key_reference.clone(),
    is_default: provider.is_default,
    models: provider
        .models
        .iter()
        .map(|model| crate::config::AiModelConfig {
            id: model.id.clone(),
            name: model.name.clone(),
            model_id: model.model_id.clone(),
            is_default: model.is_default,
        })
        .collect(),
};

if let Some(existing) = next_config
    .ai
    .providers
    .iter_mut()
    .find(|candidate| candidate.id == next_provider.id)
{
    *existing = next_provider;
} else {
    next_config.ai.providers.push(next_provider);
}

write(path, &next_config).map_err(|error| error.to_string())
```

- [ ] **Step 4: Run settings tests**

Run:

```bash
cargo test ui::settings --lib
```

Expected: PASS.

- [ ] **Step 5: Commit settings repair**

Run:

```bash
git add src/ui/settings.rs
git commit -m "fix(settings): save current provider candidate"
```

---

### Task 5: Full Verification

**Files:**
- Verify all modified files.

- [ ] **Step 1: Run formatting check**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If it fails, run:

```bash
cargo fmt
cargo fmt --check
```

- [ ] **Step 2: Run type check**

Run:

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 3: Run unit tests**

Run:

```bash
cargo test
```

Expected: PASS.

- [ ] **Step 4: Run clippy**

Run:

```bash
cargo clippy --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5: Run manual app verification**

Run:

```bash
cargo run
```

Expected: the Norma app opens. Manually verify:

- Settings provider edits are visible.
- Testing the edited provider enables save.
- Saving writes the edited provider values.
- API key remains masked until reveal is clicked.
- Composer `Enter` submits.
- Composer `Shift+Enter` inserts newline.
- Composer send button submits.
- Focused inputs show a visible caret or selected text.
- TextArea copy, cut, and paste work for multi-line text.

- [ ] **Step 6: Inspect final diff and status**

Run:

```bash
git status --short
git diff --stat
git diff --check
```

Expected: no uncommitted changes unless `cargo fmt` changed files. `git diff --check` reports no whitespace errors.

- [ ] **Step 7: Commit verification-only changes if needed**

If formatting or checklist updates changed files during Task 5, run:

```bash
git add src/ui/input/model.rs src/ui/input/mod.rs src/ui/input/field.rs src/ui/input/composer.rs src/ui/settings.rs tests/settings_visual_contract.md tests/visual_contract.md
git commit -m "chore(ui): finalize text input fixes"
```

If no files changed, do not create an empty commit.
