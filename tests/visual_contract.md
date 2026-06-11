# Norma V1 Visual Verification Checklist

Date: 2026-06-11

Use this checklist when running `cargo run` and visually inspecting the GPUI window.
Compare against `docs/superpowers/specs/assets/norma-review-first-codex-workbench.png`.

## Window Shell

- [ ] Window opens near 1440x1024
- [ ] Light native desktop surface (not dark)
- [ ] White content surfaces on very light gray app background
- [ ] Thin hairline dividers between toolbar, columns, and sections

## Top Toolbar

- [ ] Norma logo/icon on the left
- [ ] Back/forward navigation buttons
- [ ] Model, runtime, and safety pills on the right
- [ ] Play, notification, and settings buttons on the far right

## Left Sidebar (320px)

- [ ] Project card with project name and path
- [ ] Thread list with "today" and "yesterday" groupings
- [ ] Active thread highlighted with blue tint
- [ ] File tree with directory/file icons and depth indentation
- [ ] Git status card at bottom with branch, ahead/behind, and change counts
- [ ] Git metrics use green for added, red for modified, muted for deleted

## Center Execution Stream

- [ ] Task header with thread title and "continue task" pill
- [ ] Task summary card with goal, constraints, and status rows
- [ ] Summary card has blue tinted border
- [ ] Step cards with status icons (✓ completed, ◉ running, ○ waiting, ! failed)
- [ ] Running steps have blue border accent
- [ ] Step checklist items shown with status icons
- [ ] Timeline line (left border) connecting events
- [ ] Message cards for final response (green border) and errors (red border)
- [ ] Composer at bottom with placeholder text and action pills

## Right Inspector (410px)

- [ ] Tab bar with: 检查器, 上下文, 输出, 设置
- [ ] Active tab has blue bottom border and bold text
- [ ] Inactive tabs have transparent border and muted text
- [ ] Metric tiles grid (4 columns): 变更文件, 新增行, 删除行, 信心度
- [ ] Green accent for added lines, red for deleted lines, blue for confidence
- [ ] Safety check row with green background and "通过" status
- [ ] Changed files list with path, added/deleted line counts
- [ ] Selected file has tinted background
- [ ] File preview section with hunk summary and diff stats
- [ ] Git operations section with disabled action cards (opacity ~0.65)
- [ ] Disabled actions: 撤销, 丢弃, 外部编辑器

## Visual Qualities

- [ ] 13-15px typography with clear hierarchy
- [ ] 8-12px corner radius on cards and controls
- [ ] Blue accent for active/selected states
- [ ] Green for completed/safe/added states
- [ ] Red for deleted/destructive states
- [ ] Minimal shadows; prefer spacing and dividers
- [ ] No decorative illustration or gradient blobs
