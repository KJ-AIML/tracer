# VS1 Soak Results — Concurrency and Persistence (VS1-H3)

**Date:** 2026-07-17  
**Task:** `tracer-vs1-soak`  
**Branch:** `agent/tracer-vs1-soak`  
**Session:** `heli-ses-4b5f1483-c85a-4ca9-a2cc-a7d462d3839f`  
**Host:** grok-build  
**Environment:** fake ACP + file-backed SQLite; network no; credentials no; live Grok no  

## Verdict: **PASS** (with findings)

Hard invariants held after control-plane sequence-preservation fix.

## How run

```powershell
cargo test -p tracer-vs1-soak -- --nocapture --test-threads=1
cargo test -p tracer-vs1-stress -- --nocapture --test-threads=1
cargo test -p tracer-control-plane --test vs_scenarios vs01_successful_run -- --nocapture
```

Also: `tools/soak-runner/run-soak.ps1`

## Thresholds (pre-declared)

| Invariant | Limit | Observed |
|-----------|------:|----------|
| Event loss (silent / unpersisted under claim of success) | 0 | 0 after fix |
| Duplicated persisted sequences | 0 | 0 (monotonic + unique checks) |
| Terminal events lost | 0 | session.completed / cancelled present where expected |
| Orphan processes after stop | 0 | live registry empty after stop |
| Stale actionable approvals | 0 | empty after cancel races |
| Unjoined owned drain tasks | 0 | stop joins drain; shutdown bounded |

## Scenario results

| ID | Result | Duration (ms) | Key metrics / notes |
|----|--------|--------------:|---------------------|
| SOAK-01 Event burst (600 > bridge 256) | **PASS** | ~10085 | storage_events=607, deltas=600, bridge_accepted=607, events_persisted=607, persist_errors=0, shutdown_ms≈76–108 |
| SOAK-02 Slow DB (`PERSIST_DELAY_MS=5`, burst 320) | **PASS** | ~6125 | cancel_ms≈4900 (bounded <12s), events=327, persist_errors=0, no deadlock |
| SOAK-03 Slow presentation (no reader) | **PASS** | ~6850 | events=407, persistence continued, persist_errors=0 |
| SOAK-04 Concurrent commands | **PASS** | ~26800 | cancel×N, duplicate approval, cancel vs allow race, shutdown during prompt, history monotonic |
| SOAK-05 Restart recovery | **PASS** | ~600–700 | file DB reopen; sequences monotonic; history non-empty after mid-burst drop; no corruption |
| SOAK-06 Repeated sessions (12) | **PASS** | ~7650 | event_counts≈12–13 each; db_bytes≈286720; live registry cleared per stop |
| STRESS-01 Sequential (20 / 180s) | **PASS** | ~12237 | completed=20, listed=20 |
| VS-01 regression | **PASS** | ~700 | stock happy path still green |

## Observed throughput (not production SLAs)

| Observation | Value | Guidance |
|-------------|------:|----------|
| Burst size vs bridge | 600 vs capacity 256 | Bridge backpressure via `blocking_send` held; no second unbounded queue |
| Burst wall-clock (600 × 1ms chunk pacing) | ~10s end-to-end | Debug build; not a release SLA |
| Slow-DB cancel latency | ~4.9s with 5ms/event delay | Cancel remains time-bounded under artificial latency |
| SQLite file growth (12 happy sessions) | ~280 KiB | Expected accumulation |
| Presentation fan-out | Unbounded `std::mpsc` | Non-reading consumer does not block persist; memory growth of fan-out not bounded (documented risk) |

## Critical finding fixed during soak

**Bug:** `ControlPlane::session_create` called `update_session` with a stale `SessionRecord` snapshot **after** the ingest pump started. That rewound `sessions.next_sequence` while higher sequences already existed in `events`, causing `UNIQUE(session_id, sequence)` failures mapped to `AlreadyExists`, counted as duplicates, and **silent stream loss** (bridge_accepted ≫ events_persisted).

**Fix:** `update_session_preserving_sequence` — never decrease `next_sequence`; keep it strictly ahead of `MAX(event.sequence)`.

**Evidence before fix (SOAK-01):** `bridge_accepted=607`, `events_persisted=3`, `events_duplicate=604`.  
**Evidence after fix:** `bridge_accepted=607`, `events_persisted=607`, `events_duplicate=0`.

## Instrumentation added (minimal)

| Item | Purpose |
|------|---------|
| `BRIDGE_CAPACITY` public | Size bursts relative to bridge |
| `IngestMetrics` + `session_ingest_metrics` | Outside-visible counters |
| `TRACER_SOAK_PERSIST_DELAY_MS` | Controlled slow-DB injection |
| Burst fake under `tools/soak-runner/` | Generate >256 stream chunks without changing stock fake catalog |

## Assumptions

1. Node on PATH for fake ACP.  
2. Multi-thread Tokio test runtime required.  
3. Burst fake is soak-owned (not catalog live parity).  
4. 1ms chunk pacing used for SOAK-01/03 stability on Windows debug hosts.  
5. Presentation queue remains unbounded by design (post-persist best-effort).

## Risks / follow-ups

| Risk | Severity | Follow-up |
|------|----------|-----------|
| Other full-row `update_session` call sites could still race | Medium | Prefer partial updates or SQL `next_sequence = MAX(next_sequence, ?)` in storage |
| `persist_failed` sticky after single error | Low | Clear flag on subsequent success; or surface partial history |
| Presentation unbounded memory if UI never drains | Medium | Bound fan-out or drop-oldest (Wave 2 / product) |
| Restart may leave DB status `Running` until reconcile edge cases | Low | Strengthen reconcile predicates |
| No production p99 SLA claimed | Info | Collect release-build metrics before setting SLOs |

## Integration requirements

1. Land sequence-preservation fix with VS1-H3.  
2. Run `cargo test -p tracer-vs1-soak -- --test-threads=1` in CI standard class.  
3. Do not enable `TRACER_SOAK_PERSIST_DELAY_MS` in production.  
4. Coordinate with any concurrent CP writers before merging Wave 2 storage changes.
