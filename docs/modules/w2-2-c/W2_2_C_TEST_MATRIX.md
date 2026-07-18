# W2.2-C — Drain Lifecycle Test Matrix

**Task:** `tracer-w2-drain-lifecycle`  
**Primary suite:** `crates/tracer-control-plane/tests/drain_lifecycle.rs`  
**Stress:** `tests/stress/src/stress_drain_lifecycle.rs`  
**Regression:** `cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1`

## Named deterministic cases

| # | Test name | Invariant | Result |
|---|---|---|---|
| 1 | `prompt_return_before_terminal_drain` | Prompt return does not end ingestion early | PASS |
| 2 | `terminal_persisted_before_completion_presentation` | Terminal presentation only after terminal persistence | PASS |
| 3 | `normal_channel_close_does_not_increment_persist_errors` | Expected channel close ≠ persist_error | PASS |
| 4 | `real_storage_error_increments_persist_errors` | Real storage failure remains observable | PASS |
| 5 | `late_metadata_event_policy` | Late metadata: persist, no status reopen | PASS |
| 6 | `late_non_terminal_event_policy` | Late non-terminal: no status regression | PASS |
| 7 | `duplicate_terminal_event_policy` | Duplicate terminal: no status churn | PASS |
| 8 | `cancel_during_late_drain` | Cancel during late drain safe | PASS |
| 9 | `approval_during_concurrent_drain` | Approval concurrent with drain | PASS |
| 10 | `shutdown_during_late_drain` | Shutdown joins drain under race | PASS |
| 11 | `multi_session_independent_drains` | Multi-session isolation | PASS |
| 12 | `shutdown_all_joins_every_drain` | `shutdown_all` joins every drain | PASS |
| 13 | `later_session_not_poisoned` | No cross-run / cross-session poison | PASS |
| 14 | `drain_phase_advances_past_prompt_return` | Lifecycle metrics progress after return | PASS |

## Unit (lifecycle policy)

| Test | Result |
|---|---|
| `session::lifecycle::tests::phase_advances_monotonically` | PASS |
| `session::lifecycle::tests::late_policy_duplicate_terminal` | PASS |
| `session::lifecycle::tests::late_policy_non_terminal_no_regression` | PASS |
| `session::lifecycle::tests::late_policy_process_exit_applies` | PASS |
| `session::lifecycle::tests::pre_terminal_always_full` | PASS |

## Stress

| Test | Assert | Result |
|---|---|---|
| `stress_repeated_prompts_zero_false_persist_errors` | false PE=0, dups=0, shutdown clean | PASS |
| `stress_overlapping_sessions_independent_drains` | cross-session=0, false PE=0, unique seq | PASS |
| `stress_cancel_and_shutdown_races` | cancel/shutdown races; later session clean | PASS |

## VS suite regression

| Suite | Result |
|---|---|
| `vs_scenarios` (23 tests, `--test-threads=1`) | PASS |

## Evidence class

- Fake ACP (`tools/fake-acp-runtime`)
- Temp file / memory SQLite
- No network, no credentials, no live Grok

## Commands

```powershell
cargo test -p tracer-control-plane --test drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --lib session::lifecycle -- --test-threads=1
cargo test -p tracer-vs1-stress --test stress_drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
```
