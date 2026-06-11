use gpui::{AnyElement, Context, IntoElement, ParentElement, Render, Styled, Window, div, px};

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
                .text_color(if selected {
                    theme::text()
                } else {
                    theme::muted()
                })
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
