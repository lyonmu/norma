# Norma Text Input System Design

Date: 2026-06-16

## Summary

Norma needs a first-class text input system for the desktop workbench, not another narrow patch to the current settings-only `TextInput`. The first version will provide a reusable plain-text editing core, GPUI input components for common use cases, and a composer-specific input surface for agent prompts.

The accepted scope is:

- A global input system covering current and future text inputs.
- A pure text editing core with cursor, selection, clipboard-oriented commands, undo and redo, single-line and multi-line constraints, password masking, disabled and read-only states, and error display support.
- Agent composer behavior: `Enter` submits, `Shift+Enter` inserts a newline, empty content cannot submit, and sending locks editing without losing content.
- Cross-platform input abstractions, with implementation and verification prioritized for macOS desktop behavior in the first pass.

Command palette behavior, slash commands, `@` file references, history suggestions, and autocomplete popovers are out of scope for this version.

## Current Problems

The current `src/ui/text_input.rs` is a thin GPUI component. It stores a `String`, appends printable key characters, handles Backspace, clears on Delete, and blurs on Escape. It does not model cursor position, selection, paste, undo, redo, password masking, text overflow, multi-line editing, disabled states, or validation states.

The settings window wraps each `TextInput` in an additional form-row border, which blurs the boundary between input component styling and form layout styling. The current API provider settings are the only live text input use case, while the center composer is still a static placeholder.

## Architecture

Add `src/ui/input/` as the unified input domain. The module should separate editing behavior from GPUI rendering and from business state.

### Model Layer

The model layer is pure Rust and does not depend on GPUI. It owns text editing state and deterministic editing behavior.

Primary responsibilities:

- Store text content.
- Track caret and selection.
- Support single-line and multi-line modes.
- Insert text, replace selected text, delete backward, delete forward, and clear selection.
- Move the caret by character, word, line boundary, and document boundary.
- Select all and extend selection while moving.
- Maintain undo and redo history.
- Produce display text for secure inputs without exposing raw secrets to rendering helpers.
- Enforce maximum length and line constraints.

This layer should be tested heavily because it is the behavioral foundation for all input components.

### Controller Layer

The controller layer maps platform input events into model commands. It should hide platform-specific modifier details behind typed commands rather than spreading keystroke string checks through view code.

Primary commands include:

- Insert committed text.
- Delete backward and forward.
- Move caret left, right, up, down, to line start, to line end, to document start, and to document end.
- Extend selection while moving.
- Select all.
- Copy, cut, paste.
- Undo and redo.
- Submit.
- Insert newline.
- Blur.

The first implementation should prioritize macOS shortcuts:

- `Cmd+A`: select all.
- `Cmd+C`: copy selection.
- `Cmd+X`: cut selection.
- `Cmd+V`: paste text.
- `Cmd+Z`: undo.
- `Cmd+Shift+Z`: redo.
- Arrow keys: move caret.
- Shift plus movement: extend selection.
- Option and Command movement: word and boundary movement where GPUI event data supports it.
- `Enter`: submit for single-line fields and composer.
- `Shift+Enter`: newline in multi-line composer.
- `Escape`: blur or exit the active edit state.

The command model should leave room for Windows and Linux mappings, but cross-platform parity is not required before the macOS path is verified.

### View Layer

The GPUI view layer renders input state and forwards user events to the controller. It does not own business meaning.

Components:

- `TextField`: standard single-line input for names, URLs, model IDs, paths, search, and similar fields.
- `SecureTextField`: single-line secure input for API keys and other secrets. It masks by default and supports a visible show/hide affordance.
- `TextArea`: multi-line plain-text input with automatic height up to a configured maximum and internal scrolling after that point.
- `ComposerInput`: multi-line agent prompt input built on the text area behavior, with submit handling and send-state locking.
- `FormField`: optional form wrapper for label, required marker, help text, and error text. This wrapper owns form layout, while input components own input visuals.

The existing `TextInput` should not remain as a separate second system. The implementation can either replace settings call sites directly with the new components or keep a short-lived compatibility wrapper during migration, but the end state should have one input system.

## State And Data Flow

Input components own editing state: content, caret, selection, undo stack, focus state, visibility mode for secure text, and local disabled/read-only state.

Business state remains outside input components:

- Settings owns provider view-model fields and provider validation state.
- Provider tests and save gates remain in settings/config/provider code.
- Composer owns submission rules and turns submitted text into an agent request.
- Runtime execution owns running and completed session state.

