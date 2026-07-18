# Wave 2.1 Test Matrix

**Gate:** 2.1 · **Branch:** `integration/tracer-w2-1` · **Date:** 2026-07-18  
**CI class:** standard — network no · credentials no · live Grok no · fake ACP yes

## Control plane — presentation (W2-A)

| ID | Test | Result | Notes |
|---|---|---|---|
| INV-01 | Persistence independent of presentation | PASS | |
| INV-02 | Slow/absent consumer no unbounded growth | PASS | capacity=1 |
| INV-03 | Latest state via snapshot | PASS | |
| INV-04 | Terminal cannot be permanently missed | PASS | sticky terminal |
| INV-05 | Notification duplication harmless | PASS | |
| INV-06 | Notification loss recoverable via snapshot | PASS | |
| INV-07 | Snapshot revisions monotonic | PASS | |
| INV-08 | Consumer detects stale snapshot | PASS | |
| INV-09 | Multiple consumers cannot block publish | PASS | |
| INV-10 | Disconnect removes delivery state | PASS | |
| INV-11 | Shutdown clears consumers | PASS | |
| INV-12 | VS happy-path ordering smoke | PASS | |
| + | Burst coalesce, reconnect, multi-consumer, legacy, fake path | PASS | 19 total |

## Control plane — multi-session (W2-C + W2.1)

| ID | Test | Result |
|---|---|---|
| MS-01 | Sequential many sessions | PASS |
| MS-02 | Presentation focus switch | PASS |
| MS-03 | History while other ingests | PASS |
| MS-04 | Cancel isolation | PASS |
| MS-05 | Approval cross-session rejected | PASS |
| MS-06 | Runtime/session ids distinct | PASS |
| MS-07 | Sequences session-local monotonic | PASS |
| MS-08 | Failed session does not poison | PASS |
| MS-09 | persist_failed session-local | PASS |
| MS-10 | Restart restores completed | PASS |
| MS-11 | Interrupted recover independently | PASS |
| MS-12 | Stale approvals per session | PASS |
| MS-13 | No live registry leaks | PASS |
| MS-14 | shutdown_all deterministic | PASS |
| MS-15 | One prompt per session | PASS |
| MS-16 | Parallel prompts across sessions | PASS |
| MS-17 | Focus stable under background ingest | PASS |

## Desktop boundary (W2-B + W2.1)

| ID | Test | Result | Class |
|---|---|---|---|
| A1 | Registered commands stable (+ focus) | PASS | L0 |
| A2 | App info + snapshot via plane handlers | PASS | L0 |
| Journey happy / approval / cancel / reopen / heli | PASS | L1 |
| Journey multi-session focus switch | PASS | L1 |
| Invoke policy fail-closed | PASS | L0 |
| `node tools/tauri-e2e/run.mjs` | PASS | L1 harness |
| Full WebView GUI drive | NOT RUN | L2/L3 blocked |

## Soak / stress

| Suite | Result | Notes |
|---|---|---|
| tracer-vs1-soak (8) | PASS | `--test-threads=1` |
| tracer-vs1-stress multi-session (3) | PASS | |
| vs_scenarios (23) | PASS | |

## Live approval (W2-D)

| ID | Status | Evidence |
|---|---|---|
| LVA harness unit/dry-run | PASS | live-grok-smoke 24 tests |
| LVA-01..04 live reverse-request | **NOT_OBSERVED** | no credentials / no live provider |
| Live overall | **PARTIAL** | opt-in harness only |

## Aggregate

| Layer | Pass | Fail | Notes |
|---|---|---|---|
| cargo test --workspace | ALL | 0 | serial threads for determinism |
| pnpm -r test/build | ALL | 0 | |
| Gate decision | **PASS** | | |