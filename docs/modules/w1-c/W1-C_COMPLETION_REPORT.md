# W1-C Completion Report — Runtime Process Manager

| Field | Value |
|---|---|
| **Task ID** | `tracer-w1-process-manager` |
| **Work item** | W1-C |
| **Branch** | `agent/tracer-w1-process-manager` |
| **Base** | Gate 0 main `e104d8d` / tag `tracer-wave0-gate0` |
| **Heli session** | `heli-ses-ed0f4270-dfd9-400a-b9db-1c86a30f6c3a` |
| **Lease** | `heli-lease-eb78494c-d3b1-4c4f-8b6b-43df82cc8733` |
| **Host** | `grok-build` |
| **Target** | `tracer` (`repos/worktrees/tracer-w1-c`) |
| **Date** | 2026-07-17 |
| **Status** | **COMPLETE** (local commits; not pushed) |

## 1. Scope delivered

Implemented `crates/tracer-process` owning OS-level sidecar lifecycle:

| Responsibility | Implementation |
|---|---|
| Spawn configuration | `SpawnConfig` (executable, args, env, cwd, isolation, stop policy) |
| stdout / stderr | stdout handed to adapter via `take_stdout`; stderr drained → `ProcessEvent::StderrChunk` |
| Process-alive + exit signals | `ProcessPhase`, `ProcessEvent::{Started,Exited,Failed}` |
| Graceful termination | close stdin → wait → force tree kill (`StopPolicy::GracefulThenForce`) |
| Forced kill | `kill_force` + platform tree kill |
| Orphan prevention | Windows Job Object `KILL_ON_JOB_CLOSE`; Unix process group |
| Cancellation behavior | stop policies map cooperative-timeout → force (`CancellationFailed` if exit unobserved) |
| Fake-process tests | `tracer-process-test-helper` binary + `tests/lifecycle.rs` |
| process-ready ≠ session-ready | `ReadinessView` / API always `protocol_ready=false`, `session_ready=false` |

## 2. Explicit non-claims / boundaries

| Must not | Status |
|---|---|
| Parse ACP | **Not done** — no JSON-RPC / framing |
| Write session DB | **Not done** — no storage deps |
| Hardcode machine Grok paths | **Not done** — callers supply executable/args |
| Edit root workspace manifests | **Request only** — `SHARED_MANIFEST_REQUEST.md` |
| Emit `runtime.process.ready` | **Not done by design** — adapter-owned after initialize+caps |
| Claim authenticated / session-ready | **Always false** at process layer (F-A05) |

## 3. Readiness model (normative for integrators)

```text
ProcessPhase::Alive  →  OS child running, pipes open
                         == candidate for runtime.process.started
                         ≠ runtime.process.ready
                         ≠ authenticated
                         ≠ session.ready / prompts allowed
```

Control plane must compose:

1. process manager → alive  
2. ACP adapter → `runtime.process.ready`  
3. auth + `session/new` → session ready  

`ReadinessView::may_accept_prompt()` is false unless all three are true; process manager only supplies (1).

## 4. Platform orphan strategy

| OS | Strategy name | Behavior |
|---|---|---|
| Windows | `windows-job-object-kill-on-close` | CreateJobObject + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; assign child; `TerminateJobObject` on force; handle drop kills tree (F-W01, F-P11, F-P12) |
| Unix | `unix-process-group` | `process_group(0)` at spawn; `kill(-pgid, SIGKILL)` on force |
| Other | `null` | Best-effort `Child::kill` only |

## 5. Error classes (wire-aligned)

Local `ProcessErrorClass` strings match adapter contract:

- `RuntimeExecutableNotFound` (F-P01)
- `RuntimeSpawnFailed` (F-P02)
- `RuntimeCrashed` / `RuntimeDisconnected`
- `Timeout`, `CancellationFailed` (F-P10)
- `InvalidArgument`, `InternalAdapterError`

No hard dependency on `tracer-domain` yet (parallel with W1-B).

## 6. Tests

Command:

```powershell
cargo test --manifest-path crates/tracer-process/Cargo.toml
```

Result (this host, Windows): **15 passed** (2 unit readiness + 13 lifecycle).

| Test | Maps to |
|---|---|
| `spawn_emits_started_and_is_process_alive_not_protocol_ready` | start + F-A05 |
| `executable_missing_maps_to_not_found` | F-P01 |
| `invalid_cwd_fails_spawn` | F-P02 |
| `capture_stdout_via_take` | IO for adapter |
| `capture_stderr_events` | `runtime.process.stderr` shape |
| `graceful_stdin_close_exits` | F-P09 / VS-09 graceful |
| `force_kill_long_sleep` | F-P10 |
| `graceful_then_force_on_hang` | F-P10 timeout path |
| `nonzero_exit_observed` | F-P05/P06 exit observation |
| `write_stdin_roundtrip` | stdin ownership |
| `process_event_type_hints_never_ready` | no false ready events |
| `isolation_strategy_is_platform_native_when_enabled` | F-W01 strategy selected |
| `force_kill_reaps_grandchild_no_orphan` | F-P11 / F-W01 tree kill |

Index: `tests/integration/process/README.md`.

## 7. Paths touched

```text
crates/tracer-process/Cargo.toml
crates/tracer-process/src/lib.rs
crates/tracer-process/src/config.rs
crates/tracer-process/src/error.rs
crates/tracer-process/src/event.rs
crates/tracer-process/src/handle.rs
crates/tracer-process/src/ids.rs
crates/tracer-process/src/readiness.rs
crates/tracer-process/src/platform/mod.rs
crates/tracer-process/src/platform/windows.rs
crates/tracer-process/src/platform/unix.rs
crates/tracer-process/src/bin/process_test_helper.rs
crates/tracer-process/tests/lifecycle.rs
tests/integration/process/README.md
docs/modules/w1-c/W1-C_COMPLETION_REPORT.md
docs/modules/w1-c/SHARED_MANIFEST_REQUEST.md
```

## 8. Shared-manifest requests

See `docs/modules/w1-c/SHARED_MANIFEST_REQUEST.md`:

- Add `crates/tracer-process` to root workspace members when workspace lands.
- Optional later: path-dep on `tracer-domain` for shared `ErrorClass`.

## 9. Handoff notes for W1-D / W1-F

```rust
let mgr = ProcessManager::new();
let mut proc = mgr.spawn(SpawnConfig::new(exe, project_cwd).args(args))?;
// emit runtime.process.started from ProcessEvent::Started
let stdout = proc.take_stdout().unwrap();
// adapter owns framing on stdout + proc.stdin_mut()
// adapter emits runtime.process.ready after initialize — NOT process manager
// on stop:
proc.stop_default()?; // graceful stdin close then force tree kill
```

## 10. Commits

(Recorded after local commit; see git log on branch.)

## 11. Push policy

**Did not push.** Per task instructions.
