# Norma 响应式窗口阶段 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Norma 主窗口和设置窗口使用集中式 GPUI 窗口策略，在 `1024×700` 的主窗口下通过三档布局、覆盖抽屉和独立滚动区域保持可用。

**Architecture:** 新增 `ui/window` 作为 GPUI 窗口创建和尺寸策略边界；`WindowPolicy` 负责配置尺寸钳制、最小尺寸和宽度档位，`AppShell` 只消费布局 projection。阶段 1 保留现有 `NormaAppState`、`RuntimeUpdate` 和设置状态实现，不提前引入 AppModel 或异步 Runtime Bridge。

**Tech Stack:** Rust 2024、GPUI 0.2.2、Cargo、rustfmt、Clippy、Rust unit tests、GPUI `TestAppContext`

---

## 范围边界

本计划只实现已批准设计的阶段 1：

- 主窗口初始尺寸读取 `NormaConfig.window`。
- 主窗口最小尺寸为 `1024×700`。
- 设置窗口初始尺寸为 `960×720`，最小尺寸为 `840×620`。
- Workbench 支持 Wide、Medium、Compact 三档布局。
- 窄窗口使用覆盖抽屉。
- Sidebar、Execution、Inspector 和 Settings 内容拥有明确滚动边界。
- 更新人工视觉验证契约。

本计划不实现：

- 共享 `Entity<AppModel>`。
- Runtime event 异步 bridge。
- SettingsDraft/Controller 拆分。
- Provider 异步化。
- `ui/workbench/` 和 `ui/settings/` 的最终目录迁移。

这些内容分别属于后续阶段，不能混入本阶段提交。

## 文件结构

### 新建

- `src/ui/window/mod.rs`：导出窗口 host 和 policy。
- `src/ui/window/policy.rs`：窗口尺寸、钳制、宽度档位和布局 projection。
- `src/ui/window/host.rs`：主窗口和设置窗口的 GPUI 创建逻辑。

### 修改

- `src/ui/mod.rs`：注册 `window` 模块。
- `src/main.rs`：从 `ui::window::run` 启动应用。
- `src/ui/shell.rs`：消费 `WorkbenchLayout`，增加抽屉状态和紧凑工具栏。
- `src/ui/sidebar.rs`：增加 sidebar 内容滚动边界。
- `src/ui/execution.rs`：让事件流滚动并固定 Composer。
- `src/ui/inspector.rs`：让 Inspector 主内容独立滚动。
- `src/ui/settings.rs`：按设置窗口宽度切换横向/纵向 Provider 布局。
- `tests/visual_contract.md`：增加主窗口尺寸档位检查。
- `tests/settings_visual_contract.md`：增加设置窗口最小尺寸检查。

## Task 1：建立 WindowPolicy 和纯布局 projection

**Files:**

- Create: `src/ui/window/mod.rs`
- Create: `src/ui/window/policy.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1：注册模块并写入失败测试**

在 `src/ui/mod.rs` 增加：

```rust
pub mod window;
```

创建 `src/ui/window/mod.rs`：

```rust
mod policy;

pub use policy::{
    SettingsSizeClass, WindowPolicy, WindowSizeClass, WorkbenchLayout,
};
```

先创建只包含测试和必要 import 的 `src/ui/window/policy.rs`：

```rust
use gpui::{Bounds, Pixels, Size, WindowBounds, WindowOptions, point, px, size};

