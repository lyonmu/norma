# Repository Guidelines

## Project Summary

Norma is a Rust 2024 desktop agent/workbench built with Cargo and GPUI. The current code implements a GPUI workbench shell, local project/file context, read-only Git status summaries, mock agent session events, app settings UI with save-gated provider testing, local `~/.norma` paths, TOML configuration, structured tracing logs, config/skills watchers, and a non-executing Skills directory index.

The codebase has been reorganized into domain directories for runtime, logging, config, app state, workspace, Git, session, skills, and agent abstractions. The `agent/` domain includes structured input, provider, and tool-use model boundaries. Provider settings can run a low-cost real provider connection test through the provider abstraction; general runtime model calls, tools, MCP, ACP, executable Skills, sub-agents, and multi-agent workflows remain roadmap unless corresponding runtime implementation and tests exist.

README files and package metadata describe the broader product direction. Treat multi-model support, MCP, ACP, executable Skills, sub-agents, and multi-agent collaboration as roadmap unless the corresponding runtime implementation and tests exist.

## Tech Stack & Core Dependencies

- Rust 2024, Cargo, rustfmt, Clippy.
- GPUI `0.2.2` for native desktop UI.
- Runtime/config: `anyhow`, `serde`, `toml`, `config`, `notify`, `thiserror`.
- Logging: `tracing`, `tracing-subscriber`, `tracing-appender`, `flate2`.
- Project context: `ignore` for file tree traversal, read-only `git status` via `std::process::Command`.
- Tests: Rust unit tests plus `tempfile`; manual visual checklists in `tests/*.md`.

Do not add `gpui_platform`; this crate depends on `gpui`.

## Current Directory Structure

- `src/main.rs`: thin bootstrap only.
- `src/lib.rs`: module exports only.
- `src/runtime/`: startup orchestration, runtime update messages, and config/skills watchers.
- `src/logging/`: tracing initialization, JSON log writing, rotation, compression, retention, and logging errors.
- `src/config/`: app settings models, persisted runtime config, TOML read/write, validation, environment overrides, and reload state.
- `src/paths/`: local `~/.norma` path contract and directory creation.
- `src/skills/`: scans and reloads `~/.norma/skills` without executing skills.
- `src/workspace/`: project opening and file-tree loading.
- `src/git/`: read-only Git status command and parser.
- `src/session/`: session events, thread metadata, inspector state, and session state transitions.
- `src/agent/`: agent runtime trait, deterministic mock runtime, agent-domain events, structured input models, provider abstractions, and tool-use models.
- `src/app/`: aggregate `NormaAppState` and runtime update application.
- `src/ui/`: GPUI views and helpers, including workbench shell, sidebar, execution stream, inspector, settings window, components, and theme.
- `docs/superpowers/specs/`: product and architecture specs, including the module/logging restructure design.
- `docs/superpowers/plans/`: implementation plans. Treat plans as execution guidance, not proof that a capability exists unless the code is present.
- `tests/*.md`: manual visual/runtime verification checklists.
- `asset/`: static assets; current icon is `asset/img/icon.png`.
- `target/`: generated build output; do not edit or commit.

## Commands

- `cargo run`: run the desktop app.
- `cargo check`: type-check all reachable Rust code.
- `cargo test`: run unit tests.
- `cargo fmt --check`: verify Rust formatting.
- `cargo fmt`: apply Rust formatting.
- `cargo clippy --all-targets -- -D warnings`: run lint checks.
- `cargo build --release`: build the release binary referenced by package metadata.
- `cargo metadata --no-deps --format-version 1`: inspect crate/package metadata.

There are no Make, npm, pnpm, or shell task scripts in the repository.

## Structured Logging

Use `tracing` macros for runtime diagnostics: `trace!`, `debug!`, `info!`, `warn!`, and `error!`. Log application startup, config loading/reload, watcher events, key state transitions, external calls, and errors with context.

Do not use `println!`, `eprintln!`, `dbg!`, manual `stdout`/`stderr` writes, or ad hoc `std::fmt` output for logs or debugging. The only exception is a future CLI command whose purpose is to print a final user-facing result.

Prefer structured fields:

```rust
tracing::info!(component = "config", path = %config_path.display(), "config loaded");
tracing::warn!(error = %err, "failed to load config");
```

Never log secrets, API keys, model tokens, MCP credentials, raw provider credentials, or private config values. Avoid high-volume `info!` logs in GPUI render paths; use `debug!` or `trace!` sparingly.

## Modular Organization

Keep `main.rs` and `lib.rs` free of business logic. `main.rs` should bootstrap runtime and UI; `lib.rs` should export modules.

Organize independent capabilities as module directories. New work should extend the existing domain tree rather than adding broad top-level files. Use clear boundaries: config loading in `src/config/`, tracing setup in `src/logging/`, UI in `src/ui/`, aggregate state in `src/app/`, agent runtime and future provider/tool/input abstractions in `src/agent/`, and future LLM, MCP, ACP, Skills, and Subagent features in their own focused modules.

## Code Style

Use rustfmt defaults. Name functions and modules in `snake_case`, types and traits in `PascalCase`, and constants in `SCREAMING_SNAKE_CASE`. Keep public APIs narrow and document non-obvious behavior with concise doc comments. Use typed errors with context; preserve the existing pattern of `thiserror` for domain errors and `anyhow` at orchestration boundaries.

V1 Git behavior is read-only. Do not add reset, checkout, discard, patch application, or destructive Git operations unless a newer spec explicitly changes the boundary and tests cover it.

## Files And Directories Requiring Care

- `Cargo.toml` and `Cargo.lock`: keep dependency/package changes intentional and committed together.
- `Cargo.toml` package metadata: bundle/deb settings affect release packaging.
- `docs/superpowers/`: specs and plans are product contracts; update them only when intentionally changing scope.
- `asset/`: avoid replacing branding assets incidentally.
- `src/logging/`: maintain non-blocking structured JSON logging semantics.
- `src/config/`: never store real secrets in sample data, tests, screenshots, or defaults.
- `src/agent/provider/`, `src/agent/tools/`, and `src/agent/input/`: these are abstraction boundaries; do not imply real execution until runtime code and tests exist.
- `~/.norma/`: generated local runtime state outside the repo; do not copy it into version control.
- `target/`: generated output; never edit or commit.

## Agent Checklist

Before editing:

1. Read the relevant code, docs, and tests for the area.
2. Check `git status --short` and avoid overwriting unrelated user changes.
3. Verify whether README/docs claims are implemented in code before repeating them.
4. Decide whether a new feature belongs in a new module directory.
5. Plan logging points and ensure they use `tracing` with structured fields.

Before handoff:

1. Run `cargo fmt --check`.
2. Run `cargo check`.
3. Run `cargo test`.
4. Run `cargo clippy --all-targets -- -D warnings`.
5. For UI changes, run `cargo run` and compare against the relevant `docs/superpowers` visual checklist.
6. Report any commands that could not run, including exact failure reasons.
