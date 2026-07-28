# Norma 整体架构与响应式 UI 设计

日期：2026-07-28  
状态：已确认

## 1. 摘要

Norma 当前已经具备较清晰的领域目录，但窗口策略、GPUI 状态所有权、后台更新链路和设置窗口职责仍存在明显交叉。

本设计采用渐进式领域重整，而不是一次性重写：

1. 先修正 GPUI 窗口策略和 Workbench 响应式布局。
2. 再建立共享的应用模型和明确的后台事件桥接。
3. 拆分设置窗口的视图、草稿状态和业务编排。
4. 分区当前过宽的应用聚合状态。
5. 最后收紧 Agent、Provider 与异步执行边界。

每个阶段必须保持可编译、可测试、可运行。现有边界合理的模块不为目录整齐而重排。

## 2. 当前项目判断

### 2.1 已经合理的部分

以下结构已经按领域组织，职责总体清晰：

- `runtime/`：启动编排、更新消息和 watcher。
- `logging/`：结构化日志、轮转、压缩和保留。
- `config/`：持久化配置、配置文件和 reload。
- `paths/`：本地 `~/.norma` 路径。
- `workspace/`：项目和文件树。
- `git/`：只读 Git 状态。
- `session/`：线程、事件和检查器状态。
- `skills/`：Skills 目录索引。
- `agent/`：Agent runtime、Provider、输入和工具边界。

这些模块不需要被统一塞入 `domain/` 或 `infrastructure/` 大目录。继续保留领域优先结构更符合当前项目规模，也能避免同一个功能被按技术层拆散。

### 2.2 当前主要问题

#### 窗口配置与真实窗口脱节

`NormaConfig.window` 已包含 `width` 和 `height`，默认值为 `1440×1024`，但 `ui/shell.rs` 创建窗口时再次硬编码相同尺寸。

当前主窗口和设置窗口均未设置 `WindowOptions.window_min_size`。GPUI `0.2.2` 默认允许用户缩放窗口，因此窗口能够缩小到当前三栏布局无法正常工作的尺寸。

#### 固定双侧栏挤压中心区域

Workbench 当前固定使用：

- 左侧栏 `320px`
- 右侧检查器 `410px`
- 中心区域 `flex_1`

当窗口缩小时，左右栏继续占据固定宽度，中心执行流承担全部压缩。项目没有宽度档位、侧栏折叠或工具栏降级规则。

#### UI 同时承担渲染与业务编排

`ui/settings.rs` 接近 900 行，并同时承担：

- GPUI 布局与控件渲染
- Provider 编辑草稿
- Provider 候选转换
- 连接测试调用
- 保存条件判断
- 配置文件写入
- 错误文本生成

这使设置业务难以独立测试，也使未来异步 Provider 调用容易阻塞 UI。

#### 聚合状态过宽

`NormaAppState` 同时持有：

- Project selection
- File tree
- Git summary
- Session state
- Settings view config
- Persisted runtime config
- Runtime paths
- Skills index
- Runtime error

这些状态的生命周期和消费者不同。把它们放在一个结构中会扩大变更影响，也使多窗口共享状态变得模糊。

#### 后台更新依赖 Render 轮询

`AppShell::render` 在每次渲染时调用 `try_recv` 消费 `RuntimeUpdate`。

该方式存在两个问题：

- 后台事件本身没有明确触发 GPUI Entity 更新和重绘。
- Render 同时承担事件泵职责，状态更新时机依赖其他 UI 刷新。

#### 多窗口状态通过快照和锁共享

设置窗口创建时克隆 `AppConfig` 和 `NormaConfig`，再放入 `Arc<Mutex<SettingsWindowState>>`。

这会产生以下风险：

- 设置窗口可能继续编辑过期的配置快照。
- 主窗口和设置窗口缺少一个共同的 GPUI 状态所有者。
- UI 状态需要手工加锁，而不是使用 GPUI Entity 的应用线程模型。

#### Provider 执行所有权不清晰