use crate::config::WindowConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_size_clamps_each_dimension_to_supported_minimum() {
        let config = WindowConfig {
            width: 900,
            height: 680,
        };

        assert_eq!(
            WindowPolicy::main_content_size(Some(&config)),
            size(px(1024.), px(700.))
        );
    }

    #[test]
    fn main_size_preserves_valid_configured_dimensions() {
        let config = WindowConfig {
            width: 1600,
            height: 1000,
        };

        assert_eq!(
            WindowPolicy::main_content_size(Some(&config)),
            size(px(1600.), px(1000.))
        );
    }

    #[test]
    fn workbench_breakpoints_match_the_visual_contract() {
        assert_eq!(
            WorkbenchLayout::for_width(px(1024.)).size_class,
            WindowSizeClass::Compact
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1119.)).size_class,
            WindowSizeClass::Compact
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1120.)).size_class,
            WindowSizeClass::Medium
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1279.)).size_class,
            WindowSizeClass::Medium
        );
        assert_eq!(
            WorkbenchLayout::for_width(px(1280.)).size_class,
            WindowSizeClass::Wide
        );
    }

    #[test]
    fn workbench_projection_controls_inline_panels_and_status_pills() {
        let compact = WorkbenchLayout::for_width(px(1024.));
        assert!(!compact.sidebar_inline);
        assert!(!compact.inspector_inline);
        assert!(!compact.show_status_pills);

        let medium = WorkbenchLayout::for_width(px(1120.));
        assert!(medium.sidebar_inline);
        assert!(!medium.inspector_inline);
        assert!(!medium.show_status_pills);

        let wide = WorkbenchLayout::for_width(px(1280.));
        assert!(wide.sidebar_inline);
        assert!(wide.inspector_inline);
        assert!(wide.show_status_pills);
    }

    #[test]
    fn settings_switches_to_stacked_content_below_920_pixels() {
        assert_eq!(
            WindowPolicy::settings_size_class(px(919.)),
            SettingsSizeClass::Stacked
        );
        assert_eq!(
            WindowPolicy::settings_size_class(px(920.)),
            SettingsSizeClass::Inline
        );
    }

    #[test]
    fn window_options_expose_product_minimum_sizes() {
        let main = WindowPolicy::main_window_options(None);
        let settings = WindowPolicy::settings_window_options();

        assert_eq!(main.window_min_size, Some(size(px(1024.), px(700.))));
        assert_eq!(
            settings.window_min_size,
            Some(size(px(840.), px(620.)))
        );
    }
}
```

此时测试引用的 policy 类型尚不存在，编译必须失败。

- [ ] **Step 2：运行测试并确认 RED**

Run:

```bash
cargo test ui::window::policy::tests
```

Expected: FAIL，错误包含 `cannot find type WindowPolicy`。

- [ ] **Step 3：实现最小 WindowPolicy**

在 `src/ui/window/policy.rs` 的测试模块前加入：

```rust
const MAIN_DEFAULT_WIDTH: f32 = 1440.;
const MAIN_DEFAULT_HEIGHT: f32 = 1024.;
const MAIN_MIN_WIDTH: f32 = 1024.;
const MAIN_MIN_HEIGHT: f32 = 700.;
const SETTINGS_WIDTH: f32 = 960.;
const SETTINGS_HEIGHT: f32 = 720.;
const SETTINGS_MIN_WIDTH: f32 = 840.;
const SETTINGS_MIN_HEIGHT: f32 = 620.;
const MEDIUM_BREAKPOINT: f32 = 1120.;
const WIDE_BREAKPOINT: f32 = 1280.;
const SETTINGS_INLINE_BREAKPOINT: f32 = 920.;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowSizeClass {
    Compact,
    Medium,
    Wide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsSizeClass {
    Stacked,
    Inline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkbenchLayout {
    pub size_class: WindowSizeClass,
    pub sidebar_inline: bool,
    pub inspector_inline: bool,
    pub show_status_pills: bool,
}

impl WorkbenchLayout {
    pub fn for_width(width: Pixels) -> Self {
        if width >= px(WIDE_BREAKPOINT) {
            Self {
                size_class: WindowSizeClass::Wide,
                sidebar_inline: true,
                inspector_inline: true,
                show_status_pills: true,
            }
        } else if width >= px(MEDIUM_BREAKPOINT) {
            Self {
                size_class: WindowSizeClass::Medium,
                sidebar_inline: true,
                inspector_inline: false,
                show_status_pills: false,
            }
        } else {
            Self {
                size_class: WindowSizeClass::Compact,
                sidebar_inline: false,
                inspector_inline: false,
                show_status_pills: false,
            }
        }
    }
}

pub struct WindowPolicy;

impl WindowPolicy {
    pub fn main_content_size(config: Option<&WindowConfig>) -> Size<Pixels> {
        let width = config
            .map(|config| config.width as f32)
            .unwrap_or(MAIN_DEFAULT_WIDTH)
            .max(MAIN_MIN_WIDTH);
        let height = config
            .map(|config| config.height as f32)
            .unwrap_or(MAIN_DEFAULT_HEIGHT)
            .max(MAIN_MIN_HEIGHT);
        size(px(width), px(height))
    }

    pub fn main_window_options(config: Option<&WindowConfig>) -> WindowOptions {
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(80.), px(80.)),
                Self::main_content_size(config),
            ))),
            window_min_size: Some(size(px(MAIN_MIN_WIDTH), px(MAIN_MIN_HEIGHT))),
            ..WindowOptions::default()
        }
    }

    pub fn settings_window_options() -> WindowOptions {
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(180.), px(120.)),
                size(px(SETTINGS_WIDTH), px(SETTINGS_HEIGHT)),
            ))),
            window_min_size: Some(size(
                px(SETTINGS_MIN_WIDTH),
                px(SETTINGS_MIN_HEIGHT),
            )),
            ..WindowOptions::default()
        }
    }

    pub fn settings_size_class(width: Pixels) -> SettingsSizeClass {
        if width < px(SETTINGS_INLINE_BREAKPOINT) {
            SettingsSizeClass::Stacked
        } else {
            SettingsSizeClass::Inline
        }
    }
}
```

- [ ] **Step 4：运行测试并确认 GREEN**

Run:

```bash
cargo test ui::window::policy::tests
```

Expected: 6 tests PASS。

- [ ] **Step 5：格式化并提交**

Run:

```bash
cargo fmt
git add src/ui/mod.rs src/ui/window/mod.rs src/ui/window/policy.rs
git commit -m "feat(ui): 添加窗口尺寸策略"
```

Expected: commit succeeds；不得暂存 `docs/superpowers/`。

## Task 2：集中主窗口和设置窗口创建逻辑

**Files:**

- Create: `src/ui/window/host.rs`
- Modify: `src/ui/window/mod.rs`
- Modify: `src/ui/shell.rs:1-12,133-186`
- Modify: `src/main.rs:1-5`

- [ ] **Step 1：补充 options 与配置接线测试**

在 `src/ui/window/policy.rs` 的测试模块加入：

```rust
#[test]
fn main_options_use_the_configured_window_size() {
    let config = WindowConfig {
        width: 1500,
        height: 900,
    };
    let options = WindowPolicy::main_window_options(Some(&config));

    assert_eq!(
        options.window_bounds.unwrap().get_bounds().size,
        size(px(1500.), px(900.))
    );
}

