# Text Input System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build Norma's reusable desktop text input system, migrate settings provider fields to it, and replace the static execution composer with a real multi-line composer input.

**Architecture:** Add `src/ui/input/` with a pure Rust editing model, a GPUI key-command mapper, reusable GPUI field components, and a composer-specific component. Business state remains in settings, session, and execution modules; input components only own editing state and emit typed changes or submit events.

**Tech Stack:** Rust 2024, GPUI 0.2.2, existing Norma UI theme/components, Rust unit tests, manual GPUI visual verification.

---

## Scope Check

This plan implements one coherent subsystem: reusable text input. It touches settings and composer only as consumers of that subsystem. Command palette behavior, slash commands, `@` references, autocomplete popovers, and runtime task execution from composer submit are outside this plan.

## File Structure

- Create `src/ui/input/mod.rs`: module exports for the input system.
- Create `src/ui/input/model.rs`: pure text buffer, selection, edit commands, undo/redo, secure display helpers.
- Create `src/ui/input/command.rs`: GPUI `KeyDownEvent` to `InputCommand` mapping and clipboard command classification.
- Create `src/ui/input/field.rs`: GPUI `TextField`, `SecureTextField`, `TextArea`, and `FormField` rendering.
- Create `src/ui/input/composer.rs`: GPUI `ComposerInput` state and submit behavior.
- Modify `src/ui/mod.rs`: export `input`; remove `text_input` export after migration.
- Modify `src/ui/settings.rs`: replace `TextInput` entities with new field entities and invalidate provider test state on edits.
- Modify `src/ui/execution.rs`: replace static composer placeholder with `ComposerInput`.
- Delete `src/ui/text_input.rs`: remove the obsolete one-off component after all call sites move.
- Modify `tests/settings_visual_contract.md`: add input state checks.
- Modify `tests/visual_contract.md`: add composer input checks.

---

### Task 1: Add The Pure Text Editing Model

**Files:**
- Create: `src/ui/input/mod.rs`
- Create: `src/ui/input/model.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create the input module export**

Add this to `src/ui/input/mod.rs`:

```rust
pub mod command;
pub mod composer;
pub mod field;
pub mod model;

pub use command::{InputCommand, KeyBindingContext, key_to_command};
pub use composer::ComposerInput;
pub use field::{FormField, SecureTextField, TextArea, TextField};
pub use model::{
    DisplayMode, EditOutcome, InputMode, Selection, TextBuffer, TextEditError, TextSnapshot,
};
```

Modify `src/ui/mod.rs`:

```rust
pub mod components;
pub mod execution;
pub mod input;
pub mod inspector;
pub mod settings;
pub mod shell;
pub mod sidebar;
pub mod text_input;
pub mod theme;
```

- [ ] **Step 2: Write failing model tests**

Create `src/ui/input/model.rs` with the test module first. The file will not compile until Step 4 adds the implementation.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_text_at_caret_and_moves_caret() {
        let mut buffer = TextBuffer::new(InputMode::SingleLine, "hello");

        buffer.move_to_start(false);
        buffer.insert_text("Say ").unwrap();

        assert_eq!(buffer.text(), "Say hello");
        assert_eq!(buffer.selection(), Selection::caret(4));
    }

    #[test]
    fn replaces_selection_with_text() {
        let mut buffer = TextBuffer::new(InputMode::SingleLine, "hello world");

        buffer.set_selection(Selection::range(6, 11));
        buffer.insert_text("Norma").unwrap();

        assert_eq!(buffer.text(), "hello Norma");
        assert_eq!(buffer.selection(), Selection::caret(11));
    }

    #[test]
    fn delete_backward_removes_previous_character_not_byte() {
        let mut buffer = TextBuffer::new(InputMode::SingleLine, "你a");

        buffer.delete_backward();
        assert_eq!(buffer.text(), "你");
        buffer.delete_backward();
        assert_eq!(buffer.text(), "");
    }

    #[test]
    fn single_line_rejects_newline() {
        let mut buffer = TextBuffer::new(InputMode::SingleLine, "hello");

        let error = buffer.insert_text("\n").unwrap_err();

        assert_eq!(error, TextEditError::NewlineRejected);
        assert_eq!(buffer.text(), "hello");
    }

    #[test]
    fn multi_line_accepts_newline() {
        let mut buffer = TextBuffer::new(InputMode::MultiLine, "hello");

        buffer.insert_text("\nworld").unwrap();

        assert_eq!(buffer.text(), "hello\nworld");
    }

    #[test]
    fn undo_and_redo_restore_text_and_selection() {
        let mut buffer = TextBuffer::new(InputMode::SingleLine, "hello");

        buffer.insert_text(" world").unwrap();
        assert_eq!(buffer.text(), "hello world");

        assert!(buffer.undo());
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.selection(), Selection::caret(5));

        assert!(buffer.redo());
        assert_eq!(buffer.text(), "hello world");
        assert_eq!(buffer.selection(), Selection::caret(11));
    }

    #[test]
    fn max_len_blocks_extra_text() {
        let mut buffer = TextBuffer::new(InputMode::SingleLine, "abcd").with_max_len(5);

        buffer.insert_text("ef").unwrap();

        assert_eq!(buffer.text(), "abcde");
        assert_eq!(buffer.selection(), Selection::caret(5));
    }

    #[test]
    fn secure_display_masks_without_changing_raw_text() {
        let buffer = TextBuffer::new(InputMode::SingleLine, "sk-test-secret");

        assert_eq!(buffer.display_text(DisplayMode::Secure), "••••••••cret");
        assert_eq!(buffer.text(), "sk-test-secret");
        assert_eq!(buffer.display_text(DisplayMode::Plain), "sk-test-secret");
    }
}
```

