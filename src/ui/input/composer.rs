use std::sync::{Arc, Mutex};

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::ui::{
    components,
    input::{InputCommand, KeyBindingContext, TextArea, field::InputSubmit, key_to_command},
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
        let submitted = trimmed.to_string();
        self.text.clear();
        Some(submitted)
    }
}

pub fn handle_composer_command(
    state: &mut ComposerState,
    command: InputCommand,
    on_submit: Option<&InputSubmit>,
) -> bool {
    match command {
        InputCommand::Submit => {
            if let Some(content) = state.submit() {
                if let Some(on_submit) = on_submit {
                    on_submit(content);
                }
                true
            } else {
                false
            }
        }
        InputCommand::InsertNewline => {
            state.set_text(format!("{}\n", state.text()));
            true
        }
        _ => false,
    }
}

#[allow(dead_code)]
pub struct ComposerInput {
    state: Arc<Mutex<ComposerState>>,
    text_area: Entity<TextArea>,
    on_submit: Option<InputSubmit>,
}

impl ComposerInput {
    pub fn new(cx: &mut App, on_submit: Option<InputSubmit>) -> Entity<Self> {
        cx.new(|cx| {
            let state = Arc::new(Mutex::new(ComposerState::new("")));
            let callback_state = Arc::clone(&state);
            let text_area = TextArea::new_with_context(
                cx,
                "描述你的下一步需求...",
                "",
                Some(Box::new(move |content| {
                    callback_state.lock().unwrap().set_text(content);
                })),
                KeyBindingContext::Composer,
            );
            Self {
                state,
                text_area,
                on_submit,
            }
        })
    }

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
}

impl Render for ComposerInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .on_key_down(cx.listener(Self::handle_key_down))
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
                            .child(
                                div()
                                    .id(SharedString::from("composer-send"))
                                    .child(components::icon_button("↵"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.apply_command(InputCommand::Submit, cx);
                                    })),
                            ),
                    ),
            )
    }
}

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

        let callback: InputSubmit = Box::new(move |content| {
            submitted_callback.lock().unwrap().push(content);
        });
        handle_composer_command(&mut state, InputCommand::Submit, Some(&callback));

        assert_eq!(submitted.lock().unwrap().as_slice(), ["run tests"]);
        assert_eq!(state.text(), "");
    }

    #[test]
    fn shift_enter_command_inserts_newline_without_submit() {
        let submitted = Arc::new(Mutex::new(Vec::new()));
        let submitted_callback = Arc::clone(&submitted);
        let mut state = ComposerState::new("hello");

        let callback: InputSubmit = Box::new(move |content| {
            submitted_callback.lock().unwrap().push(content);
        });
        handle_composer_command(&mut state, InputCommand::InsertNewline, Some(&callback));

        assert_eq!(state.text(), "hello\n");
        assert!(submitted.lock().unwrap().is_empty());
    }
}