OpenAI 和 Anthropic Provider client 当前各自创建 Tokio current-thread runtime，并通过 `block_on` 执行请求。

如果该同步接口直接从 GPUI 点击处理器调用，会阻塞 UI 线程。Provider client 不应独立拥有执行 runtime；异步任务所有权应由应用或 Agent 执行层统一管理。

## 3. 设计目标

- 主窗口保持可缩放，并设置 `1024×700` 的可用下限。
- 初始窗口尺寸读取 `NormaConfig.window`。
- 建立明确的宽度档位和布局降级规则。
- 使用 GPUI Entity 共享跨窗口应用状态。
- 让后台事件主动进入 GPUI 应用线程并触发 `notify`。
- 将业务动作和外部 IO 移出 Render。
- 分离跨窗口共享状态与窗口本地状态。
- 拆分过大的 UI 文件，但保留现有领域优先结构。
- 让每个迁移阶段都能独立验证。
- 为 MCP、工具执行和子 Agent 保留稳定扩展边界，但不创建空实现。

## 4. 非目标

- 不禁止用户缩放窗口。
- 不一次性移动或重命名全部模块。
- 不引入万能 Store 或万能 Event Bus。
- 不把所有领域模块包装进通用 `domain/`、`application/`、`infrastructure/` 目录。
- 不实现 MCP、ACP、工具执行、子 Agent 或多 Agent 协作。
- 不改变 Git V1 只读边界。
- 不重写现有输入系统。
- 不让源码、测试或构建依赖 `.superpowers/` 或 `docs/superpowers/`。

## 5. GPUI 窗口策略

### 5.1 框架能力

GPUI `0.2.2` 的 `WindowOptions` 已提供：

- `is_resizable`
- `window_min_size`
- `window_bounds`

窗口尺寸变化会触发 GPUI 的 bounds 更新和窗口刷新。响应式布局可以从当前 `Window::bounds().size` 派生，不需要维护第二份真实窗口尺寸。

### 5.2 主窗口规则

主窗口策略：

- 默认初始尺寸：`1440×1024`
- 实际初始尺寸：读取 `NormaConfig.window`
- 最小尺寸：`1024×700`
- `is_resizable`：保持 `true`

当配置中的宽高小于最小尺寸时，创建窗口前将其钳制到最小尺寸。

最小尺寸属于产品可用性约束，不开放为普通用户配置。否则用户可以把最小值改到布局无法工作的范围。

`config.window` 只决定下一次创建主窗口时的初始尺寸。运行期间热加载该配置不会强制调整用户当前正在使用的窗口，避免窗口突然跳动。后续如需持久化用户最后一次窗口尺寸，应作为独立功能设计。

### 5.3 设置窗口规则

设置窗口策略：

- 初始尺寸：`960×720`
- 最小尺寸：`840×620`
- `is_resizable`：保持 `true`

设置窗口左侧导航可保持固定宽度，但右侧内容必须独立滚动。Provider 列表和编辑器在较窄状态下应允许纵向排列，不允许表单被硬裁剪。

### 5.4 WindowPolicy

新增集中式 `WindowPolicy`，负责：

- 主窗口初始尺寸
- 主窗口最小尺寸
- 设置窗口初始尺寸
- 设置窗口最小尺寸
- 宽度档位判定
- 配置尺寸钳制

窗口尺寸常量不再散落在 `shell.rs`、`settings.rs` 和 `theme.rs`。

`theme.rs` 继续保存视觉 token，例如颜色、圆角和标准栏宽；窗口可用性约束属于 `WindowPolicy`。

## 6. Workbench 响应式布局

### 6.1 宽度档位

定义三个 `WindowSizeClass`：

#### Wide：宽度大于等于 1280

- 显示完整三栏。
- 左侧栏保持约 `300–320px`。
- 右侧检查器保持约 `380–410px`。
- 中心执行流占据剩余空间。
- 工具栏显示完整状态 pill。

#### Medium：宽度为 1120–1279

