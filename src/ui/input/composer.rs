use std::sync::{Arc, Mutex};

use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px,
};

use crate::ui::{components, input::TextArea, input::field::InputSubmit, theme};

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
