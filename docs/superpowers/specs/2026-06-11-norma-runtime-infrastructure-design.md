# Norma Runtime Infrastructure Design

Date: 2026-06-11

## Summary

Norma needs a small runtime infrastructure layer before the GPUI workbench grows further. This layer owns local application paths, persisted configuration, structured logging, configuration hot reload, and a local Skills directory contract.

The design keeps `main.rs` thin. Startup initializes runtime infrastructure first, then constructs application state and mounts the GPUI shell. UI, workspace, Git, session, and agent modules should not assemble filesystem paths or open log files directly.

## Goals

- Create a stable local application home under `~/.norma`.
- Use `tracing` as the project-wide logging API.
- Write logs as structured JSON Lines.
- Keep logging file IO, rotation, compression, and retention off the GPUI main thread.
- Load and persist `~/.norma/config.toml` automatically on first launch.
- Watch configuration changes and update running app state/UI where safe.
- Add `~/.norma/skills` as the future local Skills loading location.

## Local Directory Contract

Norma creates this structure on first launch:

```text
~/.norma/
  config.toml
  log/
  data/
  skills/
```

Directory responsibilities:

- `config.toml`: user-editable local configuration.
- `log/`: active and compressed Norma logs.
- `data/`: future durable app data, indexes, caches, and session metadata.
- `skills/`: future local Skills installation and discovery root.

The path contract is owned by a new `paths` module. Other modules receive resolved paths from `paths` or from `NormaConfig`; they should not hard-code `~/.norma`.

## Module Boundaries

Add these focused modules:

- `paths`: resolves and creates `~/.norma`, `config.toml`, `log`, `data`, and `skills` paths.
- `config`: owns `NormaConfig`, default values, TOML read/write, validation, environment overrides, and hot reload events.
- `telemetry`: initializes `tracing`, structured JSON output, non-blocking file writing, log rotation, compression, retention, and shutdown flushing.
- `skills`: scans and indexes `~/.norma/skills`; V1 does not execute Skills.

Existing modules should use these boundaries:

- `main.rs`: calls runtime bootstrap and starts the GPUI app.
- `app_state`: holds the current config snapshot and applies config-change events.
- `ui`: renders state derived from `app_state`; it does not read `config.toml`.
- `workspace`, `git`, `session`, and `agent`: use `tracing` macros for structured events and accept config/path inputs where needed.

## Configuration

The first launch writes a default `~/.norma/config.toml`. Missing directories are created automatically.

Initial configuration shape:

```toml
[window]
width = 1440
height = 1024

[paths]
data_dir = "~/.norma/data"
log_dir = "~/.norma/log"
skills_dir = "~/.norma/skills"

[logging]
level = "info"
format = "json"
max_file_size_mb = 10
maintenance_interval_hours = 24
retention_days = 7
compress_rotated = true
```

Recommended crates:

- `config`: layered configuration, environment overrides, and typed deserialization.
- `toml`: default config serialization if direct TOML writing is needed.
- `serde`: typed config structures.
- `notify`: cross-platform file watching for `config.toml` and `skills/`.

Environment overrides should use a clear prefix such as `NORMA_`. For example, `NORMA_LOGGING_LEVEL=debug` can override `logging.level`.

Validation rules:

- `logging.level` must be one of `trace`, `debug`, `info`, `warn`, or `error`.
- `logging.format` is `json` in V1.
- `logging.max_file_size_mb` defaults to `10` and must be greater than zero.
- `logging.maintenance_interval_hours` defaults to `24` and must be greater than zero.
- `logging.retention_days` defaults to `7` and must be greater than zero.
- Window dimensions must be positive.
- Configured paths are expanded and normalized before use.

## Configuration Hot Reload

Runtime flow:

```text
~/.norma/config.toml
        |
        v
notify watcher
        |
        v
debounced reload + validation
        |
        v
ConfigChanged event
        |
        v
NormaAppState updates config snapshot
        |
        v
GPUI shell refreshes affected UI
```

Reload behavior:

- File changes are debounced to avoid duplicate reloads from editor save cycles.
- A successful reload replaces the active config snapshot and emits a structured log event.
- A failed reload keeps the previous valid config, records the validation error, and emits a structured error log.
- The UI can show a compact configuration error state in a future settings surface.

Immediately reloadable settings:

- `logging.level`
- `paths.skills_dir`
- future visual preferences and runtime labels

Restart-only settings:

- initial window size for already-open windows
- any future setting that changes bootstrap order or core path initialization

The config service should make this distinction explicit so a reload never leaves the running app in a half-applied state.

## Structured Logging

All project code logs through `tracing` macros. After telemetry initialization, any module can emit structured events:

```rust
tracing::info!(
    project = %project.name,
    root = %project.root.display(),
    "project opened"
);

tracing::warn!(
    config_path = %paths.config_file.display(),
    error = %error,
    "failed to reload config"
);
```

Logging rules:

- Use fields for machine-readable context such as `project`, `path`, `elapsed_ms`, and `event`.
- Keep the message short and stable.
- Record errors as `error = %error`.
- Never log secrets, API keys, model tokens, MCP credentials, or private config values.
- Avoid `info` logs inside high-frequency GPUI render paths; use `debug` or `trace` only when needed.

