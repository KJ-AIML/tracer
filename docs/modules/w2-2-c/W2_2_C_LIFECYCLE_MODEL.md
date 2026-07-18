# W2.2-C — Drain Lifecycle Model

**Task:** `tracer-w2-drain-lifecycle`  
**Owned code:** `crates/tracer-control-plane/src/session_runtime.rs`, `crates/tracer-control-plane/src/session/`  
**Date:** 2026-07-18

## Problem statement

After an adapter prompt operation returns, residual drain may still observe late events and channel close. Without a clear lifecycle model, those observations can be misclassified as **false `persist_errors`**, tear down ingestion too early, or present terminal UI before durable persistence.

## Authoritative completion

**Adapter / prompt RPC return is not authoritative for ingestion completion.**

Authoritative order for a prompt cycle:

```text
runtime started
  → event drain active
  → prompt active
  → adapter terminal event observed
  → terminal persisted          ← durable truth for history
  → terminal state committed    ← presentation may reflect terminal (post-persist only)
  → adapter operation returned  ← may race ahead of the three lines above
  → late-event grace / drain
  → source closed or bounded drain complete
  → drain task joined
  → pump joined
  → runtime shutdown
```

| Phase | Owner | Notes |
|---|---|---|
| `EventDrainActive` | `LiveSession::start_ingestor` | OS drain + async pump |
| `PromptActive` | `session_submit_prompt` / `begin_prompt_cycle` | Clears prior-run sticky terminal flags |
| `AdapterTerminalObserved` | persist pump (pre-append) | First prompt-terminal type seen |
| `TerminalPersisted` | successful `append_event` | Storage-authoritative |
| `TerminalStateCommitted` | post-persist state + presentation | **Never pre-persist** |
| `AdapterOperationReturned` | plane after RPC | Does **not** stop ingest |
| `LateEventGrace` | after return and/or terminal commit | Late frames still accepted under policy |
| `SourceClosedOrBoundedDrainComplete` | stop signal / channel disconnect | Expected lifecycle |
| `DrainTaskJoined` / pump join | `stop_ingestor(_async)` | Required before registry drop |
| `RuntimeShutdown` | stop complete | Safe for process teardown |

## Dual-stage ingest (unchanged architecture)

```text
adapter unbounded receiver
  → OS drain thread
  → bounded tokio mpsc (BRIDGE_CAPACITY=256)
  → async persist pump → SqliteStorage::append_event
  → presentation hub publish (post-persist only)
```

## Event-after-terminal policy

| Case | Disposition | Metrics |
|---|---|---|
| Duplicate same terminal type | Persist if reached storage; **no status churn** | `late_duplicate_terminals` |
| Late non-terminal (deltas, tools, `prompt.submitted`) | Persist; **no status regression** | `late_events_*` |
| Late metadata / protocol unknown | Persist; no reopen of run | `late_events_*` |
| Process exit / crash after terminal | **Apply fully** (may upgrade to Failed) | normal path |
| Channel close without terminal | Drain exits; **not** `persist_error` | `channel_closes` |
| Adapter return before terminal | Ingest continues; plane waits briefly for `prompt_in_flight` / terminal | phase `AdapterOperationReturned` |
| Terminal after cancel | Cancel may set Stopped; late terminal applies under policy | — |
| Storage fail on terminal write | `persist_errors++`, `persist_failed=true`; no presentation of success | real failure |

## Metrics contract

| Counter | Counts |
|---|---|
| `persist_errors` | **Real** storage failures after retry budget only |
| `channel_closes` | Expected adapter event-source disconnect |
| `bridge_send_failures` | Bridge closed / send fail (lifecycle, not storage) |
| `events_duplicate` | Unrecoverable UNIQUE after retries (not false storage fail) |
| `terminal_persisted` | Prompt-cycle terminal rows committed |
| `late_events_observed` / `late_events_applied` | Post-terminal traffic |
| `drain_joins` / `pump_joins` / `pump_aborts` | Stop path observability |

## Stop / shutdown join rules

1. Signal `stop_ingest`.
2. Join OS drain thread (drops bridge sender → pump sees EOF).
3. **Join** async pump with `LATE_DRAIN_JOIN_TIMEOUT` (5s); abort only on timeout.
4. Advance phase to `RuntimeShutdown`.

`session_stop` and create-failure paths use `stop_ingestor_async`. `Drop` uses the sync join/abort path so tasks do not leak.

## Multi-session isolation

- Per-session drain/pump/metrics/state.
- Concurrent multi-session append uses **bounded retries** so WAL/unique races do not inflate `persist_errors`.
- Sticky `persist_failed` / force-inject on one session must not poison peers.

## Test inject

`set_test_force_persist_error(bool)` (and legacy env `TRACER_TEST_FORCE_PERSIST_ERROR`) forces real failure counting without redesigning SQLite — proves failures remain observable.