- 保留左侧栏。
- 右侧检查器改为按需覆盖抽屉。
- 工具栏保留检查器入口。
- 隐藏低优先级状态文字，保留关键动作。

#### Compact：宽度为 1024–1119

- 中心执行流独占常规内容区域。
- 左侧栏改为覆盖抽屉。
- 右侧检查器改为覆盖抽屉。
- 工具栏使用紧凑按钮打开两个抽屉。
- Composer 必须始终可见并保持可输入宽度。

### 6.2 高度与滚动

最小高度为 `700px`，但内容仍不得依赖固定内容高度。

以下区域各自拥有滚动边界：

- 左侧项目、线程和文件树
- 中心执行事件流
- 右侧检查器内容
- 设置窗口主内容

顶栏和 Composer 不与执行流一起滚出视口。根布局继续使用 `overflow_hidden` 限定区域边界，实际内容容器使用对应的纵向滚动能力。

### 6.3 抽屉状态

`WorkbenchUiState` 保存：

- 左侧抽屉是否打开
- 右侧抽屉是否打开
- 临时 UI 选择状态

`WindowSizeClass` 从当前窗口 bounds 派生，不写入持久化配置。

窗口从窄档位切回宽档位时：

- 内联侧栏重新显示。
- 覆盖抽屉自动关闭。
- 不改变 Session、文件选择或检查器 tab 等业务状态。

## 7. 目标架构

### 7.1 依赖方向

目标逻辑依赖如下：

```text
main
  ↓
runtime/bootstrap
  ↓
app coordination
  ↓
workspace / session / agent / skills domain state

ui/GPUI ──────→ app commands and derived state
external IO ──→ runtime events and domain ports
```

核心约束：

- `main.rs` 只启动 runtime 和 UI。
- `runtime/bootstrap` 是唯一 composition root。
- `app/` 协调跨领域用例和跨窗口状态。
- UI 读取派生状态并发出命令。
- 外部 IO 通过领域 API 或服务端口进入。
- 业务规则不写在 GPUI Render 函数中。

### 7.2 目标文件分布

关键变化后的目录：

```text
src/
  main.rs
  runtime/
    mod.rs
    bootstrap.rs
    events.rs
    watchers.rs
  app/
    mod.rs
    model.rs
    services.rs
    workbench.rs
    runtime_state.rs
    command.rs
    settings/
      mod.rs
      draft.rs
      controller.rs
  ui/
    mod.rs
    window/
      mod.rs
      policy.rs
      host.rs
      runtime_bridge.rs
    workbench/
      mod.rs
      shell.rs
      toolbar.rs
      sidebar.rs
      execution.rs
      inspector.rs
    settings/
      mod.rs
      window.rs
      navigation.rs
      provider_editor.rs
    input/
    components.rs
    theme.rs
  agent/
  config/
  git/
  logging/
  paths/
  session/
  skills/
  workspace/
```

该目录是迁移终态，不要求第一阶段一次创建全部文件。

### 7.3 保持原位的模块

以下模块没有直接证据表明需要结构性重排：

- `logging/`
- `paths/`
- `git/`
- `workspace/`
- `skills/`
- `session/`

后续只在调用边界需要时调整窄 API，不进行无关搬迁。

## 8. 状态模型

### 8.1 AppModel

跨窗口共享一个 GPUI `Entity<AppModel>`。

`AppModel` 包含：

- `WorkbenchState`
- `RuntimeState`

主窗口和设置窗口都观察同一个 Entity。应用状态只在 GPUI 应用线程中更新，不再使用 `Arc<Mutex<UI State>>`。

外部能力由独立的 `AppServices` 持有，例如 Config repository、Provider test service 和 Agent runtime。`AppServices` 由 bootstrap 组装并交给 command handler 或 controller，不放入可变 UI 状态，也不由 Render 直接调用。

### 8.2 WorkbenchState

`WorkbenchState` 包含：

