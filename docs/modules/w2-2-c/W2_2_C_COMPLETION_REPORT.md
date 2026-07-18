# W2.2-C Completion Report — Drain Lifecycle Hardening

**Task:** `tracer-w2-drain-lifecycle`  
**Work item:** W2.2-C  
**Branch:** `agent/tracer-w2-drain-lifecycle`  
**Session:** `heli-ses-ed6c1260-f8bd-4219-9451-a385c4f34571`  
**Date:** 2026-07-18  

### Commit SHAs

| Commit | Summary |
|---|---|
| `0589b27` | feat: harden drain lifecycle + integration suite |
| `469e01c` | test: stress drain lifecycle |
| `d8d7a7c` | docs: model, matrix, completion, results |

## Summary

Hardened post-adapter-return drain lifecycle so that:

1. Prompt/adapter return does **not** end ingestion early.
2. Terminal presentation remains **post-persist only**.
3. Expected channel close and recoverable SQLite contention do **not** inflate `persist_errors`.
4. Real storage failures remain counted and observable.
5. Stop/shutdown **joins** drain and pump (abort only on timeout).
6. Multi-session drains stay isolated (no cross-session poison).

## Classification of residual risk (pre-fix)

| Hypothesis | Finding |
|---|---|
| Expected late-drain | Confirmed: adapter return races terminal drain; ingest must continue |
| Ordering race | Confirmed under multi-session file SQLite UNIQUE/busy contention |
| Premature shutdown | Confirmed: prior `stop_ingestor` aborted pump without join wait |
| Incorrect terminal ack | Not primary — terminal types were applied, but phase was implicit |
| Channel lifecycle ambiguity | Confirmed: disconnect was silent; now `channel_closes` |
| Real persistence failure | Still real; force-inject proves metrics count only true failures after budget |

**Primary false-persist-error sources addressed:**

- Recoverable `AlreadyExists` / sequence races counted after single retry → now multi-attempt; exhausted UNIQUE → `events_duplicate`, not `persist_error`.
- Transient DB lock errors under concurrent session pumps → exponential backoff retries (8 attempts).
- Stop path aborting mid-persist → join-first with timeout.

## Code changes

| Path | Change |
|---|---|
| `crates/tracer-control-plane/src/session/lifecycle.rs` | Phases, late-event policy, constants |
| `crates/tracer-control-plane/src/session/mod.rs` | Module export |
| `crates/tracer-control-plane/src/session_runtime.rs` | Metrics, join-safe stop, late policy, retries, force inject |
| `crates/tracer-control-plane/src/plane.rs` | `begin_prompt_cycle`, mark return, async stop |
| `crates/tracer-control-plane/src/lib.rs` | Export session lifecycle + inject API |
| `crates/tracer-control-plane/tests/drain_lifecycle.rs` | 14 named cases |
| `tests/stress/src/stress_drain_lifecycle.rs` | Stress suite |
| `tests/stress/Cargo.toml` | Register stress test (no root workspace change) |

## Forbidden paths (not touched)

- `apps/desktop` / Tauri E2E (W2.2-A)
- Full WebView journey (W2.2-B)
- Domain / process / storage schema redesign
- Presentation hub redesign beyond post-persist interaction
- Live Grok

## Verification

| Command | Result |
|---|---|
| `cargo test -p tracer-control-plane --test drain_lifecycle -- --test-threads=1` | **14/14 PASS** |
| `cargo test -p tracer-control-plane --lib session::lifecycle` | **5/5 PASS** |
| `cargo test -p tracer-vs1-stress --test stress_drain_lifecycle -- --test-threads=1` | **3/3 PASS** |
| `cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1` | **23/23 PASS** |

## False-persist-error result

| Path | `persist_errors` expectation | Proven |
|---|---|---|
| Happy single session | 0 | Yes |
| Happy multi-session concurrent | 0 after retry hardening | Yes |
| Normal channel close / stop | 0 | Yes |
| Forced storage failure inject | >0 | Yes |
| Peer session after force-fail | 0 | Yes |
| Stress repeated / overlap / cancel-shutdown | 0 false PE | Yes |

## Integration requirements (handoff)

1. **Do not** treat prompt RPC return as “ingestion complete” in UI or higher layers; poll storage / presentation revision after return.
2. Metrics consumers must treat `channel_closes` / `bridge_send_failures` as lifecycle, **not** as storage health.
3. Only `persist_errors` (and session `persist_failed` / last_error) indicate durable write failure.
4. `shutdown_all` / `session_stop` are join-safe; callers should not spawn parallel forced process kills that race join without need.
5. Test inject `set_test_force_persist_error` is **test-only**; never enable in production product paths.
6. Prefer `--test-threads=1` for fake-ACP Windows suites (existing convention).

## Docs

- `docs/modules/w2-2-c/W2_2_C_LIFECYCLE_MODEL.md`
- `docs/modules/w2-2-c/W2_2_C_TEST_MATRIX.md`
- `docs/modules/w2-2-c/W2_2_C_COMPLETION_REPORT.md`
- `docs/validation/drain-lifecycle/DRAIN_LIFECYCLE_RESULTS.md`