- [ ] **Step 3: Run the failing model test**

Run:

```bash
cargo test ui::input::model --lib
```

Expected: FAIL because `TextBuffer`, `InputMode`, `Selection`, `DisplayMode`, and `TextEditError` are not implemented.

- [ ] **Step 4: Implement the model**

Add this implementation above the test module in `src/ui/input/model.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputMode {
    SingleLine,
    MultiLine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisplayMode {
    Plain,
    Secure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextEditError {
    NewlineRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub fn caret(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    pub fn range(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    pub fn is_caret(self) -> bool {
        self.anchor == self.head
    }

    pub fn ordered(self) -> std::ops::Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextSnapshot {
    text: String,
    selection: Selection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditOutcome {
    pub changed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextBuffer {
    text: String,
    mode: InputMode,
    selection: Selection,
    max_len: Option<usize>,
    undo_stack: Vec<TextSnapshot>,
    redo_stack: Vec<TextSnapshot>,
}

impl TextBuffer {
    pub fn new(mode: InputMode, initial: impl Into<String>) -> Self {
        let text = initial.into();
        let end = text.len();
        Self {
            text,
            mode,
            selection: Selection::caret(end),
            max_len: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn with_max_len(mut self, max_len: usize) -> Self {
        self.max_len = Some(max_len);
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    pub fn set_selection(&mut self, selection: Selection) {
        self.selection = self.clamp_selection(selection);
    }

    pub fn display_text(&self, mode: DisplayMode) -> String {
        match mode {
            DisplayMode::Plain => self.text.clone(),
            DisplayMode::Secure => mask_secret(&self.text),
        }
    }

    pub fn insert_text(&mut self, text: &str) -> Result<EditOutcome, TextEditError> {
        if self.mode == InputMode::SingleLine && text.contains('\n') {
            return Err(TextEditError::NewlineRejected);
        }
        let mut insertion = text.to_string();
        if let Some(max_len) = self.max_len {
            let current_len = self.text.chars().count();
            let selected_len = self.selected_text().chars().count();
            let available = max_len.saturating_sub(current_len.saturating_sub(selected_len));
            insertion = insertion.chars().take(available).collect();
        }
        if insertion.is_empty() && text.is_empty() {
            return Ok(EditOutcome { changed: false });
        }
        self.push_undo();
        self.replace_selection_raw(&insertion);
        self.redo_stack.clear();
        Ok(EditOutcome { changed: true })
    }

    pub fn delete_backward(&mut self) -> EditOutcome {
        if !self.selection.is_caret() {
            self.push_undo();
            self.replace_selection_raw("");
            self.redo_stack.clear();
            return EditOutcome { changed: true };
        }
        let caret = self.selection.head;
        let Some(previous) = previous_char_boundary(&self.text, caret) else {
            return EditOutcome { changed: false };
        };
        self.push_undo();
        self.text.replace_range(previous..caret, "");
        self.selection = Selection::caret(previous);
        self.redo_stack.clear();
        EditOutcome { changed: true }
    }

    pub fn delete_forward(&mut self) -> EditOutcome {
        if !self.selection.is_caret() {
            self.push_undo();
            self.replace_selection_raw("");
            self.redo_stack.clear();
            return EditOutcome { changed: true };
        }
        let caret = self.selection.head;
        let Some(next) = next_char_boundary(&self.text, caret) else {
            return EditOutcome { changed: false };
        };
        self.push_undo();
        self.text.replace_range(caret..next, "");
        self.selection = Selection::caret(caret);
        self.redo_stack.clear();
        EditOutcome { changed: true }
    }

    pub fn select_all(&mut self) {
        self.selection = Selection::range(0, self.text.len());
    }

    pub fn move_to_start(&mut self, selecting: bool) {
        self.move_to(0, selecting);
    }

    pub fn move_to_end(&mut self, selecting: bool) {
        self.move_to(self.text.len(), selecting);
    }

    pub fn move_left(&mut self, selecting: bool) {
        let target = previous_char_boundary(&self.text, self.selection.head).unwrap_or(0);
        self.move_to(target, selecting);
    }

    pub fn move_right(&mut self, selecting: bool) {
        let target = next_char_boundary(&self.text, self.selection.head).unwrap_or(self.text.len());
        self.move_to(target, selecting);
    }

    pub fn selected_text(&self) -> &str {
        let range = self.selection.ordered();
        &self.text[range]
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.redo_stack.push(current);
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        let current = self.snapshot();
        self.undo_stack.push(current);
        self.restore(next);
        true
    }

    fn move_to(&mut self, target: usize, selecting: bool) {
        let target = clamp_to_char_boundary(&self.text, target);
        self.selection = if selecting {
            Selection::range(self.selection.anchor, target)
        } else {
            Selection::caret(target)
        };
    }

    fn replace_selection_raw(&mut self, replacement: &str) {
        let range = self.selection.ordered();
        self.text.replace_range(range.clone(), replacement);
        let caret = range.start + replacement.len();
        self.selection = Selection::caret(caret);
    }

    fn clamp_selection(&self, selection: Selection) -> Selection {
        Selection {
            anchor: clamp_to_char_boundary(&self.text, selection.anchor.min(self.text.len())),
            head: clamp_to_char_boundary(&self.text, selection.head.min(self.text.len())),
        }
    }

    fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            text: self.text.clone(),
            selection: self.selection,
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.snapshot());
    }

    fn restore(&mut self, snapshot: TextSnapshot) {
        self.text = snapshot.text;
        self.selection = self.clamp_selection(snapshot.selection);
    }
}

fn mask_secret(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= 4 {
        return "•".repeat(char_count);
    }
    let suffix: String = text
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{}{}", "•".repeat(char_count.saturating_sub(4)), suffix)
}

fn previous_char_boundary(text: &str, offset: usize) -> Option<usize> {
    if offset == 0 {
        return None;
    }
    text[..offset].char_indices().last().map(|(idx, _)| idx)
}

fn next_char_boundary(text: &str, offset: usize) -> Option<usize> {
    if offset >= text.len() {
        return None;
    }
    text[offset..]
        .char_indices()
        .nth(1)
        .map(|(idx, _)| offset + idx)
        .or(Some(text.len()))
}

fn clamp_to_char_boundary(text: &str, mut offset: usize) -> usize {
    offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}
```