#[test]
fn settings_options_keep_the_expected_initial_size() {
    let options = WindowPolicy::settings_window_options();

    assert_eq!(
        options.window_bounds.unwrap().get_bounds().size,
        size(px(960.), px(720.))
    );
}
```

- [ ] **Step 2：运行新增测试**

Run:

```bash
cargo test ui::window::policy::tests
```

Expected: 8 tests PASS。该测试保护 Task 2 要消费的窗口 options contract。

- [ ] **Step 3：实现 window host**

先在 `src/ui/window/mod.rs` 注册并导出 host：

```rust
mod host;
mod policy;

pub use host::{open_settings_window, run};
pub use policy::{
    SettingsSizeClass, WindowPolicy, WindowSizeClass, WorkbenchLayout,
};
```

创建 `src/ui/window/host.rs`：

```rust
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use gpui::{App, Application};

use crate::app::NormaAppState;
use crate::config::{AppConfig, NormaConfig};
use crate::runtime::RuntimeUpdate;
use crate::ui::settings::SettingsWindow;
use crate::ui::shell::AppShell;

use super::WindowPolicy;

pub fn open_settings_window(
    cx: &mut App,
    config: AppConfig,
    runtime_config: Option<NormaConfig>,
    config_file: Option<PathBuf>,
) {
    let result = cx.open_window(WindowPolicy::settings_window_options(), move |_, cx| {
        cx.new(|_| SettingsWindow::new(config, runtime_config, config_file))
    });

    if let Err(error) = result {
        tracing::error!(
            component = "ui",
            error = %error,
            "failed to open Norma settings window"
        );
    }
}