Logs are written as JSON Lines: one JSON object per line. This supports `jq`, future in-app log viewers, and external log collection.

## Log Rotation, Compression, And Retention

Logging output lives under `~/.norma/log`.

Required behavior:

- The active log file uses JSON Lines.
- A single log file has a maximum size of `10MB`.
- When the active file reaches `10MB`, logging rolls to a new file.
- Every `24` hours, Norma runs log maintenance in the background.
- Maintenance compresses rolled log files.
- Compressed logs are retained for `7` days.
- Logs older than `7` days are deleted.
- The active log file is never compressed or deleted.

Example file layout:

```text
~/.norma/log/
  norma.log
  norma.2026-06-11.0.log.gz
  norma.2026-06-11.1.log.gz
  norma.2026-06-12.0.log.gz
```

Recommended implementation path:

- Prefer `logroller` if its API satisfies JSON tracing, size rotation, compression, and retention cleanly.
- Use `tracing-subscriber` with JSON formatting.
- If `logroller` is not suitable after API verification, use `tracing-subscriber` plus a small custom rolling writer and maintenance worker.

The design should not expose the rest of the app to this choice. Business code only calls `tracing` macros.

## Non-Blocking Logging IO

Logging must not block the GPUI main thread or future agent/session workers.

Required behavior:

- Foreground code only emits `tracing` events.
- File writes go through a non-blocking writer, queue, or equivalent background writer.
- Rotation, compression, retention, and directory maintenance run on background threads or async tasks.
- Startup can create directories synchronously, but long-running maintenance must be background work.
- Shutdown keeps the writer guard alive long enough to flush pending logs where practical.
- If the log queue is full, the app should prefer dropping low-priority logs or recording an internal telemetry warning instead of blocking UI and core state updates.

## Skills Directory

`~/.norma/skills` is added now as a stable local contract for future skill loading.

V1 responsibilities:

- Create the directory on first launch.
- Scan the directory into a simple skill index.
- Watch the directory for file changes through `notify`.
- Log skill directory scan and reload results.
- Surface enough state for a future settings or capabilities panel.

V1 non-goals:

- Execute skills.
- Trust arbitrary skill code.
- Resolve skill dependencies.
- Sync remote skill registries.
- Load MCP credentials or model provider secrets from skills.

## Data Flow

Startup:

1. `main.rs` calls runtime bootstrap.
2. `paths` resolves and creates the `~/.norma` directory tree.
3. `config` creates default `config.toml` if missing, then loads and validates config.
4. `telemetry` initializes `tracing` using the active config.
5. `skills` scans `~/.norma/skills`.
6. `app_state` receives paths, config, and skill index snapshots.
7. `ui::shell` mounts the GPUI workbench.

Runtime updates:

1. `notify` reports a config or skills directory change.
2. The relevant service debounces and reloads.
3. Success emits a state-change event and structured log.
4. Failure keeps the last good state and emits a structured error log.
5. `app_state` applies the safe parts of the update.
6. UI refreshes from `app_state`.

## Error Handling

- Missing `~/.norma` directories are recreated automatically.
- Invalid config preserves the last valid runtime config.
- Log directory creation failure should be reported clearly and should fall back to stderr logging if possible.
- Log maintenance failures should emit telemetry but must not crash the app.
- Skills scan failures should be isolated to the skill index and must not prevent the workbench from opening.
- Path expansion failures should become typed config/path errors.

## Testing And Verification

Unit tests:

- `paths` creates the expected directory layout under a temporary home.
- `config` writes default TOML and reloads valid changes.
- `config` rejects invalid values without replacing the last valid config.
- `telemetry` formats logs as JSON Lines.
- `telemetry` rotates at the configured size threshold.
- `telemetry` compresses rolled logs.
- `telemetry` deletes compressed logs older than the retention window.
- `skills` scans and updates an index from a temporary skills directory.

Integration checks:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings` when Clippy is installed

Manual checks:

- Remove `~/.norma`, launch Norma, and confirm the directory layout is recreated.
- Edit `~/.norma/config.toml`, confirm valid changes update running state.
- Introduce an invalid config value, confirm the old config remains active and an error is logged.
- Generate enough logs to trigger size rotation.
- Run or simulate 24-hour maintenance and confirm old rolled logs are compressed.
- Confirm logs older than 7 days are deleted.

## Acceptance Criteria

- `main.rs` remains small and delegates runtime initialization.
- `~/.norma/config.toml`, `~/.norma/log`, `~/.norma/data`, and `~/.norma/skills` are created automatically.
- Any module can use `tracing` macros without knowing the logging backend.
- Logs are JSON Lines.
- Log writing and maintenance do not block the GPUI main thread.
- Log files roll at `10MB`.
- Rolled logs are compressed during 24-hour maintenance.
- Compressed logs older than `7` days are removed.
- Config changes are watched, validated, and applied to app state/UI where safe.
- Invalid config reloads do not replace the last good config.
- The Skills directory is created, scanned, watched, and treated as non-executable in V1.

## Out Of Scope

- Real model provider configuration.
- MCP credential management.
- Executing Skills.
- Remote skill registry sync.
- In-app visual log viewer.
- User-facing settings editor.
- Database schema design under `~/.norma/data`.
