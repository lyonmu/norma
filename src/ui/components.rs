use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, px};

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
    let bg = if active {
        theme::blue()
    } else {
        theme::surface_tint()
    };
    let fg = if active {
        theme::surface()
    } else {
        theme::muted()
    };
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
