# Settings Visual Contract

Use this checklist when verifying `docs/superpowers/specs/2026-06-11-norma-ai-provider-settings-design.md`.

## Workbench Entry

- [ ] The main workbench still opens at about 1440x1024.
- [ ] The top-right settings icon is compact and visually aligned with the notification/run controls.
- [ ] Clicking the settings icon opens a separate `设置` window.
- [ ] Opening settings does not replace the workbench content.
- [ ] Opening settings does not clear the center composer.
- [ ] Opening settings does not switch the selected thread.
- [ ] Opening settings does not add a session event to the execution stream.

## Settings Window

- [ ] The settings window is light, compact, and consistent with the workbench visual language.
- [ ] The left navigation includes `通用`, `AI 提供商`, `运行环境`, `安全`, `Git`, and `外观`.
- [ ] `AI 提供商` is selected with a soft blue tint.
- [ ] The right pane title is `AI 提供商`.
- [ ] The helper text says `为后续模型调用配置不同协议格式的提供商。`.
- [ ] Provider rows are grouped with separators rather than separate cards.
- [ ] The selected provider row is lightly tinted.

## Provider Editor

- [ ] The editor shows `名称`, `协议类型`, `Base URL`, `API Key`, and `模型`.
- [ ] `协议类型` has exactly `OpenAI` and `Anthropic`.
- [ ] `API Key` is masked by default.
- [ ] `保存配置` is the primary action.
- [ ] `测试连接（预览）` is disabled or clearly preview-only.
- [ ] The pane includes `模型调用将在后续通过 Rig + 自研 Provider 抽象层接入。当前仅保存配置预览。`.
- [ ] No real API key, account UI, avatar, billing, MCP, Skills, automation, or tool-calling controls are visible.
