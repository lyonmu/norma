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

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.selection = Selection::caret(self.text.len());
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn apply_command(
        &mut self,
        command: crate::ui::input::InputCommand,
    ) -> Result<EditOutcome, TextEditError> {
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

        assert_eq!(buffer.display_text(DisplayMode::Secure), "••••••••••cret");
        assert_eq!(buffer.text(), "sk-test-secret");
        assert_eq!(buffer.display_text(DisplayMode::Plain), "sk-test-secret");
    }
}