pub fn run(state: NormaAppState, updates: Receiver<RuntimeUpdate>) {
    let window_config = state
        .runtime_config
        .as_ref()
        .map(|config| config.window.clone());

    Application::new().run(move |cx: &mut App| {
        let options = WindowPolicy::main_window_options(window_config.as_ref());
        let result =
            cx.open_window(options, |_, cx| cx.new(|_| AppShell::new(state, updates)));

        if let Err(error) = result {
            tracing::error!(
                component = "ui",
                error = %error,
                "failed to open Norma window"
            );
        }
    });
}
```

- [ ] **Step 4：让 shell 只请求打开设置窗口**

在 `src/ui/shell.rs`：

1. 删除 `Application`、`Bounds`、`WindowBounds`、`WindowOptions`、`point` 和 `size` import。
2. 删除文件底部原有 `run`。
3. 将 `SettingsWindow` import 替换为 `window::open_settings_window`。
4. 将 `settings_button` 的点击闭包替换为：

```rust
.on_click(move |_, _, cx| {
    open_settings_window(
        cx,
        config.clone(),
        runtime_config.clone(),
        config_file.clone(),
    );
})
```

完整的 UI import 应包含：

```rust
use crate::ui::{
    components, execution, input::ComposerInput, inspector, sidebar, theme,
    window::open_settings_window,
};
```

- [ ] **Step 5：更新 main 入口**

将 `src/main.rs` 改为：

```rust
fn main() -> anyhow::Result<()> {
    let runtime = norma::runtime::bootstrap()?;
    norma::ui::window::run(runtime.app_state, runtime.updates);
    Ok(())
}
```

- [ ] **Step 6：验证编译和窗口策略测试**

Run:

```bash
cargo fmt --check
cargo test ui::window::policy::tests
cargo check
```

Expected: all commands exit 0。

- [ ] **Step 7：提交**

```bash
git add src/main.rs src/ui/shell.rs src/ui/window/mod.rs src/ui/window/host.rs src/ui/window/policy.rs
git commit -m "refactor(ui): 集中窗口创建逻辑"
```

Expected: commit succeeds。

## Task 3：实现 Workbench 三档布局和覆盖抽屉

**Files:**

- Modify: `src/ui/shell.rs:15-131`
- Test: `src/ui/shell.rs` module tests

- [ ] **Step 1：写入抽屉状态转换失败测试**

在 `src/ui/shell.rs` 底部加入：

```rust
#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn shell() -> AppShell {
        let (_updates_tx, updates_rx) = mpsc::channel();
        AppShell::new(NormaAppState::no_project(), updates_rx)
    }

    #[test]
    fn moving_to_a_new_size_class_closes_overlay_drawers() {
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
    fn staying_in_the_same_size_class_preserves_open_drawer() {
        let mut shell = shell();
        shell.last_size_class = Some(WindowSizeClass::Compact);
        shell.sidebar_drawer_open = true;

        shell.apply_size_class(WindowSizeClass::Compact);

        assert!(shell.sidebar_drawer_open);
    }
}
```

- [ ] **Step 2：运行测试并确认 RED**

Run:

```bash
cargo test ui::shell::tests
```

Expected: FAIL，错误包含缺失字段 `last_size_class` 或缺失方法 `apply_size_class`。

- [ ] **Step 3：实现 AppShell 响应式状态**

在 `src/ui/shell.rs` import `WindowSizeClass` 和 `WorkbenchLayout`：

```rust
use crate::ui::window::{
    WindowSizeClass, WorkbenchLayout, open_settings_window,
};
```

将 `AppShell` 和构造器更新为：

```rust
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
```

- [ ] **Step 4：运行状态测试并确认 GREEN**

Run:

```bash
cargo test ui::shell::tests
```

Expected: 2 tests PASS。

- [ ] **Step 5：增加可点击 toolbar action**

在 `src/ui/shell.rs` 增加：

```rust
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
```

确保 import 包含 `AnyElement` 和 `InteractiveElement`。

- [ ] **Step 6：让 toolbar 按档位降级**

将 `top_toolbar` 签名改为：

```rust
fn top_toolbar(
    state: &NormaAppState,
    layout: WorkbenchLayout,
    cx: &mut Context<AppShell>,
) -> AnyElement
```

在左侧按钮组最前面，仅当 sidebar 非内联时加入：

```rust
let sidebar_toggle = (!layout.sidebar_inline).then(|| {
    toolbar_action(
        "toggle-sidebar",
        "☰",
        cx.listener(|this, _, _, cx| {
            this.sidebar_drawer_open = !this.sidebar_drawer_open;
            this.inspector_drawer_open = false;
            cx.notify();
        }),
    )
});
```

中间状态组仅当 `layout.show_status_pills` 为 `true` 时加入三个 pill：

```rust
let status_pills = layout.show_status_pills.then(|| {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(components::pill("模型 GPT-4.1", false))
        .child(components::pill("运行环境 本地", false))
        .child(components::pill("安全级别 标准", false))
});
```

右侧按钮组仅当 Inspector 非内联时加入：

```rust
let inspector_toggle = (!layout.inspector_inline).then(|| {
    toolbar_action(
        "toggle-inspector",
        "检查",
        cx.listener(|this, _, _, cx| {
            this.inspector_drawer_open = !this.inspector_drawer_open;
            this.sidebar_drawer_open = false;
            cx.notify();
        }),
    )
});
```

使用 `.children(sidebar_toggle)`、`.children(status_pills)` 和 `.children(inspector_toggle)` 组装 toolbar，保留原有播放、通知和设置按钮。

- [ ] **Step 7：实现内联 panel 和覆盖抽屉**

增加两个内联 panel helper：

```rust
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
```

增加覆盖抽屉 helper：

```rust
fn sidebar_drawer(state: &NormaAppState) -> AnyElement {
    div()
        .id("sidebar-drawer")
        .absolute()
        .top(theme::TOOLBAR_HEIGHT)
        .bottom_0()
        .left_0()
        .w(theme::SIDEBAR_WIDTH)
        .border_r_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .child(sidebar::render_sidebar(state))
        .into_any_element()
}