- [ ] **Step 5: Run the model test**

Run:

```bash
cargo test ui::input::model --lib
```

Expected: PASS.

- [ ] **Step 6: Commit the model**

Run:

```bash
git add src/ui/mod.rs src/ui/input/mod.rs src/ui/input/model.rs
git commit -m "feat(ui): add text input editing model"
```

---

### Task 2: Add Key Command Mapping

**Files:**
- Create: `src/ui/input/command.rs`
- Modify: `src/ui/input/mod.rs`

- [ ] **Step 1: Write failing command tests**

Create `src/ui/input/command.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Keystroke, Modifiers};

    fn key(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            key: key.to_string(),
            key_char: key_char.map(str::to_string),
            modifiers,
        }
    }

    #[test]
    fn maps_plain_character_to_insert_text() {
        assert_eq!(
            key_to_command(&key("a", Some("a"), Modifiers::none()), KeyBindingContext::TextField),
            Some(InputCommand::InsertText("a".to_string()))
        );
    }

    #[test]
    fn maps_secondary_shortcuts() {
        assert_eq!(
            key_to_command(&key("a", None, Modifiers::secondary()), KeyBindingContext::TextField),
            Some(InputCommand::SelectAll)
        );
        assert_eq!(
            key_to_command(&key("z", None, Modifiers::secondary()), KeyBindingContext::TextField),
            Some(InputCommand::Undo)
        );
        assert_eq!(
            key_to_command(
                &key("z", None, Modifiers::secondary() | Modifiers::shift()),
                KeyBindingContext::TextField
            ),
            Some(InputCommand::Redo)
        );
    }

    #[test]
    fn maps_delete_and_movement() {
        assert_eq!(
            key_to_command(&key("backspace", None, Modifiers::none()), KeyBindingContext::TextField),
            Some(InputCommand::DeleteBackward)
        );
        assert_eq!(
            key_to_command(&key("delete", None, Modifiers::none()), KeyBindingContext::TextField),
            Some(InputCommand::DeleteForward)
        );
        assert_eq!(
            key_to_command(&key("left", None, Modifiers::shift()), KeyBindingContext::TextField),
            Some(InputCommand::MoveLeft { selecting: true })
        );
    }

    #[test]
    fn enter_submits_in_composer_and_shift_enter_inserts_newline() {
        assert_eq!(
            key_to_command(&key("enter", None, Modifiers::none()), KeyBindingContext::Composer),
            Some(InputCommand::Submit)
        );
        assert_eq!(
            key_to_command(&key("enter", None, Modifiers::shift()), KeyBindingContext::Composer),
            Some(InputCommand::InsertNewline)
        );
    }
}
```

- [ ] **Step 2: Run the failing command test**

Run:

```bash
cargo test ui::input::command --lib
```

Expected: FAIL because `InputCommand`, `KeyBindingContext`, and `key_to_command` are missing.

- [ ] **Step 3: Implement command mapping**

Add this above the tests in `src/ui/input/command.rs`:

```rust
use gpui::{Keystroke, Modifiers};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyBindingContext {
    TextField,
    TextArea,
    Composer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputCommand {
    InsertText(String),
    InsertNewline,
    DeleteBackward,
    DeleteForward,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveToStart { selecting: bool },
    MoveToEnd { selecting: bool },
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    Submit,
    Blur,
}

pub fn key_to_command(
    keystroke: &Keystroke,
    context: KeyBindingContext,
) -> Option<InputCommand> {
    let modifiers = keystroke.modifiers;
    let secondary = modifiers.secondary();
    let shift = modifiers.shift;
    let only_shift = shift && !modifiers.secondary() && !modifiers.control && !modifiers.alt;
    let no_modifiers = !modifiers.modified();

    if secondary {
        match keystroke.key.as_str() {
            "a" => return Some(InputCommand::SelectAll),
            "c" => return Some(InputCommand::Copy),
            "x" => return Some(InputCommand::Cut),
            "v" => return Some(InputCommand::Paste),
            "z" if shift => return Some(InputCommand::Redo),
            "z" => return Some(InputCommand::Undo),
            _ => {}
        }
    }

    match keystroke.key.as_str() {
        "backspace" => Some(InputCommand::DeleteBackward),
        "delete" => Some(InputCommand::DeleteForward),
        "left" => Some(InputCommand::MoveLeft { selecting: shift }),
        "right" => Some(InputCommand::MoveRight { selecting: shift }),
        "home" => Some(InputCommand::MoveToStart { selecting: shift }),
        "end" => Some(InputCommand::MoveToEnd { selecting: shift }),
        "escape" => Some(InputCommand::Blur),
        "enter" => match context {
            KeyBindingContext::Composer if shift => Some(InputCommand::InsertNewline),
            KeyBindingContext::Composer => Some(InputCommand::Submit),
            KeyBindingContext::TextArea => Some(InputCommand::InsertNewline),
            KeyBindingContext::TextField => Some(InputCommand::Submit),
        },
        _ if no_modifiers || only_shift => keystroke
            .key_char
            .as_ref()
            .map(|text| InputCommand::InsertText(text.clone())),
        _ => None,
    }
}
```

- [ ] **Step 4: Run command tests**

Run:

```bash
cargo test ui::input::command --lib
```

Expected: PASS.

- [ ] **Step 5: Commit command mapping**

Run:

```bash
git add src/ui/input/command.rs src/ui/input/mod.rs
git commit -m "feat(ui): map text input key commands"
```

---

### Task 3: Add GPUI Field Components

**Files:**
- Create: `src/ui/input/field.rs`
- Modify: `src/ui/input/model.rs`
- Modify: `src/ui/input/mod.rs`

- [ ] **Step 1: Add model helpers for view code**

Add these methods to `impl TextBuffer` in `src/ui/input/model.rs`:

```rust
pub fn set_text(&mut self, text: impl Into<String>) {
    self.text = text.into();
    self.selection = Selection::caret(self.text.len());
    self.undo_stack.clear();
    self.redo_stack.clear();
}

pub fn apply_command(&mut self, command: crate::ui::input::InputCommand) -> Result<EditOutcome, TextEditError> {
    match command {
        crate::ui::input::InputCommand::InsertText(text) => self.insert_text(&text),
        crate::ui::input::InputCommand::InsertNewline => self.insert_text("\n"),
        crate::ui::input::InputCommand::DeleteBackward => Ok(self.delete_backward()),
        crate::ui::input::InputCommand::DeleteForward => Ok(self.delete_forward()),
        crate::ui::input::InputCommand::MoveLeft { selecting } => {
            self.move_left(selecting);
            Ok(EditOutcome { changed: false })
        }
        crate::ui::input::InputCommand::MoveRight { selecting } => {
            self.move_right(selecting);
            Ok(EditOutcome { changed: false })
        }
        crate::ui::input::InputCommand::MoveToStart { selecting } => {
            self.move_to_start(selecting);
            Ok(EditOutcome { changed: false })
        }
        crate::ui::input::InputCommand::MoveToEnd { selecting } => {
            self.move_to_end(selecting);
            Ok(EditOutcome { changed: false })
        }
        crate::ui::input::InputCommand::SelectAll => {
            self.select_all();
            Ok(EditOutcome { changed: false })
        }
        crate::ui::input::InputCommand::Undo => Ok(EditOutcome {
            changed: self.undo(),
        }),
        crate::ui::input::InputCommand::Redo => Ok(EditOutcome {
            changed: self.redo(),
        }),
        crate::ui::input::InputCommand::Copy
        | crate::ui::input::InputCommand::Cut
        | crate::ui::input::InputCommand::Paste
        | crate::ui::input::InputCommand::Submit
        | crate::ui::input::InputCommand::Blur => Ok(EditOutcome { changed: false }),
    }
}
```

- [ ] **Step 2: Create field component implementation**

Create `src/ui/input/field.rs`:

