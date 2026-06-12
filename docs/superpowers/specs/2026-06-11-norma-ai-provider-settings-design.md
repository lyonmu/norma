# Norma AI Provider Settings Design

Date: 2026-06-11

## Summary

Norma needs a separate application settings window for configuring AI providers. The window opens from the top-right settings button in the main workbench, but it must not replace, clear, navigate away from, or otherwise interrupt the active model interaction session.

The selected direction is based on the first visual concept: a calm, compact settings surface that inherits Norma's Review-First Codex Workbench language while moving configuration into its own app-level window.

This design covers UI structure, provider configuration fields, save-gated provider test state, and integration boundaries. Provider connection testing is implemented by the Rig provider abstraction change.

## Product Goals

- Let users add and edit AI provider configurations.
- Support exactly two protocol formats at this stage: `OpenAI` and `Anthropic`.
- Keep the active task conversation available while settings are open.
- Provide a settings structure that can later expand to other app-level options.
- Prepare provider records for model-call integration through Rig plus a custom Provider abstraction layer.

## Non-Goals

- Do not call real models for chat/session generation from this settings page.
- Do not save provider configuration until the current provider candidate has passed a connection test.
- Do not add MCP, Skills, automation, tool-calling, pricing, quota, account, profile, or billing settings.
- Do not turn the right workbench inspector into the full settings surface.
- Do not interrupt or mutate the active session thread when opening, editing, saving, or closing settings.

## Entry Point And Window Behavior

The main workbench top toolbar keeps a compact settings icon in the top-right action group. Clicking it opens a separate GPUI settings window titled `设置`.

The settings window is application-level configuration, not a thread-level inspector mode:

- the current workbench window remains open
- the selected thread stays selected
- the center execution stream remains on the active conversation
- the composer contents are preserved
- opening settings does not append session events
- closing settings returns focus to the existing workbench state

The settings window may appear in front of the workbench, but the workbench should not be dimmed into unusability. Settings should feel parallel to the conversation, not modal to it.

## Visual Direction

Use the same product language as the Norma workbench:

- light native desktop surface
- white content panels on a very light gray background
- compact 13-15px typography
- thin hairline dividers between navigation, grouped rows, form sections, and footer actions
- 8-10px radii for grouped surfaces and controls
- restrained blue accent for selected navigation, focused rows, and primary actions
- green status for complete or safe configuration
- red only for invalid states
- minimal shadows
- no decorative illustration, gradient blobs, marketing hero, account UI, profile menu, or avatar

The window should feel dense but calm, closer to a native app preferences window than a dashboard.

## Layout

The settings window uses a compact two-column layout:

- left settings navigation, about 220-260px wide
- right content pane, flexible width with comfortable line lengths
- top title area with `设置`
- footer or bottom action area only when the current pane has editable state

The left navigation includes these sections:

- `通用`
- `AI 提供商` selected
- `运行环境`
- `安全`
- `Git`
- `外观`

Navigation rows are quiet list rows with a soft selected tint. They are not large cards. Future settings categories can be added here without changing the workbench layout.

## AI Provider Pane

The `AI 提供商` pane configures provider records for model calls. It may run a low-cost provider connection test, but must not imply that the active agent runtime is already using the provider for full chat/session generation.

The pane includes:

- title `AI 提供商`
- helper text `测试通过后才能保存配置。配置会写入本机 ~/.norma/config.toml。`
- icon-led `新增提供商` action
- grouped provider list with row separators
- selected provider detail editor
- provider test/save note `测试通过后才能保存配置。配置会写入本机 ~/.norma/config.toml。`

Provider list rows show:

- provider name, such as `OpenAI 默认` or `Claude 代理`
- protocol pill: `OpenAI` or `Anthropic`
- model name, such as `gpt-4o` or `claude-3-5-sonnet`
- status, such as `配置完整` or `待测试`

The selected provider row uses a soft blue tint. Status is textual and restrained; it should communicate candidate test/save readiness without looking like a broad account or billing health monitor.

## Provider Editor

The selected provider editor includes exactly these required configuration fields:

- `名称`
- `协议类型`
- `Base URL`
- `API Key`
- `模型`

`协议类型` is a segmented control with exactly two options:

- `OpenAI`
- `Anthropic`

`API Key` is masked by default and includes a reveal/hide icon button. Mock screenshots, docs, logs, sample data, and default fixtures must never show a real API key.

The action area includes:

- primary `保存配置`
- `测试连接`

`测试连接` runs a low-cost provider test through the provider abstraction. `保存配置` remains unavailable until the tested candidate fingerprint matches the current provider candidate.

## Data Model Boundaries

The exact Rust names can evolve, but the design should preserve these concepts:

- `SettingsSection`: general, AI providers, runtime, safety, Git, appearance.
- `ProviderProtocol`: OpenAI or Anthropic.
- `AiProviderConfig`: id, name, protocol, base URL, masked API key reference or secret-storage key, model, status.
- `ProviderConfigStatus`: complete, incomplete, invalid, or preview/unvalidated.

Configuration state belongs under the future `config` boundary, not `session`. Settings reads and writes must not mutate the active session thread or execution stream.

## Validation And Error Handling

Validation stays local to the settings window:

- missing provider fields show inline errors in the provider editor
- invalid protocol state is impossible through the segmented control
- API keys remain masked unless the user explicitly reveals them
- connection failures are shown as provider-test errors and must not expose raw API keys or credentials

Global modal errors should be rare. Provider editing errors belong inside the provider editor.

## Acceptance Criteria

- Clicking the top-right settings icon opens a separate `设置` window.
- The active workbench conversation remains visible and usable behind the settings window.
- Opening settings does not clear the composer, switch threads, append session events, or navigate the center stream.
- The settings window has left navigation with `通用`, `AI 提供商`, `运行环境`, `安全`, `Git`, and `外观`.
- `AI 提供商` is selected and shows a provider list plus selected provider editor.
- Provider records expose `名称`, `协议类型`, `Base URL`, `API Key`, and `模型`.
- Protocol selection is limited to `OpenAI` and `Anthropic`.
- API keys are masked by default.
- `测试连接` runs the configured provider test path and updates tested state on success.
- `保存配置` is blocked until the current provider candidate matches the successful test state.
- The visual style matches Norma's calm, compact, light desktop workbench language.

## Implementation Notes

Suggested UI boundaries:

- `SettingsWindow`
- `SettingsNavigation`
- `AiProviderSettingsPane`
- `ProviderList`
- `ProviderEditor`

Suggested tests:

- opening settings preserves the active session/thread state
- provider records validate required fields without network calls
- protocol selection is limited to `OpenAI` and `Anthropic`
- API keys render masked by default in provider rows, editor fields, screenshots, and sample data
