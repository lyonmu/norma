use gpui::{AnyElement, Entity, InteractiveElement, ParentElement, Styled, div, prelude::*, px};

use crate::session::{ExecutionStep, SessionEvent, SessionState, StepStatus};
use crate::ui::{components, input::ComposerInput, theme};

pub fn render_execution(
    session: &SessionState,
    composer: Option<&Entity<ComposerInput>>,
) -> AnyElement {
    div()
        .size_full()
        .min_h(px(0.))
        .flex()
        .flex_col()
        .gap_4()
        .child(task_header(session))
        .child(task_summary(session))
        .child(
            div()
                .id("execution-scroll")
                .flex_1()
                .min_h(px(0.))
                .overflow_y_scroll()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .border_l_1()
                        .border_color(theme::border())
                        .pl_4()
                        .children(session.events.iter().filter_map(render_event)),
                ),
        )
        .children(composer.cloned())
        .into_any_element()
}

fn task_header(session: &SessionState) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(18.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(session.thread.title.clone())
                .child(" ✎"),
        )
        .child(components::pill("继续任务", false))
        .into_any_element()
}

fn task_summary(session: &SessionState) -> AnyElement {
    let mut goal = "完善 Norma 的整体设计、架构、模块、数据流、UI、Git 交互".to_string();
    let mut constraints = "不内置代码编辑器，专注项目上下文、执行流、变更审查与回滚".to_string();
    for event in &session.events {
        if let SessionEvent::AgentPlan {
            goal: event_goal,
            constraints: event_constraints,
        } = event
        {
            goal = event_goal.clone();
            constraints = event_constraints.join("，");
        }
    }
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(gpui::hsla(218. / 360., 0.8, 0.88, 1.))
        .bg(theme::surface_tint())
        .p_4()
        .flex()
        .flex_col()
        .gap_2()
        .child(row("目标", goal))
        .child(row("约束", constraints))
        .child(row("状态", "进行中"))
        .into_any_element()
}

fn render_event(event: &SessionEvent) -> Option<AnyElement> {
    match event {
        SessionEvent::StepUpdated(step) => Some(step_card(step)),
        SessionEvent::FinalResponse { content } => {
            Some(message_card("完成", content, theme::green()))
        }
        SessionEvent::Error { message } => Some(message_card("需要确认", message, theme::red())),
        SessionEvent::UserTask { .. }
        | SessionEvent::AgentPlan { .. }
        | SessionEvent::ChangeSummary { .. } => None,
    }
}

fn step_card(step: &ExecutionStep) -> AnyElement {
    let color = match step.status {
        StepStatus::Completed => theme::green(),
        StepStatus::Running => theme::blue(),
        StepStatus::Waiting => theme::muted(),
        StepStatus::Failed => theme::red(),
    };
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(if step.status == StepStatus::Running {
            theme::blue()
        } else {
            theme::border()
        })
        .bg(theme::surface())
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(div().text_color(color).child(status_icon(&step.status)))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(step.title.clone()),
                        ),
                )
                .child(components::label(
                    step.duration_label
                        .clone()
                        .unwrap_or_else(|| "等待中".to_string()),
                )),
        )
        .child(components::label(step.description.clone()))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .children(step.checklist.iter().map(|item| {
                    div()
                        .flex()
                        .gap_2()
                        .text_size(px(13.))
                        .child(status_icon(&item.status))
                        .child(item.label.clone())
                })),
        )
        .into_any_element()
}

fn message_card(title: &str, content: &str, color: gpui::Hsla) -> AnyElement {
    div()
        .rounded(px(10.))
        .border_1()
        .border_color(color)
        .bg(theme::surface())
        .p_4()
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(title.to_string()),
        )
        .child(components::label(content.to_string()))
        .into_any_element()
}

fn row(label: &str, value: impl Into<String>) -> AnyElement {
    div()
        .flex()
        .gap_2()
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(format!("{label}:")),
        )
        .child(value.into())
        .into_any_element()
}

fn status_icon(status: &StepStatus) -> &'static str {
    match status {
        StepStatus::Completed => "✓",
        StepStatus::Running => "◉",
        StepStatus::Waiting => "○",
        StepStatus::Failed => "!",
    }
}
