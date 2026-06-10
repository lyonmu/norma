# Repository Guidelines

## Project Structure & Module Organization

Norma is a Rust desktop-agent project built around Cargo and GPUI. The current executable entry point is `src/main.rs`. Keep application code under `src/`, grouping new modules by responsibility, for example `src/ui/`, `src/agent/`, or `src/config/` as the codebase grows. Static assets live under `asset/`; the current app icon is `asset/img/icon.png`. Package metadata and bundle/deb settings are defined in `Cargo.toml`. `README.md` and `README.zh.md` are the user-facing English and Simplified Chinese docs.

## Build, Test, and Development Commands

- `cargo run`: builds and runs the local binary for quick development checks.
- `cargo check`: type-checks the crate without producing an optimized binary; run this before submitting changes.
- `cargo build --release`: creates the release binary used by packaging metadata under `target/release/norma`.
- `cargo test`: runs unit and integration tests when present.
- `cargo fmt --check`: verifies Rust formatting without changing files.

Use `cargo fmt` to apply formatting before committing. Keep `Cargo.lock` committed because this is an application, not only a library crate.

## Coding Style & Naming Conventions

Use standard Rust formatting from `rustfmt` with 4-space indentation. Prefer descriptive module and function names in `snake_case`; use `PascalCase` for types and traits, and `SCREAMING_SNAKE_CASE` for constants. Keep public APIs small and document non-obvious behavior with concise Rust doc comments. Avoid large logic blocks in `main.rs`; move reusable behavior into modules under `src/`.

## Testing Guidelines

Place unit tests beside the code they cover using `#[cfg(test)] mod tests`. Put cross-module or CLI-style tests in `tests/` once needed. Name tests after observable behavior, such as `loads_default_config` or `renders_main_window`. At minimum, run `cargo test` and `cargo check` before opening a pull request.

## Commit & Pull Request Guidelines

This repository currently has no commit history, so no project-specific commit convention is established. Use short imperative commit messages, for example `Add GPUI window skeleton` or `Fix config loading error`. Pull requests should describe the user-visible change, list validation commands run, and include screenshots or recordings for UI changes. Link related issues when available and call out any packaging or asset changes.

## Security & Configuration Tips

Do not commit API keys, local model tokens, or private MCP server credentials. Store environment-specific configuration outside the repository and document required variables in the README when new integrations are added.
