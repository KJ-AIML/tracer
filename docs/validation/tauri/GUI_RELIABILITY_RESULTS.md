# GUI Reliability Results (W2.3-C)

**Task:** `tracer-w2-gui-reliability`  
**Branch:** `agent/tracer-w2-gui-reliability`  
**Host:** Windows GUI runner (manual_local / windows_gui_runner)  
**Date:** 2026-07-18  
**Batch ID:** `repeat-2026-07-18T13-38-58-034Z-31336`  
**Evidence:** `artifacts/tauri-e2e/repeat-2026-07-18T13-38-58-034Z-31336/repeat-gui-report.json` (gitignored)

## Objective

≥5 consecutive first-attempt full L3-J suites (GJ-01…12) with fresh env each run; product assertion failures=0; orphans=0; port collisions=0; temp cleanup failures=0; unsanitized artifacts=0.

## Result: **PASS (5/5 consecutive)**

| Metric | Value |
|---|---|
| Consecutive first-attempt PASS | **5/5** |
| Product assertion failures | **0** |
| Orphans | **0** |
| Port collisions (failed) | **0** |
| Temp cleanup failures | **0** |
| Unlimited retries | **false** |
| Live provider / credentials | **false** |
| Objective met | **true** |

## Per-run summary

| Run | Run ID | Result | Journeys | Duration (ms) | Driver (ms) | App ready (ms) | Suite (ms) | Shutdown (ms) | Orphans |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `l3j-2026-07-18T13-39-01-436Z-21724` | PASS | 12/12 | 55352 | 347 | 1522 | 49169 | 4769 | 0 |
| 2 | `l3j-2026-07-18T13-39-58-415Z-34168` | PASS | 12/12 | 55645 | 342 | 1281 | 49857 | 4749 | 0 |
| 3 | `l3j-2026-07-18T13-40-52-121Z-24828` | PASS | 12/12 | 48572 | 339 | 932 | 43046 | 4214 | 0 |
| 4 | `l3j-2026-07-18T13-41-41-386Z-40560` | PASS | 12/12 | 51563 | 340 | 1346 | 45228 | 4479 | 0 |
| 5 | `l3j-2026-07-18T13-42-32-937Z-24032` | PASS | 12/12 | 50532 | 343 | 1053 | 43538 | 3949 | 0 |

Each run: unique workDir + SQLite, unique preferred port base, fresh fake ACP, first attempt only (`retries=0`), doctor READY preflight.

## Journey coverage (every run)

GJ-01…GJ-12 all **PASS** on each of the five runs (startup, create session, streaming, approval allow/deny, cancel, multi-session, crash/EOF, restart restore, Heli unavailable, fail-closed invoke, clean shutdown).

## Failure injection

`pnpm test:tauri-e2e:inject-fail` → **PASS (113/113)** covering C6 modes including app launch, tauri-driver/msedgedriver startup, root marker, fake runtime, SQLite, forced GUI assert, shutdown timeout, stale Edge, plus artifact/port/orphan/mid-journey cases.

## Reliability self-test

`pnpm test:tauri-e2e:reliability` → **PASS (18/18)**

## Timing observations (C5)

- Driver startup ~340ms; app readiness ~0.9–1.5s; full suite ~43–50s; shutdown ~4–5s  
- Fixed sleeps replaced with `waitUntil` / poll-based readiness for driver ready, app ready, desktop exit, driver exit  
- Wait policy documented in `WAIT_POLICY` (`tools/tauri-e2e/lib/reliability.mjs`) and `W2_3_C_TEST_MATRIX.md`

## Process / artifact cleanup

- Orphan verify after each run: OK  
- Temp dirs cleaned on PASS  
- Artifact sanitization + audit in harness path  

## Runner classification

`windows_gui_runner | platform_gated_ci | manual_local`  
**Not** part of `pnpm -r test` / `cargo test --workspace`.  
Contract: `docs/validation/tauri/WINDOWS_GUI_RUNNER_CONTRACT.md`
