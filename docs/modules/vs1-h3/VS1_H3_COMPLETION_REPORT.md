# VS1-H3 Completion Report

**Task id:** `tracer-vs1-soak`  
**Work item:** VS1-H3 â€” Concurrency and Persistence Soak  
**Branch:** `agent/tracer-vs1-soak`  
**Base SHA:** `15c9399c28f79bdf9c125c26f52d7bf956fb4722`  
**Head SHA:** 97e214e4a0c9fbaf64017dbfa36a0e3966f1e745  
**Session id:** `heli-ses-4b5f1483-c85a-4ca9-a2cc-a7d462d3839f`  
**Host:** grok-build  
**Target:** tracer  

## Summary

Delivered a time-bounded soak/stress suite for Vertical Slice 1 concurrency and persistence: event burst beyond bridge capacity 256, slow-DB backpressure, slow presentation consumer, concurrent command races, restart recovery, and repeated sessions. Discovered and fixed a control-plane race that rewound `next_sequence` under concurrent ingest, which caused unique-constraint storms and silent stream loss under burst. Suite **PASS** with documented observed metrics (no invented production SLAs).

## Files changed

| Path | Action | Notes |
|------|--------|-------|
| `crates/tracer-control-plane/src/session_runtime.rs` | updated | `BRIDGE_CAPACITY` export, `IngestMetrics`, soak delay hook, retry log |
| `crates/tracer-control-plane/src/plane.rs` | updated | `session_ingest_metrics`, `update_session_preserving_sequence` |
| `crates/tracer-control-plane/src/lib.rs` | updated | re-exports |
| `Cargo.toml` | updated | workspace members `tests/soak`, `tests/stress` |
| `tests/soak/**` | added | SOAK-01â€¦06 suite + thresholds |
| `tests/stress/**` | added | time-capped sequential stress |
| `tools/soak-runner/**` | added | burst fake, runner, README |
| `docs/validation/soak/VS1_SOAK_PLAN.md` | added | plan + thresholds |
| `docs/validation/soak/VS1_SOAK_RESULTS.md` | added | evidence |
| `docs/modules/vs1-h3/VS1_H3_COMPLETION_REPORT.md` | added | this report |

## Validation

| Command | Result |
|---------|--------|
| `cargo test -p tracer-vs1-soak -- --nocapture --test-threads=1` | **PASS** (7 tests) |
| `cargo test -p tracer-vs1-stress -- --nocapture --test-threads=1` | **PASS** (1 test) |
| `cargo test -p tracer-control-plane --test vs_scenarios vs01_successful_run` | **PASS** |

## Owned path compliance

- Owned: `tests/soak/`, `tests/stress/`, `tools/soak-runner/`, `docs/validation/soak/`, `docs/modules/vs1-h3/`
- Minimal control-plane touch: observability + sequence-preservation bugfix required for soak correctness
- Not touched: desktop UI, Wave 2 features, ACP adapter redesign, process manager redesign, storage driver redesign, credentials/live Grok

## Assumptions

- Fake ACP + file SQLite only (standard CI class)
- Node available on PATH
- Multi-thread Tokio for tests

## Risks / follow-ups

- Storage-level `next_sequence = MAX(...)` would be more robust than CP retry loop
- Presentation fan-out still unbounded
- Sticky `persist_failed` flag may over-report StorageError after transient failure

## Integration notes

- Merge after / with VS1 main; no Wave 2 start
- CI: add soak package with `--test-threads=1` and time budget
- Never push from worker unless authorized

## Lease

- Released: yes (end of task)
- Push: **no**

