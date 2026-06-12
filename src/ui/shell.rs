use std::sync::mpsc::Receiver;

use gpui::{
    App, Application, Bounds, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};

use crate::app::NormaAppState;
use crate::runtime::RuntimeUpdate;
use crate::ui::{components, execution, inspector, settings::SettingsWindow, sidebar, theme};

pub struct AppShell {
    state: NormaAppState,
    updates: Receiver<RuntimeUpdate>,
}

impl AppShell {
    pub fn new(state: NormaAppState, updates: Receiver<RuntimeUpdate>) -> Self {
        Self { state, updates }
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        while let Ok(update) = self.updates.try_recv() {
            self.state.apply_runtime_update(update);
        }
        div()
            .size_full()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .flex()
            .flex_col()
            .child(top_toolbar(&self.state))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .w(theme::SIDEBAR_WIDTH)
                            .h_full()
                            .border_r_1()
                            .border_color(theme::border())
                            .child(sidebar::render_sidebar(&self.state)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .p_6()
                            .child(execution::render_execution(&self.state.session)),
                    )
                    .child(
                        div()
                            .w(theme::INSPECTOR_WIDTH)
                            .h_full()
                            .border_l_1()
                            .border_color(theme::border())
                            .child(inspector::render_inspector(&self.state)),
                    ),
            )
    }
}

fn top_toolbar(state: &NormaAppState) -> impl IntoElement {
    div()
        .h(theme::TOOLBAR_HEIGHT)
        .w_full()
        .px_5()
        .border_b_1()
        .border_color(theme::border())
        .flex()
        .items_center()
        .justify_between()
        .bg(theme::surface())
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(components::icon_button("N"))
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Norma"))
                .child(components::icon_button("←"))
                .child(components::icon_button("→")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(components::pill("模型 GPT-4.1", false))
                .child(components::pill("运行环境 本地", false))
                .child(components::pill("安全级别 标准", false)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(components::icon_button("▶"))
                .child(components::icon_button("🔔"))
                .child(settings_button(state)),
        )
}

fn settings_button(state: &NormaAppState) -> impl IntoElement {
    let config = state.config.clone();
    let runtime_config = state.runtime_config.clone();
    let config_file = state
        .runtime_paths
        .as_ref()
        .map(|paths| paths.config_file.clone());

    div()
        .id(SharedString::from("settings-button"))
        .w(px(32.))
        .h(px(32.))
        .rounded(px(8.))
        .border_1()
        .border_color(theme::border())
        .cursor_pointer()
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
            cx.open_window(options, |_, cx| {
                let config = config.clone();
                let runtime_config = runtime_config.clone();
                let config_file = config_file.clone();
                cx.new(|_| SettingsWindow::new(config, runtime_config, config_file))
            })
            .expect("failed to open Norma settings window");
        })
}

pub fn run(state: NormaAppState, updates: Receiver<RuntimeUpdate>) {
    Application::new().run(move |cx: &mut App| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(80.), px(80.)),
                size(px(1440.), px(1024.)),
            ))),
            ..WindowOptions::default()
        };
        cx.open_window(options, |_, cx| cx.new(|_| AppShell::new(state, updates)))
            .expect("failed to open Norma window");
    });
}
