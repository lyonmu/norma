# Text Input System Fix Design

Date: 2026-06-16

## Summary

The text input system implementation compiles and passes the current tests, but review found behavior gaps against the approved input-system design. This fix keeps the original architecture and fills the missing behavior without expanding into a full desktop text editor.

The accepted repair scope is:

- Wire composer submission through `Enter` and the send button.
- Keep `Shift+Enter` as composer newline.
- Save edited provider settings from the current candidate values instead of writing stale persisted config.
- Add TextArea copy, cut, and paste behavior.
- Add a minimum visual caret and selection rendering path for single-line fields, secure fields, and text areas.

Mouse character hit-testing, drag selection, and caret-following scroll behavior are not in this repair scope.

## Composer Submission

`ComposerInput` must own a submit path instead of only storing `on_submit`.

Required behavior:

- `Enter` in composer context calls `ComposerState::submit()`.
- `Shift+Enter` inserts a newline.
- The send button calls the same submit path as `Enter`.
- Empty or whitespace-only content does not submit.
- Sending state blocks submit.
- Successful submit calls `on_submit(trimmed_content)`.
- Successful submit clears the input.
- Blocked or failed submit does not clear the input.

The component should expose a small command-handling method that can be unit tested without relying on GPUI click simulation. GPUI rendering can bind both key handling and the send button to that method.

## Settings Persistence

Provider editing currently updates `AppConfig`, while save writes `persisted_config` unchanged. The fix must make save write the current candidate provider data.

Required behavior:

- Editing provider name, Base URL, API key, or model still updates the selected provider view model.
- Editing still invalidates the previous tested fingerprint.
- Provider testing still runs against the current candidate values.
- Saving after a fresh successful test writes a `NormaConfig` that includes the current candidate provider values.
- Saving must not log or expose raw API keys.

The conversion from `AppConfig` provider view model to persisted `AiProviderConfig` should be explicit and tested. It should replace the matching provider in `persisted_config.ai.providers`; if the provider is new, it may be appended.

## TextArea Clipboard

`TextArea` must not route clipboard commands into no-op model branches.

Required behavior:

- Copy writes selected text to the clipboard.
- Cut writes selected text to the clipboard, removes the selection, and emits change.
- Paste inserts clipboard text at the current selection/caret and emits change.
- Multi-line pasted text remains multi-line in `TextArea`.
- Clipboard operations should share helper behavior with `TextField` where practical.

The text editing model should expose small pure helpers for cut and paste so behavior can be unit tested without GPUI clipboard access.

## Caret And Selection Visuals

The current model tracks selection but the UI renders only plain text. The fix must add a minimum visible text state.

Required behavior:

- When focused and no text is selected, render a visible 1px blue caret.
- When a range is selected, render selected text with a blue or light-blue background.
- Plain text before and after the selection remains visible.
- Placeholder text remains muted and does not show a caret unless the input is focused.
- Secure input keeps masking behavior while still showing caret/selection at masked-character positions.
- Text areas use the same segment concept, including multi-line content.

Implementation can use a simple segment helper such as `display_segments(display_mode)` on `TextBuffer`. It does not need mouse hit-testing, drag selection, or scroll-following behavior in this repair.

## Testing

Add tests that fail against the reviewed implementation and pass after the repair.

Composer tests:

- `Enter` submit triggers the stored callback with trimmed content.
- Successful submit clears content.
- `Shift+Enter` inserts newline and does not submit.
- Empty content and sending state do not submit or clear.

Settings tests:

- Edit a provider field, mark/test the candidate, save, and assert the captured `NormaConfig` contains the edited field.
- Repeat for API key or Base URL so stale persisted config cannot pass.

Text model and clipboard tests:

- Cut selected text returns the selected text and removes it.
- Paste inserts single-line and multi-line text at the caret.
- Clipboard helpers update undo/redo state consistently.

Visual segment tests:

- Caret segment at start, middle, and end.
- Forward and reversed selections produce the same selected segment.
- Chinese text selection uses valid character boundaries.
- Secure display segments mask text without exposing raw values.

Manual verification:

- Settings input editing, testing, and saving writes the edited values.
- API key remains masked by default.
- Composer `Enter` submits and `Shift+Enter` creates a newline.
- Composer send button submits through the same path.
- TextArea copy, cut, and paste work for multi-line text.
- Focused inputs show visible caret or selected text.

## Verification

Before handoff, run:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`
- `cargo run` for manual settings and composer verification

If manual UI verification cannot be completed, report the exact reason and the automated verification that did run.