fn inspector_drawer(state: &NormaAppState) -> AnyElement {
    div()
        .id("inspector-drawer")
        .absolute()
        .top(theme::TOOLBAR_HEIGHT)
        .bottom_0()
        .right_0()
        .w(theme::INSPECTOR_WIDTH)
        .border_l_1()
        .border_color(theme::border())
        .bg(theme::surface())
        .child(inspector::render_inspector(state))
        .into_any_element()
}
```

- [ ] **Step 8：重写 AppShell::render 的布局组装**

在 `render` 开始处使用真实窗口 bounds：

```rust
let layout = WorkbenchLayout::for_width(window.bounds().size.width);
self.apply_size_class(layout.size_class);
```

把参数 `_window` 改为 `window`。

中心区 padding：

```rust
let center_padding = if layout.size_class == WindowSizeClass::Compact {
    px(16.)
} else {
    px(24.)
};
```

将根布局组装为相对定位容器：

```rust
let inline_sidebar = layout.sidebar_inline.then(|| sidebar_panel(&self.state));
let inline_inspector = layout
    .inspector_inline
    .then(|| inspector_panel(&self.state));
let overlay_sidebar = (!layout.sidebar_inline && self.sidebar_drawer_open)
    .then(|| sidebar_drawer(&self.state));
