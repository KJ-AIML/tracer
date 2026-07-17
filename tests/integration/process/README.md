# Process manager integration tests (W1-C)

Runnable fake-process lifecycle tests live in the crate:

```text
crates/tracer-process/tests/lifecycle.rs
```

Run from the worktree (no root workspace required yet):

```powershell
cargo test --manifest-path crates/tracer-process/Cargo.toml
```

## Coverage map

| Case | Failure matrix | Test |
|---|---|---|
| Spawn + started event | F-P* happy | `spawn_emits_started_and_is_process_alive_not_protocol_ready` |
| Process-alive ≠ protocol/session ready | F-A05 | same + `process_event_type_hints_never_ready` |
| Executable missing | F-P01 | `executable_missing_maps_to_not_found` |
| Invalid cwd | F-P02 | `invalid_cwd_fails_spawn` |
| Stdout capture | IO | `capture_stdout_via_take` |
| Stderr events | F-P08 shape | `capture_stderr_events` |
| Graceful stdin close | F-P09 / VS-09 | `graceful_stdin_close_exits` |
| Force kill | F-P10 | `force_kill_long_sleep` |
| Graceful timeout → force | F-P10 | `graceful_then_force_on_hang` |
| Non-zero exit | F-P05/P06 | `nonzero_exit_observed` |
| Tree kill / no orphan | F-P11, F-W01 | `force_kill_reaps_grandchild_no_orphan` |
| Platform strategy | F-W01 | `isolation_strategy_is_platform_native_when_enabled` |

## Shared workspace

When the root `Cargo.toml` workspace lands (see `docs/modules/w1-c/SHARED_MANIFEST_REQUEST.md`), these paths should be members:

- `crates/tracer-process`
- optional future harness under `tests/integration/process/` if split out
