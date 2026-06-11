use gpui::{AnyElement, ParentElement, Styled, div, prelude::*, px};

use crate::app::NormaAppState;
use crate::ui::{components, theme};
use crate::workspace::FileKind;

pub fn render_sidebar(state: &NormaAppState) -> AnyElement {
    div()
        .size_full()
        .bg(theme::surface())
        .p_4()
        .flex()
        .flex_col()
        .gap_5()
        .child(project_card(state))
        .child(thread_list(state))
        .child(file_tree(state))
        .child(git_card(state))
        .into_any_element()
}

fn project_card(state: &NormaAppState) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .p_3()
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(state.project_name()),
        )
        .child(components::label(state.project_path_label()))
        .into_any_element()
}

fn thread_list(state: &NormaAppState) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("线程"))
        .child(components::label("今天"))
        .child(
            div()
                .rounded(px(8.))
                .bg(theme::surface_tint())
                .px_3()
                .py_2()
                .flex()
                .justify_between()
                .child(state.session.thread.title.clone())
                .child(components::label(
                    state.session.thread.updated_label.clone(),
                )),
        )
        .child(components::label("昨天"))
        .child(components::label("搭建 GPUI 窗口框架"))
        .child(components::label("接入配置管理模块"))
        .into_any_element()
}

fn file_tree(state: &NormaAppState) -> AnyElement {
    let rows = state.files.iter().take(18).map(|node| {
        let indent = px((node.depth as f32) * 14.);
        let icon = match node.kind {
            FileKind::Directory => "▸",
            FileKind::File => "◇",
        };
        div()
            .pl(indent)
            .h(px(24.))
            .flex()
            .items_center()
            .gap_2()
            .text_size(px(13.))
            .child(icon)
            .child(node.name.clone())
    });

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("项目文件"))
        .children(rows)
        .into_any_element()
}

fn git_card(state: &NormaAppState) -> AnyElement {
    let branch = state
        .git
        .branch
        .clone()
        .unwrap_or_else(|| "非 Git 仓库".to_string());
    div()
        .mt_auto()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("Git 状态"))
        .child(components::label(branch))
        .child(
            div()
                .flex()
                .gap_3()
                .child(metric("已修改", state.git.modified, theme::red()))
                .child(metric(
                    "已添加",
                    state.git.added + state.git.untracked,
                    theme::green(),
                ))
                .child(metric("已删除", state.git.deleted, theme::muted())),
        )
        .child(components::label(format!(
            "↑ {} ahead / ↓ {} behind",
            state.git.ahead, state.git.behind
        )))
        .into_any_element()
}

fn metric(label: &str, value: usize, color: gpui::Hsla) -> AnyElement {
    div()
        .flex()
        .gap_1()
        .text_size(px(12.))
        .child(div().text_color(color).child(value.to_string()))
        .child(components::label(label))
        .into_any_element()
}