let overlay_inspector = (!layout.inspector_inline && self.inspector_drawer_open)
    .then(|| inspector_drawer(&self.state));

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
    .children(overlay_sidebar)
    .children(overlay_inspector)
```

- [ ] **Step 9：验证响应式逻辑**

Run:

```bash
cargo fmt --check
cargo test ui::shell::tests
cargo test ui::window::policy::tests
cargo check
```

Expected: all commands exit 0。

- [ ] **Step 10：提交**

```bash
git add src/ui/shell.rs
git commit -m "feat(ui): 添加工作台响应式布局"
```

Expected: commit succeeds。

## Task 4：建立三个主区域的独立滚动边界

**Files:**

- Modify: `src/ui/sidebar.rs:7-19`
- Modify: `src/ui/execution.rs:7-29`
- Modify: `src/ui/inspector.rs:7-45`
- Test: `src/ui/shell.rs` GPUI test module

- [ ] **Step 1：先写 GPUI resize 回归测试**

在 `src/ui/shell.rs` 测试模块增加 import：

```rust
use gpui::{AppContext, TestAppContext, size};
```

增加测试：

```rust
#[gpui::test]
fn resizing_the_window_recomputes_the_shell_size_class(
    cx: &mut TestAppContext,
) {
    let (_updates_tx, updates_rx) = mpsc::channel();
    let (shell, cx) = cx.add_window_view(|_, _| {
        AppShell::new(NormaAppState::no_project(), updates_rx)
    });

    cx.simulate_resize(size(px(1024.), px(700.)));
    cx.run_until_parked();
    assert_eq!(
        shell.read(cx).last_size_class,
        Some(WindowSizeClass::Compact)
    );

    cx.simulate_resize(size(px(1280.), px(800.)));
    cx.run_until_parked();
    assert_eq!(
        shell.read(cx).last_size_class,
        Some(WindowSizeClass::Wide)
    );
}
```

- [ ] **Step 2：运行 GPUI 测试**

Run:

```bash
cargo test ui::shell::tests::resizing_the_window_recomputes_the_shell_size_class
```

Expected: PASS，并证明真实 GPUI resize 会重新运行 AppShell 布局。不得把该测试替换成 `WorkbenchLayout::for_width` 的纯函数断言，因为 Task 1 已覆盖纯函数。

- [ ] **Step 3：让 Sidebar 内容滚动**

将 `render_sidebar` 根 div 增加：

```rust
.min_h(px(0.))
.overflow_y_scroll()
```

保留 `size_full`、padding 和现有内容顺序。

- [ ] **Step 4：固定 Composer 并只滚动执行事件**

将 `render_execution` 重组为：

```rust
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
```

- [ ] **Step 5：让 Inspector 主内容滚动**

在 `render_inspector` 的 tabs 后主内容 div 上增加：

```rust
.id("inspector-scroll")
.flex_1()
.min_h(px(0.))
.overflow_y_scroll()
```

根 div 保持 `size_full().flex().flex_col()`。

- [ ] **Step 6：运行相关测试和编译**

Run:

```bash
cargo fmt --check
cargo test ui::shell::tests
cargo check
```

Expected: all commands exit 0。

- [ ] **Step 7：提交**

```bash
git add src/ui/sidebar.rs src/ui/execution.rs src/ui/inspector.rs src/ui/shell.rs
git commit -m "fix(ui): 增加独立滚动区域"
```

Expected: commit succeeds。

## Task 5：让设置窗口在最小宽度下纵向布局

**Files:**

- Modify: `src/ui/settings.rs:1-7,206-299,359-452`
- Test: `src/ui/window/policy.rs`

- [ ] **Step 1：补充设置窗口 projection 测试**

在 `src/ui/window/policy.rs` 测试模块加入：

```rust
#[test]
fn settings_minimum_width_uses_stacked_provider_layout() {
    let options = WindowPolicy::settings_window_options();
    let minimum = options.window_min_size.unwrap();

    assert_eq!(minimum.width, px(840.));
    assert_eq!(
        WindowPolicy::settings_size_class(minimum.width),
        SettingsSizeClass::Stacked
    );
}
```

- [ ] **Step 2：运行测试**

Run:

```bash
cargo test ui::window::policy::tests
```

Expected: 9 tests PASS。

- [ ] **Step 3：从 SettingsWindow::render 派生布局**

在 `src/ui/settings.rs` import：

```rust
use crate::ui::window::{SettingsSizeClass, WindowPolicy};
```

把 `render` 的 `_window` 改为 `window`，并在读取 config 后加入：

```rust
let stacked = WindowPolicy::settings_size_class(window.bounds().size.width)
    == SettingsSizeClass::Stacked;
