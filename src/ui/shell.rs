use std::sync::mpsc::Receiver;

use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::*, px,
};

use crate::app::NormaAppState;
use crate::runtime::RuntimeUpdate;
use crate::ui::{
    components, execution, input::ComposerInput, inspector, sidebar, theme,
    window::open_settings_window,
};

pub struct AppShell {
    state: NormaAppState,
    updates: Receiver<RuntimeUpdate>,
    composer_input: Option<Entity<ComposerInput>>,
}

impl AppShell {
    pub fn new(state: NormaAppState, updates: Receiver<RuntimeUpdate>) -> Self {
        Self {
            state,
            updates,
            composer_input: None,
        }
    }
}

impl Render for AppShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while let Ok(update) = self.updates.try_recv() {
            self.state.apply_runtime_update(update);
        }

        if self.composer_input.is_none() {
            self.composer_input = Some(ComposerInput::new(
                cx,
                Some(Box::new(|content| {
                    tracing::info!(
                        component = "composer",
                        prompt_len = content.chars().count(),
                        "composer submit captured"
                    );
                })),
            ));
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
                            .child(execution::render_execution(
                                &self.state.session,
                                self.composer_input.as_ref(),
                            )),
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
            open_settings_window(
                cx,
                config.clone(),
                runtime_config.clone(),
                config_file.clone(),
            );
        })
}
