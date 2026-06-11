# Norma Module And Logging Restructure Design

Date: 2026-06-11

## Summary

Norma needs a source layout that can scale beyond the current V1 workbench shell. The repository already has working modules for runtime bootstrap, config, logging, skills indexing, workspace loading, read-only Git status, session state, mock agent events, and GPUI views. This design reorganizes those single-file or shallow modules into domain directories and then expands structured logging at runtime boundaries.

The goal is not to preserve internal API compatibility. The goal is a clearer large-project structure that gives future model providers, tool use, structured agent input, MCP, ACP, Skills, and Subagent work meaningful homes.

This design does not implement real model calls, real provider connection tests, real tool execution, executable Skills, MCP runtime integration, ACP, destructive Git operations, or session persistence.

## Goals

- Move current top-level domain files into module directories with clear boundaries.
- Replace `src/telemetry.rs` with a `src/logging/` domain.
- Give `agent/` meaningful subdomains for runtime, mock runtime, structured input, provider abstraction, and tool use.
- Keep `main.rs` and `lib.rs` thin.
- Add structured `tracing` events around startup, config, logging, skills, workspace, Git, app state, watcher, and mock agent flows.
- Keep current behavior and tests intact unless module boundaries require intentional renaming.

## Non-Goals

- No new package manager or task runner.
- No new Cargo dependencies unless compilation exposes a concrete need.
- No UI redesign.
- No real LLM/provider execution.
- No real tool invocation.
- No destructive Git command.
- No compatibility shims for old internal module paths.

## Target Source Layout

```text
src/
  main.rs
  lib.rs
  runtime/
    mod.rs
    bootstrap.rs
    update.rs
    watchers.rs
  logging/
    mod.rs
    error.rs
    init.rs
    maintenance.rs
    writer.rs
  config/
    mod.rs
    app.rs
    file.rs
    model.rs
    reload.rs
  paths/
    mod.rs
    local.rs
  skills/
    mod.rs
    index.rs
    reload.rs
  workspace/
    mod.rs
    file_tree.rs
    project.rs
  git/
    mod.rs
    status.rs
  session/
    mod.rs
    event.rs
    inspector.rs
    state.rs
    thread.rs
  agent/
    mod.rs
    event.rs
    mock.rs
    runtime.rs
    input/
      mod.rs
      message.rs
      request.rs
      schema.rs
    provider/
      mod.rs
      capability.rs
      model.rs
      registry.rs
    tools/
      mod.rs
      definition.rs
      invocation.rs
      result.rs
  app/
    mod.rs
    state.rs
  ui/
    mod.rs
    components.rs
    execution.rs
    inspector.rs
    settings.rs
    shell.rs
    sidebar.rs
    theme.rs
```

`ui/` remains at its current component-file granularity in this change. The UI layer is already separated enough for the requested module/logging work, and deeper UI decomposition would make this refactor harder to review.

## Module Boundaries

### Runtime

`runtime/` coordinates startup and runtime messages. It owns bootstrap order, watcher setup, and runtime update enums. It does not parse config files, write logs, scan skills, or load workspace data directly except through domain APIs.

Expected files:

- `bootstrap.rs`: creates paths, loads config, initializes logging, scans skills, starts watchers, builds app state.
- `watchers.rs`: config and skills watcher setup plus debounce loops.
- `update.rs`: `RuntimeUpdate` and runtime update channel helpers.
- `mod.rs`: public exports.

### Logging

`logging/` replaces `telemetry.rs`. Business code should only call `tracing` macros; logging internals stay hidden behind this domain.

Expected files:

- `error.rs`: logging error type.
- `init.rs`: tracing subscriber setup and guard.
- `writer.rs`: rotating JSON Lines writer.
- `maintenance.rs`: compression, retention, and maintenance worker.
- `mod.rs`: public exports such as `init_tracing`, `maintain_logs`, `start_log_maintenance`, and `TelemetryGuard` or a renamed guard.

The exported guard name may remain `TelemetryGuard` for now if that reduces churn, but new code should refer to the domain as logging.

### Config

`config/` separates app settings preview models from persisted runtime configuration.

Expected files:

- `app.rs`: `AppConfig`, `SettingsSection`, provider preview models, masking, and local validation.
- `model.rs`: persisted `NormaConfig`, window/path/logging config structs, defaults, validation.
- `file.rs`: config read/write, default creation, and environment override loading.
- `reload.rs`: reload result/state and config path event filtering.
- `mod.rs`: public exports.

### Agent

`agent/` should be shaped for future real agent execution, not only for the current mock runtime.

Expected files:

- `runtime.rs`: `AgentRuntime` trait and runtime-facing contracts.
- `mock.rs`: deterministic mock implementation for the current visual contract.
- `event.rs`: agent-domain events that can later map to session/UI events.
- `input/`: structured user/task input, request context, and future schema-driven input.
- `provider/`: provider model, capability description, and registry abstractions. This prepares for future Rig or custom provider integration without performing network calls in this change.
- `tools/`: tool definition, invocation request, and result/error boundaries. This prepares for future tool use without executing tools in this change.