```

将右侧内容容器改为：

```rust
div()
    .flex_1()
    .min_w(px(0.))
    .min_h(px(0.))
    .overflow_y_scroll()
    .p_6()
    .child(settings_content(
        &self.state,
        &config,
        self.name_input.as_ref(),
        self.base_url_input.as_ref(),
        self.api_key_input.as_ref(),
        self.model_input.as_ref(),
        stacked,
    ))
```

- [ ] **Step 4：向设置内容传递 stacked projection**

在 `settings_content` 和 `ai_provider_pane` 的参数末尾增加：

```rust
stacked: bool,
```

并在调用链中完整传递该值。

在 `ai_provider_pane` 中先构建 provider body：

```rust
let provider_body = div().flex().gap_5();
let provider_body = if stacked {
    provider_body.flex_col()
} else {
    provider_body
};
```

同时把 `ai_provider_pane` 根 div 的 `.size_full()` 改为 `.w_full()`，使纵向堆叠内容可以超过视口高度并由外层设置内容容器滚动。

将原有第 423–435 行 body 替换为：

```rust
.child(
    provider_body
        .child(provider_list(state, config, stacked))
        .child(provider_editor(
            state,
            config,
            name_input,
            base_url_input,
            api_key_input,
            model_input,
        )),
)
```

- [ ] **Step 5：让 Provider 列表宽度响应布局**

将 `provider_list` 签名改为：

```rust
fn provider_list(
    state: &Arc<Mutex<SettingsWindowState>>,
    config: &AppConfig,
    stacked: bool,
) -> AnyElement
```

把函数开头改为：

```rust
let list = div()
    .rounded(px(10.))
    .border_1()
    .border_color(theme::border())
    .bg(theme::surface())
    .flex()
    .flex_col();
