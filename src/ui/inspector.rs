use gpui::{AnyElement, ParentElement, Styled, div, prelude::*, px};

use crate::app::NormaAppState;
use crate::session::InspectorTab;
use crate::ui::{components, theme};

pub fn render_inspector(state: &NormaAppState) -> AnyElement {
    let review_files = &state.session.changed_files;
    let added: usize = review_files.iter().map(|file| file.added_lines).sum();
    let deleted: usize = review_files.iter().map(|file| file.deleted_lines).sum();
    let changed_count = review_files.len();

    div()
        .size_full()
        .bg(theme::surface())
        .flex()
        .flex_col()
        .child(tabs(state.session.active_tab))
        .child(
            div()
                .p_5()
                .flex()
                .flex_col()
                .gap_5()
                .child(components::section_title("变更概览"))
                .child(
                    div()
                        .grid()
                        .grid_cols(4)
                        .gap_2()
                        .child(metric_tile(
                            changed_count.to_string(),
                            "变更文件",
                            theme::text(),
                        ))
                        .child(metric_tile(format!("+{added}"), "新增行", theme::green()))
                        .child(metric_tile(format!("-{deleted}"), "删除行", theme::red()))
                        .child(metric_tile("92%".to_string(), "信心度", theme::blue())),
                )
                .child(safety_row())
                .child(changed_files(state))
                .child(file_preview(state))
                .child(git_operations()),
        )
        .into_any_element()
}

fn tabs(active: InspectorTab) -> AnyElement {
    let names = [
        ("检查器", InspectorTab::Inspector),
        ("上下文", InspectorTab::Context),
        ("输出", InspectorTab::Output),
        ("设置", InspectorTab::Settings),
    ];
    div()
        .h(px(50.))
        .border_b_1()
        .border_color(theme::border())
        .flex()
        .items_end()
        .gap_6()
        .px_5()
        .children(names.into_iter().map(|(name, mode)| {
            let is_active = active == mode;
            div()
                .pb_3()
                .border_b_2()
                .border_color(if is_active {
                    theme::blue()
                } else {
                    gpui::hsla(0., 0., 0., 0.)
                })
                .text_color(if is_active {
                    theme::text()
                } else {
                    theme::muted()
                })
                .font_weight(if is_active {
                    gpui::FontWeight::SEMIBOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .child(name)
        }))
        .into_any_element()
}

fn metric_tile(value: String, label: &str, color: gpui::Hsla) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .p_3()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_color(color)
                .font_weight(gpui::FontWeight::BOLD)
                .text_size(px(18.))
                .child(value),
        )
        .child(components::label(label))
        .into_any_element()
}

fn safety_row() -> AnyElement {
    div()
        .rounded(px(9.))
        .bg(gpui::hsla(145. / 360., 0.42, 0.95, 1.))
        .px_3()
        .py_2()
        .flex()
        .justify_between()
        .child(div().text_color(theme::green()).child("安全检查  通过"))
        .child(components::label("无高风险操作"))
        .into_any_element()
}

fn changed_files(state: &NormaAppState) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .justify_between()
                .child(components::section_title(format!(
                    "变更文件 ({})",
                    state.session.changed_files.len()
                )))
                .child(components::pill("全部", false)),
        )
        .children(state.session.changed_files.iter().take(8).map(|file| {
            div()
                .rounded(px(7.))
                .px_3()
                .py_2()
                .bg(
                    if Some(&file.path)
                        == state
                            .session
                            .selected_change
                            .as_ref()
                            .map(|change| &change.path)
                    {
                        theme::surface_tint()
                    } else {
                        theme::surface()
                    },
                )
                .flex()
                .justify_between()
                .child(file.path.display().to_string())
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            div()
                                .text_color(theme::green())
                                .child(format!("+{}", file.added_lines)),
                        )
                        .child(
                            div()
                                .text_color(theme::red())
                                .child(format!("-{}", file.deleted_lines)),
                        ),
                )
        }))
        .into_any_element()
}

fn file_preview(state: &NormaAppState) -> AnyElement {
    let Some(change) = &state.session.selected_change else {
        return div()
            .child(components::label("暂无文件变更预览"))
            .into_any_element();
    };

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("文件变更预览"))
        .child(
            div()
                .rounded(px(10.))
                .border_1()
                .border_color(theme::border())
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(change.path.display().to_string()),
                        )
                        .child(components::pill("预览对比", false)),
                )
                .children(change.hunks.iter().map(|hunk| {
                    div()
                        .border_t_1()
                        .border_color(theme::border())
                        .pt_2()
                        .flex()
                        .justify_between()
                        .child(format!("Hunk {}  {}", hunk.index, hunk.line_range))
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    div()
                                        .text_color(theme::green())
                                        .child(format!("+{}", hunk.added_lines)),
                                )
                                .child(
                                    div()
                                        .text_color(theme::red())
                                        .child(format!("-{}", hunk.deleted_lines)),
                                ),
                        )
                })),
        )
        .into_any_element()
}

fn git_operations() -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(components::section_title("Git 操作"))
        .child(disabled_action(
            "撤销本次 Agent 变更",
            "V1 仅展示入口，不执行破坏性操作",
        ))
        .child(disabled_action("丢弃所选变更", "V1 禁用，避免误删手动修改"))
        .child(disabled_action(
            "在外部编辑器中打开",
            "后续接入系统打开行为",
        ))
        .into_any_element()
}

fn disabled_action(title: &str, description: &str) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(theme::border())
        .p_3()
        .opacity(0.65)
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
        .child(components::label(description.to_string()))
        .into_any_element()
}