The initial provider, input, and tools modules can contain focused data types and empty registries where useful, but they must not imply that real provider or tool execution exists.

### Other Domains

- `paths/`: local `~/.norma` path contract and directory creation.
- `skills/`: skill directory indexing and reload state. It must not execute skills.
- `workspace/`: project opening and file-tree loading.
- `git/`: read-only Git status command and parser.
- `session/`: thread metadata, events, inspector state, and session state updates.
- `app/`: aggregate `NormaAppState` and runtime update application.

## Structured Logging Design

Logging should be added after the directory migration so the first implementation phase remains mechanically reviewable.

Log only at external boundaries and state changes, not in hot GPUI render paths.

### Runtime Logs

- `info!`: bootstrap started, runtime initialized, watchers started.
- `debug!`: watcher event received, debounce applied, update sent.
- `warn!`: watcher error, rejected config/skills reload.
- `error!`: startup failures that prevent the app from opening.

### Config Logs

- `info!`: default config written, config loaded, reload applied.
- `debug!`: environment-aware config load attempted.
- `warn!`: reload rejected, invalid config retained.

### Logging Domain Logs

- `info!`: tracing initialized, startup maintenance completed, maintenance worker started.
- `debug!`: active log opened, log rolled, file compressed, expired log removed.
- `warn!`: maintenance pass failed or compression failed for a non-critical file.

### Skills Logs

- `debug!`: scan started, skill directory entry inspected.
- `info!`: scan completed with entry count, reload applied.
- `warn!`: scan failed, reload rejected.

### Workspace And Git Logs

- `info!`: project opened.
- `debug!`: file tree load started/completed, Git status command started/parsed.
- `warn!`: project open failed, file tree fallback used, Git unavailable or non-repository.

### App And Agent Logs

- `info!`: runtime update applied, mock agent task started/completed.
- `debug!`: mock agent emitted event count, runtime snapshots attached to app state.
- `warn!`: runtime update rejected and surfaced to state.

### Field Conventions

Use stable messages and structured fields:

```rust
tracing::info!(
    component = "runtime",
    config_path = %paths.config_file.display(),
    "config loaded"
);

tracing::warn!(
    component = "config",
    path = %path.display(),
    error = %error,
    "config reload rejected"
);
```

Use `error = %error` for errors, `component = "..."` for major domains, and path fields such as `path`, `root`, `config_path`, `log_dir`, and `skills_dir`. Use counts such as `entry_count`, `file_count`, `changed_files`, `added`, and `deleted` where helpful.

Never log API keys, model tokens, MCP credentials, raw secret values, or private config contents. Provider-related logs may include provider id, protocol, and status only.

## Migration Plan

1. Create the new directory modules and move logging first because runtime depends on it.
2. Move paths, config, and skills.
3. Move workspace, git, session, and agent.
4. Move app state into `app/`.
5. Split runtime last and update all imports.
6. Run formatting and tests after the mechanical migration.
7. Add structured logs in the new modules.
8. Run final verification.

The implementation should prefer small commits by domain where possible:

- `refactor(logging): split telemetry into logging domain`
- `refactor(config): split settings and runtime config modules`
- `refactor(agent): add provider input and tool domains`
- `refactor(runtime): split bootstrap and watcher modules`
- `feat(logging): add structured runtime diagnostics`

## Testing And Verification

Required commands:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

Additional static checks:

```bash
rg -n "println!|eprintln!|dbg!" src tests
rg -n "telemetry" src
```

The first check should find no debugging output. The second check should confirm that code no longer depends on the old `telemetry` module name, except for an intentionally retained guard type if the implementation keeps that name temporarily.

Existing unit tests should move with their modules. Tests should continue to cover config validation/reload, log rotation/compression, skills scanning, file-tree behavior, Git status parsing, session state transitions, app state creation, and mock agent visual-contract events.

If a logging helper is added, tests should validate behavior and file effects rather than snapshotting entire JSON log lines.

## Risks And Mitigations

- Large diff risk: migrate mechanically first, then add logs in a second pass.
- Import churn risk: do not preserve old internal paths; fix imports directly and verify with `cargo check`.
- False capability risk: provider/tool modules are abstractions only. Do not add real network calls or executable tool behavior.
- Logging noise risk: avoid `info!` inside render paths and loops that fire frequently.
- Secret exposure risk: never log provider secrets or raw config contents.
- Git safety risk: keep Git code read-only.

## Acceptance Criteria

- Top-level source domains are directory modules rather than large single-file modules.
- `src/telemetry.rs` is replaced by a `src/logging/` domain.
- `agent/` includes meaningful subdomains for runtime, input, provider, and tools.
- `main.rs` remains a thin bootstrap.
- `lib.rs` remains module exports only.
- Runtime, config, logging, skills, workspace, Git, app state, and mock agent flows emit structured logs at appropriate levels.
- No `println!`, `eprintln!`, or `dbg!` calls exist in `src/` or tests.
- Existing behavior and tests continue to pass.