```rust
use gpui::{
    App, AppContext, Context, CursorStyle, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Window, div, px,
};

use crate::ui::{
    input::{
        DisplayMode, InputCommand, InputMode, KeyBindingContext, TextBuffer, key_to_command,
    },
    theme,
};

pub type InputChange = Box<dyn Fn(String) + 'static>;
pub type InputSubmit = Box<dyn Fn(String) + 'static>;

pub struct TextField {
    buffer: TextBuffer,
    placeholder: String,
    focus_handle: FocusHandle,
    on_change: Option<InputChange>,
    on_submit: Option<InputSubmit>,
    error_text: Option<String>,
    disabled: bool,
    read_only: bool,
}

pub struct SecureTextField {
    field: TextField,
    reveal: bool,
}

pub struct TextArea {
    buffer: TextBuffer,
    placeholder: String,
    focus_handle: FocusHandle,
    on_change: Option<InputChange>,
    error_text: Option<String>,
    disabled: bool,
    read_only: bool,
    max_height: gpui::Pixels,
}

pub struct FormField {
    label: String,
    help_text: Option<String>,
    error_text: Option<String>,
    child: AnyInput,
}

pub enum AnyInput {
    TextField(Entity<TextField>),
    SecureTextField(Entity<SecureTextField>),
    TextArea(Entity<TextArea>),
}

impl TextField {
    pub fn new(
        cx: &mut App,
        placeholder: impl Into<String>,
        initial: &str,
        on_change: Option<InputChange>,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            buffer: TextBuffer::new(InputMode::SingleLine, initial),
            placeholder: placeholder.into(),
            focus_handle: cx.focus_handle(),
            on_change,
            on_submit: None,
            error_text: None,
            disabled: false,
            read_only: false,
        })
    }

    pub fn content(&self) -> &str {
        self.buffer.text()
    }

    pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
        self.buffer.set_text(content);
        cx.notify();
    }

    pub fn set_error_text(&mut self, error_text: Option<String>, cx: &mut Context<Self>) {
        self.error_text = error_text;
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(command) = key_to_command(&event.keystroke, KeyBindingContext::TextField) else {
            return;
        };
        self.apply_command(command, window, cx);
    }

    fn apply_command(&mut self, command: InputCommand, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        match command {
            InputCommand::Submit => {
                if let Some(on_submit) = &self.on_submit {
                    on_submit(self.buffer.text().to_string());
                } else {
                    window.blur();
                }
            }
            InputCommand::Blur => window.blur(),
            InputCommand::Copy => cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.buffer.selected_text().to_string())),
            InputCommand::Cut if !self.read_only => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.buffer.selected_text().to_string()));
                if self.buffer.delete_backward().changed {
                    self.emit_change();
                }
            }
            InputCommand::Paste if !self.read_only => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    if self.buffer.insert_text(&text).map(|outcome| outcome.changed).unwrap_or(false) {
                        self.emit_change();
                    }
                }
            }
            command if !self.read_only => {
                if self.buffer.apply_command(command).map(|outcome| outcome.changed).unwrap_or(false) {
                    self.emit_change();
                }
            }
            _ => {}
        }
        cx.notify();
    }

    fn emit_change(&self) {
        if let Some(on_change) = &self.on_change {
            on_change(self.buffer.text().to_string());
        }
    }

    fn on_mouse_down(&mut self, _: &gpui::MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.focus_self(window);
    }
}

impl Focusable for TextField {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_input_shell(
            self.focus_handle.is_focused(window),
            self.disabled,
            self.error_text.is_some(),
            self.buffer.text().is_empty(),
            self.buffer.display_text(DisplayMode::Plain),
            self.placeholder.clone(),
        )
        .track_focus(&self.focus_handle)
        .on_key_down(cx.listener(Self::handle_key_down))
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(Self::on_mouse_down))
    }
}

impl SecureTextField {
    pub fn new(
        cx: &mut App,
        placeholder: impl Into<String>,
        initial: &str,
        on_change: Option<InputChange>,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            field: TextField {
                buffer: TextBuffer::new(InputMode::SingleLine, initial),
                placeholder: placeholder.into(),
                focus_handle: cx.focus_handle(),
                on_change,
                on_submit: None,
                error_text: None,
                disabled: false,
                read_only: false,
            },
            reveal: false,
        })
    }
}

impl Focusable for SecureTextField {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.field.focus_handle.clone()
    }
}

impl Render for SecureTextField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let display_mode = if self.reveal {
            DisplayMode::Plain
        } else {
            DisplayMode::Secure
        };
        let label = if self.reveal { "隐藏" } else { "显示" };
        render_input_shell(
            self.field.focus_handle.is_focused(window),
            self.field.disabled,
            self.field.error_text.is_some(),
            self.field.buffer.text().is_empty(),
            self.field.buffer.display_text(display_mode),
            self.field.placeholder.clone(),
        )
        .track_focus(&self.field.focus_handle)
        .on_key_down(cx.listener(|this, event, window, cx| {
            this.field.handle_key_down(event, window, cx);
        }))
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
            cx.focus(&this.field.focus_handle, window);
        }))
        .child(
            div()
                .id(SharedString::from("secure-input-reveal"))
                .border_l_1()
                .border_color(theme::border())
                .pl_2()
                .ml_2()
                .text_size(px(12.))
                .text_color(theme::muted())
                .cursor_pointer()
                .child(label)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.reveal = !this.reveal;
                    cx.notify();
                })),
        )
    }
}

impl TextArea {
    pub fn new(
        cx: &mut App,
        placeholder: impl Into<String>,
        initial: &str,
        on_change: Option<InputChange>,
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
        })
    }
}

impl Focusable for TextArea {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextArea {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_empty = self.buffer.text().is_empty();
        let text = if is_empty {
            self.placeholder.clone()
        } else {
            self.buffer.text().to_string()
        };
        div()
            .min_h(px(86.))
            .max_h(self.max_height)
            .overflow_hidden()
            .rounded(px(8.))
            .border_1()
            .border_color(if self.focus_handle.is_focused(window) {
                theme::blue()
            } else if self.error_text.is_some() {
                theme::red()
            } else {
                theme::border()
            })
            .bg(if self.disabled { theme::surface_tint() } else { theme::surface() })
            .px_3()
            .py_2()
            .text_size(px(14.))
            .text_color(if is_empty { theme::muted() } else { theme::text() })
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let Some(command) = key_to_command(&event.keystroke, KeyBindingContext::TextArea) else {
                    return;
                };
                if this.disabled || this.read_only {
                    return;
                }
                if this.buffer.apply_command(command).map(|outcome| outcome.changed).unwrap_or(false) {
                    if let Some(on_change) = &this.on_change {
                        on_change(this.buffer.text().to_string());
                    }
                }
                cx.notify();
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                cx.focus(&this.focus_handle, window);
            }))
            .child(text)
    }
}

fn render_input_shell(
    focused: bool,
    disabled: bool,
    has_error: bool,
    empty: bool,
    text: String,
    placeholder: String,
) -> gpui::Div {
    div()
        .h(px(38.))
        .rounded(px(8.))
        .border_1()
        .border_color(if has_error {
            theme::red()
        } else if focused {
            theme::blue()
        } else {
            theme::border()
        })
        .bg(if disabled { theme::surface_tint() } else { theme::surface() })
        .px_3()
        .flex()
        .items_center()
        .cursor(CursorStyle::IBeam)
        .text_size(px(14.))
        .text_color(if empty { theme::muted() } else { theme::text() })
        .child(if empty { placeholder } else { text })
}
```

