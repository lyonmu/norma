use gpui::{
    App, AppContext, Context, CursorStyle, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, Styled, Window, div, px,
};

use crate::ui::theme;

pub struct TextInput {
    content: String,
    placeholder: String,
    focus_handle: FocusHandle,
    on_change: Option<Box<dyn Fn(String) + 'static>>,
}

impl TextInput {
    pub fn new(cx: &mut App, placeholder: impl Into<String>, initial: &str) -> Entity<Self> {
        Self::new_with_callback(cx, placeholder, initial, None)
    }

    pub fn new_with_callback(
        cx: &mut App,
        placeholder: impl Into<String>,
        initial: &str,
        on_change: Option<Box<dyn Fn(String) + 'static>>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            Self {
                content: initial.to_string(),
                placeholder: placeholder.into(),
                focus_handle,
                on_change,
            }
        })
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn set_content(&mut self, content: String, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;

        if let Some(key_char) = &keystroke.key_char {
            self.content.push_str(key_char);
            if let Some(on_change) = &self.on_change {
                on_change(self.content.clone());
            }
            cx.notify();
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.content.pop();
                if let Some(on_change) = &self.on_change {
                    on_change(self.content.clone());
                }
                cx.notify();
            }
            "delete" => {
                if !self.content.is_empty() {
                    self.content.clear();
                    if let Some(on_change) = &self.on_change {
                        on_change(self.content.clone());
                    }
                    cx.notify();
                }
            }
            "escape" => {
                _window.blur();
            }
            _ => {}
        }
    }

    fn on_mouse_down(
        &mut self,
        _event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.focus_self(window);
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(_window);
        let display_text = if self.content.is_empty() {
            self.placeholder.clone()
        } else {
            self.content.clone()
        };
        let text_color = if self.content.is_empty() {
            theme::muted()
        } else {
            theme::text()
        };

        div()
            .h(px(38.))
            .rounded(px(8.))
            .border_1()
            .border_color(if is_focused {
                theme::blue()
            } else {
                theme::border()
            })
            .bg(theme::surface())
            .px_3()
            .flex()
            .items_center()
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(Self::on_mouse_down))
            .child(
                div()
                    .text_color(text_color)
                    .text_size(px(14.))
                    .child(display_text),
            )
    }
}
