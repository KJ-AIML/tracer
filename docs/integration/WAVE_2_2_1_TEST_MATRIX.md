# Wave 2.2.1 Test Matrix

**Gate:** 2.2.1 · **Branch:** `integration/tracer-w2-2-1` · **Date:** 2026-07-18  
**CI class:** standard — network no · credentials no · live Grok no · fake ACP yes  
**Platform-gated:** L2 smoke / L3-I (Windows GUI host)

## Drain lifecycle (W2.2-C)

| # | Test | Invariant | Result |
|---|---|---|---|
| 1 | `prompt_return_before_terminal_drain` | Prompt return ≠ end of ingestion | PASS |
| 2 | `terminal_persisted_before_completion_presentation` | Terminal UI only after persist | PASS |
| 3 | `normal_channel_close_does_not_increment_persist_errors` | Channel close ≠ PE | PASS |
| 4 | `real_storage_error_increments_persist_errors` | Real failure observable | PASS |
| 5 | `late_metadata_event_policy` | Late metadata no reopen | PASS |
| 6 | `late_non_terminal_event_policy` | Late non-terminal no status regression | PASS |
| 7 | `duplicate_terminal_event_policy` | Dup terminal no status churn | PASS |
| 8 | `cancel_during_late_drain` | Cancel during late drain safe | PASS |
| 9 | `approval_during_concurrent_drain` | Approval concurrent with drain | PASS |
| 10 | `shutdown_during_late_drain` | Shutdown joins under race | PASS |
| 11 | `multi_session_independent_drains` | Multi-session isolation | PASS |
| 12 | `shutdown_all_joins_every_drain` | `shutdown_all` joins all drains | PASS |
| 13 | `later_session_not_poisoned` | No cross-session poison | PASS |
| 14 | `drain_phase_advances_past_prompt_return` | Metrics progress after return | PASS |

| Unit policy | Result |
|---|---|
| `phase_advances_monotonically` | PASS |
| `late_policy_duplicate_terminal` | PASS |
| `late_policy_non_terminal_no_regression` | PASS |
| `late_policy_process_exit_applies` | PASS |
| `pre_terminal_always_full` | PASS |

| Stress | Result |
|---|---|
| `stress_repeated_prompts_zero_false_persist_errors` | PASS |
| `stress_overlapping_sessions_independent_drains` | PASS |
| `stress_cancel_and_shutdown_races` | PASS |

**Note:** Worker matrix lists **14** named deterministic cases (+ 5 unit + 3 stress). Brief “15 invariants” maps to the 14 cases plus explicit phase/model unit coverage; all green.

## Regression — presentation / multi-session / VS / soak

| Suite | Count | Result |
|---|---|---|
| presentation_delivery | 19 | PASS |
| multi_session MS-01..17 | 17 | PASS |
| vs_scenarios | 23 | PASS |
| tracer-vs1-soak | 8 | PASS |
| stress multi-session + sequential | 3 | PASS |
| desktop_boundary_journey | 9 | PASS |
| live-grok-smoke unit/dry | 24 | PASS (no live) |

## Tauri E2E infrastructure (W2.2-A + integration)

| Surface | Command | Classification | Product impact |
|---|---|---|---|
| Doctor | `pnpm test:tauri-e2e:doctor` | **DRIVER_UNAVAILABLE** | Advisory; L2 still attemptable |
| L0 invoke policy | via `pnpm test:tauri-e2e` | PASS | Product path green |
| L1 desktop boundary | via `pnpm test:tauri-e2e` | PASS | Product path green |
| L2 app launch smoke | `node tools/tauri-e2e/l2-smoke.mjs --skip-build` | **PASS** | Process ownership + cleanup OK |
| L3-I driver infra | `node tools/tauri-e2e/l3i-infra.mjs` | **BLOCKED_BY_TOOLING** | Not product FAIL |
| L3-J product journey | — | **NOT_STARTED** | Future W2.2-B |

### Doctor host facts (this run)

| Item | Value |
|---|---|
| WebView2 | 150.0.4078.65 |
| tauri-driver | missing |
| msedgedriver | missing |
| frontend dist | present |
| app binary | `target/debug/tracer-desktop.exe` |

## Aggregate

| Layer | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace -- --test-threads=1` | PASS |
| `cargo clippy --workspace --all-targets` | PASS (warnings only) |
| `pnpm install --frozen-lockfile` | PASS |
| `pnpm -r test` / `pnpm -r build` | PASS |
| Gate decision | **PASS** |
