# VS1 Soak Plan — Concurrency and Persistence (VS1-H3)

**Work item:** VS1-H3  
**Task id:** `tracer-vs1-soak`  
**Branch:** `agent/tracer-vs1-soak`  
**Base:** VS1 main `15c9399c28f79bdf9c125c26f52d7bf956fb4722`  
**CI class:** standard — network **no**, credentials **no**, live Grok **no**, provider **no**  
**Evidence:** fake ACP (stock + soak burst) + file-backed SQLite  

## Goals

Validate Vertical Slice 1 control-plane concurrency and persistence under load that stresses:

1. W1-D unbounded adapter channel drain  
2. W1-F bounded bridge (`BRIDGE_CAPACITY = 256`) and async persist pump  
3. Sole SQLite writer (W1-E) under backpressure  
4. Cancel / approval races without deadlock  
5. Restart recovery of committed history  
6. Session lifecycle hygiene (no live-registry leaks)

Out of scope: desktop UI, Wave 2 features, live Grok, redesign of ACP adapter / process manager / storage driver, unbounded secondary queues.

## Control-plane concurrency model (studied)

Source: `crates/tracer-control-plane/src/session_runtime.rs`, `docs/modules/w1-f/W1_F_CONCURRENCY_MODEL.md`.

```text
adapter unbounded mpsc (W1-D)
    -> OS drain thread (blocking_send / try_send on stop)
    -> tokio mpsc bridge (BRIDGE_CAPACITY=256)
    -> async persist pump -> SqliteStorage::append_event
    -> optional presentation fan-out (post-persist; must not block forever)
```

- Full bridge applies backpressure via `blocking_send`.  
- Stop path uses `try_send` to remain responsive.  
- Presentation failure does not fail persist.  
- Storage sequence is authoritative; adapter sequence is observation-only.

## Hard thresholds (defined before run)

| Invariant | Limit |
|-----------|------:|
| Maximum allowed event loss (silent) | **0** |
| Maximum duplicated persisted events (storage sequence) | **0** |
| Terminal events lost | **0** |
| Orphan processes after stop | **0** |
| Stale actionable approvals | **0** |
| Unjoined owned drain tasks after shutdown | **0** |

**Not claimed as production SLAs** (observed only):

- Burst drain wall-clock  
- Events/sec under burst  
- Cancel latency under slow persist  
- SQLite file growth per N sessions  

## Scenarios

| ID | Name | Driver | Measures |
|----|------|--------|----------|
| SOAK-01 | Event burst | `tools/soak-runner/burst-fake-acp.js` with `TRACER_SOAK_BURST_COUNT=600` (>256) | Source backlog drain, bridge saturation, drain/SQLite throughput, terminal delivery, ordering, drop=0, shutdown duration |
| SOAK-02 | Slow database | `TRACER_SOAK_PERSIST_DELAY_MS=5` + burst 320 | Predictable backpressure, no deadlock, no silent loss, terminal observable, cancel responsive, memory finite |
| SOAK-03 | Slow presentation | Presentation `mpsc` with no reader + burst 400 | Persist continues; fan-out does not block pump |
| SOAK-04 | Concurrent commands | permission_hold + cancel_mid_stream + happy | Repeated cancel; duplicate approval; cancel vs approval race; shutdown during prompt; snapshot/history during ingestion |
| SOAK-05 | Restart recovery | Burst mid-flight, drop CP, reopen file DB | Migrations valid, committed ordering, incomplete→interrupted/disconnected, stale approval not actionable, no corruption |
| SOAK-06 | Repeated sessions | 12× `happy_prompt_stream` sequential | No live-registry leak after stop, DB growth expected, completion projection present |
| STRESS-01 | Sequential stress | 20 sessions or 180s budget | Time-capped growth; no hang |

## Instrumentation (minimal)

Owned path preference: measure from harness. Minimal control-plane hooks only:

| Hook | Location | Purpose |
|------|----------|---------|
| `BRIDGE_CAPACITY` public | `session_runtime.rs` | Size bursts relative to bridge |
| `IngestMetrics` atomics | drain + pump | bridge_accepted, events_persisted, persist_errors, presentation_* |
| `ControlPlane::session_ingest_metrics` | `plane.rs` | Test read of counters |
| `TRACER_SOAK_PERSIST_DELAY_MS` | `persist_one` | Controlled slow-DB injection without redesigning storage |

## How to run

```powershell
pwsh -File tools/soak-runner/run-soak.ps1
# or
cargo test -p tracer-vs1-soak -- --nocapture --test-threads=1
cargo test -p tracer-vs1-stress -- --nocapture --test-threads=1
```

See `tools/soak-runner/README.md`.

## Deliverables

- `docs/validation/soak/VS1_SOAK_PLAN.md` (this file)  
- `docs/validation/soak/VS1_SOAK_RESULTS.md`  
- `docs/modules/vs1-h3/VS1_H3_COMPLETION_REPORT.md`  
- `tests/soak/`, `tests/stress/`, `tools/soak-runner/`  

## Assumptions

1. Node is on PATH for fake ACP processes.  
2. Multi-thread Tokio test runtime is required so the async persist pump runs while prompt blocks.  
3. Soak burst fake is not a catalog scenario; it lives under owned `tools/soak-runner/` so stock fake-acp-runtime remains untouched.  
4. `TRACER_SOAK_PERSIST_DELAY_MS` is soak-only; production leaves it unset.  
5. Presentation fan-out uses unbounded `std::mpsc`; “slow consumer” validates non-blocking of persist, not bounded presentation memory (documented risk).  

## Risks

| Risk | Mitigation |
|------|------------|
| Windows process spawn contention | Global soak mutex; `--test-threads=1` |
| OnceLock env cache | Persist delay reads env each event |
| Burst fake protocol drift vs stock fake | Minimal initialize/session/prompt subset aligned with W1-D normalize |
| Aborting prompt task mid-flight on Windows | Soak-05 allows empty committed set if crash before first commit; asserts reopen safety |