Every content change emits an `InputChange` to the caller. The caller may update its view model, invalidate derived state, or block downstream actions. The input component must not write configuration files, call providers, start runtime execution, or mutate session state directly.

For settings:

- Provider name, Base URL, API key, and model fields should use the new input components.
- Editing any provider field should invalidate the previous tested candidate state.
- API keys should be masked by default.
- Save remains gated by a fresh successful provider test.

For composer:

- Empty or whitespace-only content cannot submit.
- `Enter` submits the current prompt.
- `Shift+Enter` inserts a newline.
- Sending locks editing or makes the input read-only.
- Failed submission should keep the user-entered content available.

## Visual Design

The input system should match Norma's current light desktop workbench style:

- Compact 13-15px text.
- 38px single-line input height by default.
- 8px input radius.
- White input surfaces on the existing light app background.
- Thin hairline borders.
- Blue border and subtle focus ring for focus.
- Red border and short red helper text for errors.
- Muted text and light tinted background for disabled or read-only state.
- Placeholder text uses muted color and never competes with content.
- Secure fields show masked text by default with a compact show/hide affordance.
- Composer uses the same visual language, with a taller multi-line body and a compact footer for actions and hints.

Inputs should not resize unpredictably when content, placeholder text, hover state, focus state, or error state changes. Long single-line values should scroll or clip within the input rather than expanding the layout.

## IME And Unicode

The first version must be friendly to Chinese text input. Composition text should not be split into incorrect intermediate characters. Only committed text should enter the buffer.

The model layer should operate on safe character boundaries rather than raw byte offsets. Tests should cover multi-byte Unicode text, including Chinese text and mixed ASCII/Chinese content, for insertion, deletion, selection, and cursor movement.

If a GPUI event limitation prevents complete IME behavior in the first implementation, the gap must be explicit in the implementation plan and verification notes. It should not be silently treated as complete.

## Error Handling

Editing errors are local to the input component. Examples include editing while disabled, inserting a newline into a single-line field, or exceeding a configured maximum length. These should be ignored or surfaced as lightweight local state. They should not create high-volume runtime logs.

Form errors belong to the caller. Examples include missing required values, invalid URLs, stale provider test state, and failed provider connection tests. The caller computes these states and passes error text into the input/form wrapper.

Composer submission errors belong to the composer layer. Empty submit and duplicate submit while sending should be blocked without clearing user text. Runtime or provider failures should be shown in the session stream, not as low-level input errors.

No input path may log secrets, raw API keys, model tokens, provider credentials, or private prompt content.

## Testing

Unit tests should cover the model layer:

- Insert text at caret.
- Replace selected text.
- Delete backward and forward.
- Select all.
- Move caret by character and boundary.
- Extend selection.
- Undo and redo.
- Single-line newline rejection.
- Multi-line newline insertion.
- Max length behavior.
- Secure display masking.
- Unicode and Chinese text editing.

Controller tests should cover command mapping:

- macOS select all, copy, cut, paste, undo, redo.
- Enter submit.
- Shift+Enter newline.
- Backspace and Delete.
- Escape blur behavior.
- Movement and selection extension where event data supports it.

Settings tests should cover integration:

- Editing provider fields updates the selected provider view model.
- Editing provider fields invalidates stale provider test state.
- API key fields are secure by default.
- Save remains blocked until the current candidate has a fresh successful test.

Composer tests should cover integration:

- Empty content cannot submit.
- Enter submits content.
- Shift+Enter inserts a newline.
- Sending state locks or makes input read-only.
- Failed submit does not clear input content.

Manual visual verification should update or add checklists for:

- Focus, hover, error, disabled, and read-only states.
- Secure input masking and show/hide behavior.
- Long URL and long model ID behavior.
- Chinese IME input.
- Copy, paste, undo, and redo.
- Composer submission, newline, and sending state.

## Migration Plan Boundary

The design phase does not implement code. The follow-up implementation plan should break the work into small steps:

1. Add the pure input model and tests.
2. Add command mapping and tests.
3. Add GPUI field components.
4. Migrate settings provider fields.
5. Add composer input state and render it in the execution pane.
6. Update manual visual checklists.
7. Run formatting, check, tests, clippy, and manual UI verification.

This migration should avoid unrelated UI redesign. It should only improve input behavior, visual states, and the surfaces directly affected by text entry.
