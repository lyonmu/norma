use std::path::PathBuf;

use crate::git::{ChangeKind, ChangedFile};
use crate::session::{ChecklistItem, ExecutionStep, SessionEvent, StepStatus};

pub trait AgentRuntime {
    fn run_mock_task(&self, task: &str) -> Vec<SessionEvent>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MockAgentRuntime;

impl AgentRuntime for MockAgentRuntime {
    fn run_mock_task(&self, task: &str) -> Vec<SessionEvent> {
        vec![
            SessionEvent::UserTask {
                content: task.to_string(),
            },
            SessionEvent::AgentPlan {
                goal: "完善 Norma 的整体设计、架构、模块、数据流、UI、Git 交互".to_string(),
                constraints: vec![
                    "不内置代码编辑器".to_string(),
                    "专注项目上下文、执行流、变更审查与回滚".to_string(),
                    "V1 不执行破坏性 Git 操作".to_string(),
                ],
            },
            SessionEvent::StepUpdated(completed_step(
                "读取 README",
                "读取并解析 README.md 与 README.zh.md，提取产品定位与目标。",
                "18.4s",
            )),
            SessionEvent::StepUpdated(completed_step(
                "确认产品边界",
                "确认 Norma 不包含代码编辑器，聚焦执行流与变更审查。",
                "15.7s",
            )),
            SessionEvent::StepUpdated(running_step()),
            SessionEvent::ChangeSummary {
                files: vec![
                    changed("src/ui/inspector.rs", 86, 10, 4),
                    changed("src/ui/execution_item.rs", 42, 3, 2),
                    changed("src/agent/runner.rs", 38, 2, 2),
                    changed("src/git/repository.rs", 29, 5, 2),
                    changed("src/config/settings.rs", 24, 0, 1),
                ],
            },
            SessionEvent::StepUpdated(waiting_step()),
        ]
    }
}

fn completed_step(title: &str, description: &str, duration: &str) -> ExecutionStep {
    ExecutionStep {
        title: title.to_string(),
        description: description.to_string(),
        status: StepStatus::Completed,
        duration_label: Some(duration.to_string()),
        checklist: Vec::new(),
    }
}

fn running_step() -> ExecutionStep {
    ExecutionStep {
        title: "生成 Codex 风格 UI".to_string(),
        description: "设计三栏工作台：线程侧栏、执行流、动态检查器。".to_string(),
        status: StepStatus::Running,
        duration_label: Some("32.1s".to_string()),
        checklist: vec![
            ChecklistItem {
                label: "分析 Codex 设计语言".to_string(),
                status: StepStatus::Completed,
            },
            ChecklistItem {
                label: "制定三栏布局与信息层级".to_string(),
                status: StepStatus::Completed,
            },
            ChecklistItem {
                label: "设计右侧检查器（Diff + Git）".to_string(),
                status: StepStatus::Completed,
            },
            ChecklistItem {
                label: "制作高保真界面草图".to_string(),
                status: StepStatus::Running,
            },
            ChecklistItem {
                label: "评审与优化".to_string(),
                status: StepStatus::Waiting,
            },
            ChecklistItem {
                label: "输出交互规范".to_string(),
                status: StepStatus::Waiting,
            },
        ],
    }
}

fn waiting_step() -> ExecutionStep {
    ExecutionStep {
        title: "检查变更摘要".to_string(),
        description: "汇总变更文件与行数，生成审查摘要。".to_string(),
        status: StepStatus::Waiting,
        duration_label: None,
        checklist: Vec::new(),
    }
}

fn changed(path: &str, added_lines: usize, deleted_lines: usize, hunk_count: usize) -> ChangedFile {
    ChangedFile {
        path: PathBuf::from(path),
        kind: ChangeKind::Modified,
        added_lines,
        deleted_lines,
        hunk_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_runtime_emits_visual_contract_states() {
        let runtime = MockAgentRuntime;
        let events = runtime.run_mock_task("完善 Norma 项目设计");

        assert!(matches!(
            events.first(),
            Some(SessionEvent::UserTask { .. })
        ));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, SessionEvent::AgentPlan { .. }))
        );
        assert!(events.iter().any(
            |event| matches!(event, SessionEvent::ChangeSummary { files } if files.len() == 5)
        ));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::StepUpdated(step) if step.status == StepStatus::Running && step.checklist.len() == 6)));
        assert!(events.iter().any(|event| matches!(event, SessionEvent::StepUpdated(step) if step.status == StepStatus::Waiting)));
    }
}