- `ProjectSelectionState`
- 文件树
- `GitStatusSummary`
- `SessionState`

它代表工作台业务状态，不包含窗口尺寸、抽屉或输入焦点。

### 8.3 RuntimeState

`RuntimeState` 包含：

- 当前已生效的 `NormaConfig`
- `SkillIndex`
- Provider runtime 可用状态
- 最近的 runtime reload 错误

持久化配置是运行时事实来源。设置窗口保存成功后，最终仍以 watcher 重新加载并发出的 `ConfigApplied` 为准。

### 8.4 SettingsDraft

每个设置窗口持有自己的 `SettingsDraft` Entity，包含：

- 当前编辑的 Provider 候选
- 字段校验错误
- 最近测试的候选 fingerprint
- 测试进度和错误
- 保存进度和错误

SettingsDraft 从 `RuntimeState` 初始化。收到外部配置更新时：

- 未修改的草稿可以自动刷新。
- 已修改的草稿不被静默覆盖。
- 界面显示“配置已在外部更新”，由用户选择重新加载。

### 8.5 WorkbenchUiState

Workbench 窗口本地保存：

- 抽屉开关
- 临时选择状态
- 与布局相关的局部交互状态

输入控件继续使用独立 GPUI Entity 保存焦点、光标、选择和撤销历史。

## 9. 命令与事件

### 9.1 使用原则

不建立覆盖所有 UI 行为的万能事件总线。

以下简单行为直接修改窗口本地 Entity：

- 输入内容变化
- 抽屉开关
- 本地 tab 或临时选择

以下行为使用显式 Command：

- 打开或切换项目
- 测试 Provider
- 保存配置
- 启动 Agent 任务
- 取消 Agent 任务
- 任何异步、持久化或跨窗口动作

### 9.2 RuntimeEvent

后台边界发送结构化 `RuntimeEvent`：

```text
ConfigApplied
ConfigRejected
SkillsApplied
SkillsRejected
ProviderTestCompleted
ProviderTestFailed
AgentEvent
```

Provider 和 Agent 事件可在对应阶段加入；第一阶段不提前实现没有消费者的变体。

### 9.3 GPUI Runtime Bridge

`ui/window/runtime_bridge.rs` 负责：

1. 在 GPUI foreground task 中等待 runtime event。
2. 在应用线程更新 `Entity<AppModel>`。
3. 归约事件并调用 `cx.notify()`。
4. channel 关闭或应用退出时正常结束。

Render 不再调用 `try_recv`。

为了让 watcher 线程和 GPUI foreground task 共享可唤醒的异步 channel，优先复用现有 Tokio 依赖并启用 `sync` feature，使用有界 `tokio::sync::mpsc`。这比新增另一个 channel crate 更小，也避免阻塞 GPUI 应用线程。

专用 watcher 线程使用 `blocking_send`，GPUI foreground task 使用异步 `recv`。任何发送失败都表示应用接收端已关闭，应记录低噪声诊断并结束对应生产任务。

channel 必须有界。低频 config/skills 更新使用较小容量即可；未来高频 Agent token 流需要单独定义背压或合并策略，不能直接复用低频 runtime channel 的容量假设。

## 10. 设置窗口用例

### 10.1 编辑

字段输入只更新 `SettingsDraft`，并使之前的测试 fingerprint 失效。

UI 不直接修改 `NormaConfig`。

### 10.2 测试连接

流程：

1. SettingsDraft 进行本地必填和候选一致性校验。
2. 发出 `TestProvider` command。
3. Controller 构建领域 Provider candidate。
4. Provider service 在后台执行测试。
5. 结果返回应用线程并更新 SettingsDraft。
6. 只有 fingerprint 仍与当前候选一致时，才标记测试通过。

Provider 网络请求不能在 GPUI 点击处理器中同步 `block_on`。

### 10.3 保存

流程：