- [ ] **Step 3: Compile to catch GPUI type errors**

Run:

```bash
cargo check
```

Expected: PASS. `gpui::Div` is a public GPUI element type in GPUI 0.2.2, so `render_input_shell` can return `gpui::Div`.

- [ ] **Step 4: Commit field components**

Run:

```bash
git add src/ui/input/field.rs src/ui/input/model.rs src/ui/input/mod.rs
git commit -m "feat(ui): add reusable text field components"
```

---

### Task 4: Migrate Provider Settings Inputs

**Files:**
- Modify: `src/ui/settings.rs`
- Delete: `src/ui/text_input.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Write failing settings tests**

Add these tests to `#[cfg(test)] mod tests` in `src/ui/settings.rs`:

```rust
#[test]
fn editing_provider_field_invalidates_previous_test() {
    let runtime_config = sample_runtime_config();
    let mut config = AppConfig::from_norma_config(&runtime_config);
    config.selected_provider_mut().unwrap().mark_tested();
    assert!(config.selected_provider().unwrap().can_save());

    let mut state = SettingsWindowState {
        config,
        persisted_config: Some(runtime_config),
        config_file: None,
    };

    state.update_selected_provider_field(ProviderField::BaseUrl, "https://proxy.example.com/v1".to_string());

    let provider = state.config.selected_provider().unwrap();
    assert_eq!(provider.base_url, "https://proxy.example.com/v1");
    assert!(!provider.can_save());
    assert_eq!(provider.tested_candidate_fingerprint, None);
}

#[test]
fn api_key_field_updates_provider_without_logging_or_masking_raw_value() {
    let runtime_config = sample_runtime_config();
    let config = AppConfig::from_norma_config(&runtime_config);
    let mut state = SettingsWindowState {
        config,
        persisted_config: Some(runtime_config),
        config_file: None,
    };

    state.update_selected_provider_field(ProviderField::ApiKey, "sk-new-secret".to_string());

    assert_eq!(
        state.config.selected_provider().unwrap().api_key_reference,
        "sk-new-secret"
    );
}
```

- [ ] **Step 2: Run the failing settings tests**

Run:

```bash
cargo test ui::settings::tests::editing_provider_field_invalidates_previous_test ui::settings::tests::api_key_field_updates_provider_without_logging_or_masking_raw_value --lib
```

Expected: FAIL because `ProviderField` and `update_selected_provider_field` are not implemented.

- [ ] **Step 3: Add provider field update helper**

Add near `SettingsWindowState` in `src/ui/settings.rs`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProviderField {
    Name,
    BaseUrl,
    ApiKey,
    Model,
}

impl SettingsWindowState {
    fn update_selected_provider_field(&mut self, field: ProviderField, value: String) {
        if let Some(provider) = self.config.selected_provider_mut() {
            match field {
                ProviderField::Name => provider.name = value,
                ProviderField::BaseUrl => provider.base_url = value,
                ProviderField::ApiKey => provider.api_key_reference = value,
                ProviderField::Model => provider.model = value,
            }
            provider.status = ProviderConfigStatus::PreviewUnvalidated;
            provider.tested_candidate_fingerprint = None;
        }
    }
}
```

- [ ] **Step 4: Replace settings input entity types**

Change imports:

```rust
use crate::ui::{
    components,
    input::{SecureTextField, TextField},
    theme,
};
```

Change `SettingsWindow` fields:

```rust
name_input: Option<Entity<TextField>>,
base_url_input: Option<Entity<TextField>>,
api_key_input: Option<Entity<SecureTextField>>,
model_input: Option<Entity<TextField>>,
```

Change function signatures currently accepting `Entity<TextInput>` to `Entity<TextField>` and the API key argument to `Entity<SecureTextField>`.

- [ ] **Step 5: Create new input entities with centralized field updates**

Replace the four `TextInput::new_with_callback` blocks in `SettingsWindow::render` with:

```rust
self.name_input = Some(TextField::new(
    cx,
    "提供商名称",
    &provider.name,
    Some(Box::new({
        let state = Arc::clone(&self.state);
        move |content| {
            state
                .lock()
                .unwrap()
                .update_selected_provider_field(ProviderField::Name, content);
        }
    })),
));

