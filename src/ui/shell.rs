use std::sync::mpsc::Receiver;

use gpui::{
    AnyElement, App, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div, prelude::*, px,
};

use crate::app::NormaAppState;
use crate::runtime::RuntimeUpdate;
use crate::ui::{
    components, execution,
    input::ComposerInput,
    inspector, sidebar, theme,
    window::{WindowSizeClass, WorkbenchLayout, open_settings_window},
};

pub struct AppShell {
    state: NormaAppState,
    updates: Receiver<RuntimeUpdate>,
    composer_input: Option<Entity<ComposerInput>>,
    last_size_class: Option<WindowSizeClass>,
    sidebar_drawer_open: bool,
    inspector_drawer_open: bool,
}

impl AppShell {
    pub fn new(state: NormaAppState, updates: Receiver<RuntimeUpdate>) -> Self {
        Self {
            state,
            updates,
            composer_input: None,
            last_size_class: None,
            sidebar_drawer_open: false,
            inspector_drawer_open: false,
        }
    }

    fn apply_size_class(&mut self, next: WindowSizeClass) {
        if self.last_size_class != Some(next) {
            self.sidebar_drawer_open = false;
            self.inspector_drawer_open = false;
            self.last_size_class = Some(next);
        }
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        while let Ok(update) = self.updates.try_recv() {
            self.state.apply_runtime_update(update);
        }

        let layout = WorkbenchLayout::for_width(window.bounds().size.width);
        self.apply_size_class(layout.size_class);

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

        let inline_sidebar = layout.sidebar_inline.then(|| sidebar_panel(&self.state));
        let inline_inspector = layout
            .inspector_inline
            .then(|| inspector_panel(&self.state));
        let sidebar_overlay = (!layout.sidebar_inline && self.sidebar_drawer_open)
            .then(|| sidebar_drawer(&self.state));
        let inspector_overlay = (!layout.inspector_inline && self.inspector_drawer_open)
            .then(|| inspector_drawer(&self.state));
        let center_padding = if layout.size_class == WindowSizeClass::Compact {
            px(16.)
        } else {
            px(24.)
        };

        div()
            .relative()
            .size_full()
            .bg(theme::app_bg())
            .text_color(theme::text())
            .flex()
            .flex_col()
            .child(top_toolbar(&self.state, layout, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_hidden()
                    .children(inline_sidebar)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .h_full()
                            .p(center_padding)
                            .child(execution::render_execution(
                                &self.state.session,
                                self.composer_input.as_ref(),
                            )),
                    )
                    .children(inline_inspector),
            )
            .children(sidebar_overlay)
            .children(inspector_overlay)
    }
}

fn toolbar_action(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(SharedString::from(id))
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
        .child(label)
        .on_click(on_click)
        .into_any_element()
}

fn top_toolbar(
    state: &NormaAppState,
    layout: WorkbenchLayout,
    cx: &mut Context<AppShell>,
) -> AnyElement {
    let sidebar_toggle = (!layout.sidebar_inline).then(|| {
        toolbar_action(
            "toggle-sidebar",
            "☰",
            cx.listener(|shell, _, _, cx| {
                shell.sidebar_drawer_open = !shell.sidebar_drawer_open;
                shell.inspector_drawer_open = false;
                cx.notify();
            }),
        )
    });
    let inspector_toggle = (!layout.inspector_inline).then(|| {
        toolbar_action(
            "toggle-inspector",
            "检查",
            cx.listener(|shell, _, _, cx| {
                shell.inspector_drawer_open = !shell.inspector_drawer_open;
                shell.sidebar_drawer_open = false;
                cx.notify();
            }),
        )
    });
    let status_pills = layout.show_status_pills.then(|| {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(components::pill("模型 GPT-4.1", false))
            .child(components::pill("运行环境 本地", false))
            .child(components::pill("安全级别 标准", false))
    });

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
                .children(sidebar_toggle)
                .child(components::icon_button("N"))
                .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child("Norma"))
                .child(components::icon_button("←"))
                .child(components::icon_button("→")),
        )
        .children(status_pills)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(inspector_toggle)
                .child(components::icon_button("▶"))
                .child(components::icon_button("🔔"))
                .child(settings_button(state)),
        )
        .into_any_element()
}

fn sidebar_panel(state: &NormaAppState) -> AnyElement {
    div()
        .w(theme::SIDEBAR_WIDTH)
        .h_full()
        .border_r_1()
        .border_color(theme::border())
        .child(sidebar::render_sidebar(state))
        .into_any_element()
}

fn inspector_panel(state: &NormaAppState) -> AnyElement {
    div()
        .w(theme::INSPECTOR_WIDTH)
        .h_full()
        .border_l_1()
        .border_color(theme::border())
        .child(inspector::render_inspector(state))
        .into_any_element()
}

fn sidebar_drawer(state: &NormaAppState) -> AnyElement {
    div()
        .id(SharedString::from("sidebar-drawer"))
        .absolute()
        .top(theme::TOOLBAR_HEIGHT)
        .bottom(px(0.))
        .left(px(0.))
        .w(theme::SIDEBAR_WIDTH)
        .border_r_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .child(sidebar::render_sidebar(state))
        .into_any_element()
}

fn inspector_drawer(state: &NormaAppState) -> AnyElement {
    div()
        .id(SharedString::from("inspector-drawer"))
        .absolute()
        .top(theme::TOOLBAR_HEIGHT)
        .bottom(px(0.))
        .right(px(0.))
        .w(theme::INSPECTOR_WIDTH)
        .border_l_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .child(inspector::render_inspector(state))
        .into_any_element()
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

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use gpui::{AppContext, TestAppContext, size};

    use super::*;

    fn shell() -> AppShell {
        let (_updates_tx, updates_rx) = mpsc::channel();
        AppShell::new(NormaAppState::no_project(), updates_rx)
    }

    #[test]
    fn changing_size_class_closes_open_drawers() {
        let mut shell = shell();
        shell.last_size_class = Some(WindowSizeClass::Compact);
        shell.sidebar_drawer_open = true;
        shell.inspector_drawer_open = true;

        shell.apply_size_class(WindowSizeClass::Wide);

        assert_eq!(shell.last_size_class, Some(WindowSizeClass::Wide));
        assert!(!shell.sidebar_drawer_open);
        assert!(!shell.inspector_drawer_open);
    }

    #[test]
    fn keeping_size_class_preserves_open_drawer() {
        let mut shell = shell();
        shell.last_size_class = Some(WindowSizeClass::Compact);
        shell.sidebar_drawer_open = true;

        shell.apply_size_class(WindowSizeClass::Compact);

        assert!(shell.sidebar_drawer_open);
    }

    #[gpui::test]
    fn resizing_the_window_recomputes_the_shell_size_class(cx: &mut TestAppContext) {
        let (_updates_tx, updates_rx) = mpsc::channel();
        let (shell, cx) =
            cx.add_window_view(|_, _| AppShell::new(NormaAppState::no_project(), updates_rx));

        cx.simulate_resize(size(px(1024.), px(700.)));
        cx.run_until_parked();
        assert_eq!(
            cx.read_entity(&shell, |shell, _| shell.last_size_class),
            Some(WindowSizeClass::Compact)
        );

        cx.simulate_resize(size(px(1280.), px(800.)));
        cx.run_until_parked();
        assert_eq!(
            cx.read_entity(&shell, |shell, _| shell.last_size_class),
            Some(WindowSizeClass::Wide)
        );
    }
}
