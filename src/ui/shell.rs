use gpui::{
    App, Application, Bounds, Context, IntoElement, ParentElement, Render, Styled, Window,
    WindowBounds, WindowOptions, div, point, prelude::*, px, size,
};

use crate::app_state::NormaAppState;
use crate::ui::{components, sidebar, theme};

pub struct AppShell {
    state: NormaAppState,
}

impl AppShell {
    pub fn new(state: NormaAppState) -> Self {
        Self { state }
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .flex()
            .flex_col()
            .child(top_toolbar())
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
                            .child(components::section_title(
                                self.state.session.thread.title.clone(),
                            )),
                    )
                    .child(
                        div()
                            .w(theme::INSPECTOR_WIDTH)
                            .h_full()
                            .border_l_1()
                            .border_color(theme::border())
                            .p_5()
                            .child(components::section_title("检查器")),
                    ),
            )
    }
}

fn top_toolbar() -> impl IntoElement {
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
                .child(components::icon_button("⚙")),
        )
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let state = NormaAppState::load_current_project();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(80.), px(80.)),
                size(px(1440.), px(1024.)),
            ))),
            ..WindowOptions::default()
        };
        cx.open_window(options, |_, cx| cx.new(|_| AppShell::new(state)))
            .expect("failed to open Norma window");
    });
}