self.base_url_input = Some(TextField::new(
    cx,
    "Base URL",
    &provider.base_url,
    Some(Box::new({
        let state = Arc::clone(&self.state);
        move |content| {
            state
                .lock()
                .unwrap()
                .update_selected_provider_field(ProviderField::BaseUrl, content);
        }
    })),
));

self.api_key_input = Some(SecureTextField::new(
    cx,
    "API Key",
    &provider.api_key_reference,
    Some(Box::new({
        let state = Arc::clone(&self.state);
        move |content| {
            state
                .lock()
                .unwrap()
                .update_selected_provider_field(ProviderField::ApiKey, content);
        }
    })),
));

self.model_input = Some(TextField::new(
    cx,
    "模型 ID",
    &provider.model,
    Some(Box::new({
        let state = Arc::clone(&self.state);
        move |content| {
            state
                .lock()
                .unwrap()
                .update_selected_provider_field(ProviderField::Model, content);
        }
    })),
));
```

- [ ] **Step 6: Remove duplicate input chrome in form rows**

Replace `editable_form_row` with two functions:

```rust
fn editable_text_form_row(label: &str, input: Option<&Entity<TextField>>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::label(label))
        .children(input.cloned())
        .into_any_element()
}

fn editable_secure_form_row(label: &str, input: Option<&Entity<SecureTextField>>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::label(label))
        .children(input.cloned())
        .into_any_element()
}
```

Update provider editor rows:

```rust
.child(editable_text_form_row("名称", name_input))
.child(protocol_segment(provider.protocol))
.child(editable_text_form_row("Base URL", base_url_input))
.child(editable_secure_form_row("API Key", api_key_input))
.child(editable_text_form_row("模型", model_input))
```

- [ ] **Step 7: Remove the old input module**

Delete `src/ui/text_input.rs`.

Modify `src/ui/mod.rs`:

```rust
pub mod components;
pub mod execution;
pub mod input;
pub mod inspector;
pub mod settings;
pub mod shell;
pub mod sidebar;
pub mod theme;
```

- [ ] **Step 8: Run settings tests**

Run:

```bash
cargo test ui::settings --lib
```

Expected: PASS.

- [ ] **Step 9: Run check**

Run:

```bash
cargo check
```

Expected: PASS.

- [ ] **Step 10: Commit settings migration**

Run:

```bash
git add src/ui/settings.rs src/ui/mod.rs src/ui/text_input.rs
git commit -m "feat(settings): use reusable text inputs"
```

---

### Task 5: Add Composer Input And Replace The Static Composer

**Files:**
- Create: `src/ui/input/composer.rs`
- Modify: `src/ui/execution.rs`
- Modify: `src/session/state.rs` if a sending flag is needed by existing session state

- [ ] **Step 1: Write composer tests**

Create `src/ui/input/composer.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_content_cannot_submit() {
        assert_eq!(ComposerState::new("  ").submit(), None);
    }

    #[test]
    fn submit_returns_trimmed_content_without_clearing_on_failure_boundary() {
        let mut state = ComposerState::new(" summarize project ");

        assert_eq!(state.submit(), Some("summarize project".to_string()));
        assert_eq!(state.text(), " summarize project ");
    }

    #[test]
    fn sending_state_blocks_submit() {
        let mut state = ComposerState::new("summarize project");
        state.set_sending(true);

        assert_eq!(state.submit(), None);
    }
}
```

- [ ] **Step 2: Run failing composer tests**

Run:

```bash
cargo test ui::input::composer --lib
```

Expected: FAIL because `ComposerState` is missing.

- [ ] **Step 3: Implement composer state and GPUI wrapper**

Add above the tests in `src/ui/input/composer.rs`:

```rust
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px,
};

use crate::ui::{
    components,
    input::{InputChange, InputSubmit, TextArea},
    theme,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerState {
    text: String,
    sending: bool,
}

impl ComposerState {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            text: initial.into(),
            sending: false,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
    }

    pub fn set_sending(&mut self, sending: bool) {
        self.sending = sending;
    }

    pub fn submit(&mut self) -> Option<String> {
        if self.sending {
            return None;
        }
        let trimmed = self.text.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    }
}

pub struct ComposerInput {
    state: ComposerState,
    text_area: Entity<TextArea>,
    on_submit: Option<InputSubmit>,
}

impl ComposerInput {
    pub fn new(cx: &mut App, on_submit: Option<InputSubmit>) -> Entity<Self> {
        cx.new(|cx| {
            let text_area = TextArea::new(
                cx,
                "描述你的下一步需求...",
                "",
                Some(Box::new(|_content| {})),
            );
            Self {
                state: ComposerState::new(""),
                text_area,
                on_submit,
            }
        })
    }
}

