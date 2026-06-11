# Repository Guidelines

## Project Summary

Norma is a Rust 2024 desktop-agent project. The current implementation is still a minimal binary (`src/main.rs` prints `Hello, world!`), while `README.md`, `README.zh.md`, and `Cargo.toml` describe the intended GPUI-based agent product. Treat the code as the source of truth: do not claim multi-model, MCP, ACP, Skills, sub-agent, or multi-agent runtime support is implemented until the corresponding Rust modules exist.

The active V1 direction is documented under `docs/superpowers/`: a GPUI project workbench shell with real local project/file and read-only Git context, mock agent events, and disabled or preview-only review actions.

## Tech Stack & Dependencies

- Language/tooling: Rust 2024, Cargo, rustfmt.
- Runtime/UI dependency: `gpui`; `Cargo.lock` currently resolves it to `0.2.2`.
- Packaging metadata: `Cargo.toml` contains `package.metadata.bundle` and `package.metadata.deb`.
- Assets: current visible app asset is `asset/img/icon.png`.

Do not reintroduce `gpui_platform`; the public dependency used by this crate is `gpui`.

## Repository Layout

- `src/main.rs`: current executable entry point. Keep it small as the app grows.
- `asset/`: static assets; preserve existing icons unless intentionally updating branding.
- `docs/superpowers/specs/`: product/design specs and reference assets.
- `docs/superpowers/plans/`: implementation plans. Follow these when implementing the V1 workbench.
- `README.md` and `README.zh.md`: user-facing docs.
- `Cargo.toml` and `Cargo.lock`: crate config, dependencies, and packaging metadata.
- `target/`: build output; never edit or commit generated files from here.

## Commands

- `cargo run`: build and run the current binary.
- `cargo check`: type-check the crate; run before finishing code changes.
- `cargo test`: run unit/integration tests when present.
- `cargo fmt --check`: verify formatting.
- `cargo fmt`: apply Rust formatting.
- `cargo clippy --all-targets -- -D warnings`: lint when the Clippy component is installed.
- `cargo build --release`: build the release binary referenced by packaging metadata.

There are no Make, npm, pnpm, or shell task scripts in the current repository.

## Code Style & Architecture

Use standard Rust naming: `snake_case` functions/modules, `PascalCase` types/traits, and `SCREAMING_SNAKE_CASE` constants. Keep public APIs small and document non-obvious behavior with concise doc comments.

As the app grows, avoid large logic blocks in `main.rs`. Prefer focused modules aligned with the V1 plan, such as `ui`, `workspace`, `git`, `session`, `agent`, `config`, and app state. In V1, Git operations must be read-only unless a later spec explicitly changes that boundary; destructive actions such as reset, checkout, discard, or patch application should remain disabled or preview-only.

## Agent Rules

- Inspect the current checkout before editing; avoid copying README claims into code or docs as implemented facts.
- Preserve `Cargo.lock`; this is an application crate.
- Keep README language pairs in sync when changing user-facing behavior.
- For UI work, respect the visual contract in `docs/superpowers/specs/2026-06-11-norma-project-workbench-design.md` and the reference image beside it.
- Do not commit secrets, API keys, model tokens, MCP credentials, or machine-local config.
- Be cautious when editing `Cargo.toml` packaging metadata, `asset/`, and `docs/superpowers/`; these files define product direction, packaging, or visual targets.

## Pre-Submission Checklist

Before handing off code changes:

1. Run `cargo fmt --check`.
2. Run `cargo check`.
3. Run `cargo test`.
4. Run `cargo clippy --all-targets -- -D warnings` when available.
5. For UI changes, compare the GPUI window against the relevant `docs/superpowers` spec or checklist and include screenshots/notes.
6. State any command that could not run and the exact reason.
