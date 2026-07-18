# Drain Lifecycle Contract (Gate 2.2.1)

**Authority:** Integrated from W2.2-C into `main` via Gate 2.2.1.  
**Code owners:** `crates/tracer-control-plane/src/session/lifecycle.rs`, `session_runtime.rs`, `plane.rs`  
**Detail model:** `docs/modules/w2-2-c/W2_2_C_LIFECYCLE_MODEL.md`  
**Date:** 2026-07-18

## Core rule

**Adapter / prompt RPC return is not authoritative for ingestion completion.**

UI and higher layers must not treat prompt acceptance / RPC return as “run finished.” Durable truth is storage (and presentation revision after post-persist publish).

## Authoritative order

```text
runtime started
  → event drain active
  → prompt active
  → adapter terminal event observed
  → terminal persisted              ← durable history
  → terminal state committed        ← presentation may show terminal
  → adapter operation returned      ← may race ahead of the three lines above
  → late-event grace / drain
  → source closed or bounded drain complete
  → drain task joined
  → pump joined
  → runtime shutdown
```

## Metrics contract

| Counter | Meaning | Misuse to avoid |
|---|---|---|
| `persist_errors` | Real storage failures after retry budget | Do not treat as “busy drain” or channel close |
| `channel_closes` | Expected event-source disconnect | Lifecycle, not storage health |
| `bridge_send_failures` | Bridge closed / send fail | Lifecycle, not storage health |
| `events_duplicate` | UNIQUE exhausted after retries | Not a silent false PE |
| `terminal_persisted` | Terminal rows committed | Progress after return |
| `late_events_*` | Post-terminal traffic | Expected under grace |
| `drain_joins` / `pump_joins` / `pump_aborts` | Stop path | Abort only after join timeout |

## False persist-error matrix (Gate 2.2.1 proven)

| Path | Expected `persist_errors` | Proven |
|---|---|---|
| Happy single session | 0 | Yes |
| Happy multi-session concurrent | 0 | Yes |
| Normal channel close / stop | 0 | Yes |
| Stress repeated / overlap / cancel-shutdown | 0 false PE | Yes |
| Forced `set_test_force_persist_error(true)` | >0 | Yes (true positive) |
| Peer session after peer force-fail | 0 | Yes |

## Stop / shutdown

1. Signal stop ingest.  
2. Join OS drain (drops bridge sender).  
3. Join async pump with `LATE_DRAIN_JOIN_TIMEOUT` (5s); abort only on timeout.  
4. Advance to `RuntimeShutdown`.

Callers must not race process kill with join without need. `shutdown_all` joins every live session drain.

## Multi-session isolation

- Per-session drain, pump, metrics, and sticky `persist_failed`.
- Concurrent append uses bounded retries so WAL / UNIQUE races do not inflate `persist_errors`.
- One session’s force-fail or failure must not poison peers.

## Test inject (test-only)

`set_test_force_persist_error(bool)` and env `TRACER_TEST_FORCE_PERSIST_ERROR` prove real failures remain countable. **Never enable in production product paths.**

## Integration obligations for consumers

1. Poll storage / presentation revision after prompt return.  
2. Treat only `persist_errors` / session `persist_failed` / `last_error` as durable write failure.  
3. Prefer `--test-threads=1` for fake-ACP Windows suites.  
4. Evidence class for gate validation: **fake ACP + file SQLite only** (no network, no live Grok).

## Gate 2.2.1 verification commands

```powershell
cargo test -p tracer-control-plane --test drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --lib session::lifecycle -- --test-threads=1
cargo test -p tracer-vs1-stress --test stress_drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
cargo test -p tracer-control-plane --test multi_session -- --test-threads=1
cargo test -p tracer-vs1-soak -- --test-threads=1
```

**Gate status:** PASS (see `WAVE_2_2_1_TEST_MATRIX.md`).