impl Render for ComposerInput {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
            .child(self.text_area.clone())
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
    }
}
```

- [ ] **Step 4: Wire TextArea changes into ComposerState**

Refine `ComposerInput::new` so the `TextArea` callback updates a shared `Arc<Mutex<ComposerState>>`. Use this exact state shape:

```rust
use std::sync::{Arc, Mutex};
```

Change `ComposerInput`:

```rust
pub struct ComposerInput {
    state: Arc<Mutex<ComposerState>>,
    text_area: Entity<TextArea>,
    on_submit: Option<InputSubmit>,
}
```

Change `new`:

```rust
pub fn new(cx: &mut App, on_submit: Option<InputSubmit>) -> Entity<Self> {
    cx.new(|cx| {
        let state = Arc::new(Mutex::new(ComposerState::new("")));
        let callback_state = Arc::clone(&state);
        let text_area = TextArea::new(
            cx,
            "描述你的下一步需求...",
            "",
            Some(Box::new(move |content| {
                callback_state.lock().unwrap().set_text(content);
            })),
        );
        Self {
            state,
            text_area,
            on_submit,
        }
    })
}
```

- [ ] **Step 5: Replace static composer rendering**

Modify `src/ui/execution.rs`.

Change imports:

```rust
use crate::ui::{components, input::ComposerInput, theme};
```

Change `render_execution` signature:

```rust
pub fn render_execution(session: &SessionState, composer: Option<&gpui::Entity<ComposerInput>>) -> AnyElement
```

Replace `.child(composer())` with:

```rust
.children(composer.cloned())
```

Remove the old `fn composer() -> AnyElement`.

Update `src/ui/shell.rs` to store and pass the composer entity:

```rust
use crate::ui::{
    components,
    execution,
    input::ComposerInput,
    inspector,
    settings::SettingsWindow,
    sidebar,
    theme,
};

pub struct AppShell {
    state: NormaAppState,
    updates: Receiver<RuntimeUpdate>,
    composer_input: Option<Entity<ComposerInput>>,
}
```

Initialize in `AppShell::new`:

```rust
Self {
    state,
    updates,
    composer_input: None,
}
```

In `render`, before returning the root div:

```rust
if self.composer_input.is_none() {
    self.composer_input = Some(ComposerInput::new(
        _cx,
        Some(Box::new(|content| {
            tracing::info!(
                component = "composer",
                prompt_len = content.chars().count(),
                "composer submit captured"
            );
        })),
    ));
}
```

Pass it:

```rust
.child(execution::render_execution(
    &self.state.session,
    self.composer_input.as_ref(),
))
```

- [ ] **Step 6: Run composer tests and check**

Run:

```bash
cargo test ui::input::composer --lib
cargo check
```

Expected: PASS.

- [ ] **Step 7: Commit composer migration**

Run:

```bash
git add src/ui/input/composer.rs src/ui/execution.rs src/ui/shell.rs
git commit -m "feat(ui): add composer text input"
```

---

### Task 6: Update Manual Visual Contracts

**Files:**
- Modify: `tests/settings_visual_contract.md`
- Modify: `tests/visual_contract.md`

- [ ] **Step 1: Update settings checklist**

Add this section to `tests/settings_visual_contract.md` after `## Provider Editor`:

```markdown
## Text Input States

- [ ] Provider name, Base URL, API Key, and model fields use one consistent input style.
- [ ] Focused input shows a blue border or subtle focus ring without changing layout size.
- [ ] API Key is masked by default and has a compact show/hide affordance.
- [ ] Long Base URL and model values remain inside the input bounds.
- [ ] Editing a provider field invalidates the previous tested state and disables saving until the candidate is tested again.
- [ ] Error text appears below the relevant field and does not overlap adjacent controls.
- [ ] Disabled or read-only inputs use muted text and a light tinted background.
- [ ] Chinese IME input enters committed Chinese text without duplicate or broken characters.
- [ ] Copy, paste, undo, redo, and select-all work in focused inputs.
```

- [ ] **Step 2: Update workbench checklist**

Replace the existing center execution stream composer item in `tests/visual_contract.md`:

```markdown
- [ ] Composer at bottom has a real multi-line text input with placeholder text and action pills.
- [ ] Composer supports focused, typing, multi-line, and sending/locked visual states.
- [ ] Enter submits composer content; Shift+Enter inserts a newline.
- [ ] Empty composer content cannot submit.
- [ ] Composer text does not overlap footer actions or resize the center pane unexpectedly.
```

- [ ] **Step 3: Commit checklist updates**

Run:

```bash
git add tests/settings_visual_contract.md tests/visual_contract.md
git commit -m "test(ui): document text input visual checks"
```

---

### Task 7: Full Verification And Cleanup

**Files:**
- Verify all modified files.

- [ ] **Step 1: Run rustfmt check**

Run:

```bash
cargo fmt --check
```

Expected: PASS. If it fails, run `cargo fmt`, then rerun `cargo fmt --check`.

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

- [ ] **Step 5: Run the desktop app for manual verification**

Run:

```bash
cargo run
```

Expected: the main Norma window opens. Verify the input-related items in `tests/visual_contract.md` and `tests/settings_visual_contract.md`.

- [ ] **Step 6: Inspect git diff**

Run:

```bash
git status --short
git diff --stat
git diff --check
```

Expected: only planned files changed, and `git diff --check` reports no whitespace errors.

- [ ] **Step 7: Final commit if formatting or verification changed files**

If Task 7 changed files, run:

```bash
git add src/ui/input src/ui/mod.rs src/ui/settings.rs src/ui/execution.rs src/ui/shell.rs tests/settings_visual_contract.md tests/visual_contract.md
git commit -m "chore(ui): finalize text input system"
```

If Task 7 did not change files, do not create an empty commit.