let list = if stacked {
    list.w_full()
} else {
    list.w(px(360.))
};
```

后续在 `list` 上继续调用 `.child(...)` 和 `.children(...)`，删除原有固定 `.w(px(360.))`。

- [ ] **Step 6：收紧设置导航宽度**

将 `settings_navigation` 的固定宽度从：

```rust
.w(px(240.))
```

改为：

```rust
.w(px(220.))
```

该调整为最小窗口给右侧内容额外保留 20px，不改变导航结构。

- [ ] **Step 7：验证设置布局**

Run:

```bash
cargo fmt --check
cargo test ui::window::policy::tests
cargo check
```

Expected: all commands exit 0。

- [ ] **Step 8：提交**

```bash
git add src/ui/settings.rs src/ui/window/policy.rs
git commit -m "feat(settings): 支持窄窗口布局"
```

Expected: commit succeeds。

## Task 6：更新视觉契约并执行完整验证

**Files:**

- Modify: `tests/visual_contract.md`
- Modify: `tests/settings_visual_contract.md`

- [ ] **Step 1：更新主窗口视觉契约**

在 `tests/visual_contract.md` 的 `Window Shell` 下加入：

```markdown
- [ ] Window cannot be resized below 1024x700
- [ ] At 1280px and wider, sidebar and inspector are both inline
- [ ] At 1120-1279px, sidebar is inline and inspector opens as an overlay drawer
- [ ] At 1024-1119px, sidebar and inspector both open as overlay drawers
- [ ] Compact toolbar keeps navigation, run, inspector, and settings actions reachable
- [ ] Resizing between layout classes closes stale overlay drawers
```

在 `Center Execution Stream` 下加入：

```markdown
- [ ] Execution events scroll independently while the composer remains visible
```

在 `Left Sidebar` 和 `Right Inspector` 对应段落加入：

```markdown
- [ ] Sidebar content scrolls without moving the workbench toolbar
- [ ] Inspector content scrolls without moving the inspector tabs
```

- [ ] **Step 2：更新设置窗口视觉契约**

在 `tests/settings_visual_contract.md` 的 `Settings Window` 下加入：

```markdown
- [ ] The settings window cannot be resized below 840x620
- [ ] At 920px and wider, provider list and editor remain side by side
- [ ] Below 920px, provider list stacks above the editor
- [ ] Settings content scrolls without moving the settings header
- [ ] At 840x620, every provider field and action remains reachable
```

- [ ] **Step 3：运行静态和单元验证**

Run:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Expected: all commands exit 0；不得通过降低 lint 或删除测试绕过失败。

- [ ] **Step 4：运行应用并完成主窗口视觉检查**

Run:

```bash
cargo run
```

依次检查：

- `1440×1024`：完整三栏和完整状态 pill。
- `1280×800`：仍为完整三栏。
- `1279×800`：右侧 Inspector 变为抽屉。
- `1120×760`：左侧内联、右侧抽屉。
- `1119×760`：左右均为抽屉。
- `1024×700`：无法继续缩小，Composer 和 toolbar 动作可见。

Expected: 无裁剪、无区域重叠、无不可达操作；执行流、Sidebar 和 Inspector 可独立滚动。

- [ ] **Step 5：完成设置窗口视觉检查**

从主窗口打开设置，依次检查：

- `960×720`：Provider 列表和编辑器横向排列。
- `919×700`：Provider 列表堆叠到编辑器上方。
- `840×620`：无法继续缩小，所有输入和动作可通过滚动访问。

Expected: 输入焦点、复制粘贴、API Key 显示/隐藏和保存门禁行为保持不变。

- [ ] **Step 6：检查 Git 范围**

Run:

```bash
git status --short
git diff --check
git diff --stat
```

Expected:

- 只包含本计划列出的源码和测试契约。
- `docs/superpowers/` 未被暂存。
- 没有 `target/`、`.superpowers/` 或 `~/.norma/` 产物。
- `git diff --check` exit 0。

- [ ] **Step 7：提交验证契约**

```bash
git add tests/visual_contract.md tests/settings_visual_contract.md
git commit -m "test(ui): 更新响应式视觉验证"
```

Expected: commit succeeds。

## 完成条件

完成阶段 1 前必须同时满足：

- `WindowPolicy` 是主窗口、设置窗口初始尺寸和最小尺寸的唯一来源。
- 主窗口使用 `NormaConfig.window`，且小于产品下限的尺寸会被钳制。
- 主窗口在 `1024×700` 可正常操作。
- 设置窗口在 `840×620` 可访问全部字段和动作。
- Workbench 三个宽度档位与设计规范一致。
- Composer 不随执行事件滚出视口。
- Sidebar、Execution、Inspector 和 Settings 内容分别拥有滚动边界。
- 所有新增纯逻辑和 GPUI resize 测试通过。
- `cargo fmt --check`、`cargo check`、`cargo test`、Clippy 全部通过。
- 人工视觉清单完成。
- 没有实现阶段 2 及后续范围。
