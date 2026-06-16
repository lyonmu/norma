use gpui::{
    App, AppContext, Context, CursorStyle, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Window, div, px,
};

use crate::ui::{
    input::{DisplayMode, InputCommand, InputMode, KeyBindingContext, TextBuffer, key_to_command},
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

#[allow(dead_code)]
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

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = key_to_command(&event.keystroke, KeyBindingContext::TextField) else {
            return;
        };
        self.apply_command(command, window, cx);
    }

    fn apply_command(
        &mut self,
        command: InputCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
            InputCommand::Copy => cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                self.buffer.selected_text().to_string(),
            )),
            InputCommand::Cut if !self.read_only => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                    self.buffer.selected_text().to_string(),
                ));
                if self.buffer.delete_backward().changed {
                    self.emit_change();
                }
            }
            InputCommand::Paste if !self.read_only => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                    && self
                        .buffer
                        .insert_text(&text)
                        .map(|outcome| outcome.changed)
                        .unwrap_or(false)
                {
                    self.emit_change();
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

    fn on_mouse_down(
        &mut self,
        _: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            let Some(command) = key_to_command(&event.keystroke, KeyBindingContext::TextField)
            else {
                return;
            };
            if this.field.disabled {
                return;
            }
            match command {
                InputCommand::Submit => {
                    if let Some(on_submit) = &this.field.on_submit {
                        on_submit(this.field.buffer.text().to_string());
                    } else {
                        window.blur();
                    }
                }
                InputCommand::Blur => window.blur(),
                InputCommand::Copy => cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                    this.field.buffer.selected_text().to_string(),
                )),
                InputCommand::Cut if !this.field.read_only => {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                        this.field.buffer.selected_text().to_string(),
                    ));
                    if this.field.buffer.delete_backward().changed
                        && let Some(on_change) = &this.field.on_change
                    {
                        on_change(this.field.buffer.text().to_string());
                    }
                }
                InputCommand::Paste if !this.field.read_only => {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                        && this
                            .field
                            .buffer
                            .insert_text(&text)
                            .map(|outcome| outcome.changed)
                            .unwrap_or(false)
                        && let Some(on_change) = &this.field.on_change
                    {
                        on_change(this.field.buffer.text().to_string());
                    }
                }
                command if !this.field.read_only => {
                    if this
                        .field
                        .buffer
                        .apply_command(command)
                        .map(|outcome| outcome.changed)
                        .unwrap_or(false)
                        && let Some(on_change) = &this.field.on_change
                    {
                        on_change(this.field.buffer.text().to_string());
                    }
                }
                _ => {}
            }
            cx.notify();
        }))
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _, window, _cx| {
                this.field.focus_handle.focus(window);
            }),
        )
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
            .bg(if self.disabled {
                theme::surface_tint()
            } else {
                theme::surface()
            })
            .px_3()
            .py_2()
            .text_size(px(14.))
            .text_color(if is_empty {
                theme::muted()
            } else {
                theme::text()
            })
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                let Some(command) = key_to_command(&event.keystroke, KeyBindingContext::TextArea)
                else {
                    return;
                };
                if this.disabled || this.read_only {
                    return;
                }
                if this
                    .buffer
                    .apply_command(command)
                    .map(|outcome| outcome.changed)
                    .unwrap_or(false)
                    && let Some(on_change) = &this.on_change
                {
                    on_change(this.buffer.text().to_string());
                }
                cx.notify();
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, _cx| {
                    this.focus_handle.focus(window);
                }),
            )
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
        .bg(if disabled {
            theme::surface_tint()
        } else {
            theme::surface()
        })
        .px_3()
        .flex()
        .items_center()
        .cursor(CursorStyle::IBeam)
        .text_size(px(14.))
        .text_color(if empty { theme::muted() } else { theme::text() })
        .child(if empty { placeholder } else { text })
}