1. 检查当前候选已通过测试且 fingerprint 未变化。
2. Controller 将 SettingsDraft 转换为持久化 Provider config。
3. Config repository 写入配置文件。
4. watcher reload 并验证配置。
5. `ConfigApplied` 更新 RuntimeState。
6. 主窗口和设置窗口收到 Entity 通知。

如果文件写入成功但 reload 验证失败，保留最后有效运行时配置，并在设置窗口显示明确错误。

## 11. 错误处理

错误按作用域归属：

- 字段校验：SettingsDraft，对应字段附近显示。
- Provider 测试：设置窗口状态，不写入 Session 事件。
- Config/Skills reload：RuntimeState，保留最后有效值。
- Agent 执行：SessionState 和 execution stream。
- 项目打开或 Git 读取：WorkbenchState 对应区域。
- 启动失败：bootstrap 返回带上下文的 `anyhow::Error`，不进入半初始化 UI。

外部调用和错误使用结构化 `tracing`，不得记录 API key 或原始敏感配置。

## 12. Agent 与 Provider 边界

本轮架构蓝图保留现有 `agent/provider` 领域，但后续需要解决执行所有权：

- Provider client 不再各自创建 Tokio runtime。
- Provider trait 逐步改为异步执行边界，或由统一执行服务包装同步适配器。
- GPUI 只发出 command，不直接调用阻塞 Provider 方法。
- Agent runtime 负责把 Provider 结果转换为 `SessionEvent`。
- Provider connection test 与 Agent completion 共享 client 创建规则，但保持不同用例接口。

该调整位于第五阶段，不能与第一阶段窗口改造混成一个大提交。

## 13. 分阶段迁移

### 阶段 1：WindowPolicy 与响应式布局

范围：

- 接入 `config.window` 初始尺寸。
- 设置主窗口和设置窗口最小尺寸。
- 新增 `WindowPolicy` 与 `WindowSizeClass`。
- 实现 Wide、Medium、Compact 布局。
- 加入抽屉和独立滚动区域。
- 更新人工视觉验证清单。

本阶段不拆 AppModel，不改 Provider。

### 阶段 2：AppModel 与 Runtime Bridge

范围：

- 创建共享 `Entity<AppModel>`。
- RuntimeUpdate 重命名或演进为 RuntimeEvent。
- 使用异步有界 channel。
- 在 GPUI task 中消费事件并更新 Entity。
- 删除 `AppShell::render` 中的 channel 轮询。
- 主窗口和设置窗口观察共享状态。

### 阶段 3：Settings 垂直切片

范围：

- 拆分 `ui/settings.rs`。
- 创建 SettingsDraft。
- 创建 Settings controller。
- 将 Provider 测试和配置写入移出 UI。
- 处理外部配置更新与脏草稿冲突。

### 阶段 4：Workbench 状态分区

范围：

- 创建 WorkbenchState 和 RuntimeState。
- 将 project、files、git、session 移入 WorkbenchState。
- 收紧 UI 读取接口，减少对整个 AppModel 的依赖。
- 保持 workspace、git、session 领域模型原位。

### 阶段 5：Agent 与 Provider 执行边界

范围：

- 移除 Provider client 自有 Tokio runtime。
- 建立统一异步执行所有权。
- 确保 Provider test 和 completion 不阻塞 UI。
- 明确 AgentEvent 到 SessionEvent 的转换位置。

### 阶段 6：收口

范围：

- 删除过渡 API。
- 收紧 `pub` 导出。
- 删除重复窗口常量和旧状态结构。
- 更新 README、架构文档和验证清单。
- 核对文档声明与实际能力一致。

## 14. 测试策略

### 14.1 WindowPolicy 单元测试

覆盖：

- 配置尺寸大于最小值时保持不变。
- 宽度或高度低于最小值时分别钳制。
- `1024`、`1119`、`1120`、`1279`、`1280` 边界映射正确。
- 设置窗口尺寸使用独立策略。

### 14.2 布局状态测试

覆盖：

- Wide 显示两个内联侧栏。
- Medium 只内联显示左侧栏。
- Compact 不内联显示左右侧栏。
- 从 Compact 切换到 Wide 后覆盖抽屉关闭。
- SizeClass 切换不改变 Session 或检查器业务状态。

优先测试纯布局 projection，避免用脆弱的像素截图代替行为断言。

### 14.3 GPUI 集成测试

使用 GPUI 测试上下文模拟窗口 resize，验证：

- 窗口 bounds 变化触发布局状态刷新。
- AppModel Entity 更新会通知窗口。
- RuntimeEvent 不依赖其他 UI 交互即可刷新。
- 窗口关闭后 bridge task 正常停止。

### 14.4 Settings 测试

覆盖：

- 编辑字段使测试 fingerprint 失效。
- 旧测试结果不能覆盖更新后的候选。
- 未测试或已变化候选不能保存。
- 保存成功后以 reload 结果更新 RuntimeState。
- 外部 reload 不静默覆盖脏草稿。
- Provider 错误不泄漏 API key。

### 14.5 人工视觉验证

主窗口至少验证：

- `1440×1024`
- `1280×800`
- `1279×800`
- `1120×760`
- `1119×760`
- `1024×700`

设置窗口至少验证：

- `960×720`
- `840×620`

检查滚动、抽屉、工具栏降级、Composer 可见性、输入焦点和 Inspector 内容。

### 14.6 仓库级门禁

每个阶段运行：

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

UI 阶段额外运行：

```bash
cargo run
```

并执行对应人工视觉清单。

## 15. 风险与缓解

### 过度抽象

风险：为未来能力创建大量空 trait、空事件和空模块。

缓解：只在当前迁移阶段创建有调用方和测试的接口。未来能力只通过稳定边界说明扩展位置。

### 响应式布局复杂度

风险：每个组件自行判断窗口宽度，产生分散条件。

缓解：统一使用 `WindowPolicy` 和 `WindowSizeClass`，组件只消费布局 projection。

### 多窗口状态冲突

风险：设置草稿与外部 reload 相互覆盖。

缓解：共享 RuntimeState，窗口保留独立 SettingsDraft，并显式处理 dirty 状态。

### 异步任务生命周期

风险：窗口关闭后任务继续更新已释放 Entity。

缓解：保存必要的 Task/Subscription 生命周期句柄；Entity 更新失败时正常终止；测试窗口关闭路径。

### Channel 背压

风险：未来高频 Agent 流堵塞低频配置更新。

缓解：本轮 runtime channel 只承载低频事件；Agent 流在第五阶段设计独立容量和合并策略。

### 大规模迁移

风险：同时拆目录、改状态和改异步导致问题难以定位。

缓解：严格按六个阶段执行，每阶段先补测试，再进行最小实现，完成全部门禁后进入下一阶段。

## 16. 验收标准

- 主窗口初始尺寸来自 `NormaConfig.window`。
- 主窗口最小尺寸为 `1024×700`。
- 设置窗口最小尺寸为 `840×620`。
- 三个宽度档位行为与本设计一致。
- 在最小尺寸下 Composer、主操作和内容滚动仍可用。
- WindowPolicy 是窗口尺寸与断点的唯一来源。
- Render 不再消费 runtime channel 或执行外部 IO。
- 主窗口和设置窗口共享一个 AppModel Entity。
- SettingsDraft 与持久化 RuntimeState 明确分离。
- Provider 测试和 Agent 请求不阻塞 GPUI 应用线程。
- `ui/settings.rs` 和 `ui/shell.rs` 的混合职责按目标模块拆分。
- 稳定领域模块不发生无关重排。
- 每个迁移阶段通过格式化、检查、测试和 Clippy。
- UI 阶段完成指定尺寸的人工视觉验证。

## 17. 参考

- GPUI `WindowOptions`：<https://docs.rs/gpui/0.2.2/gpui/struct.WindowOptions.html>
- 当前人工验证契约：`tests/visual_contract.md`
- 当前设置窗口验证契约：`tests/settings_visual_contract.md`
