# Drain Lifecycle Validation Results (W2.2-C)

**Task:** `tracer-w2-drain-lifecycle`  
**Host:** grok-build  
**Date:** 2026-07-18  
**Evidence class:** fake ACP + temp SQLite (no live Grok)

## Commands executed

```powershell
cargo test -p tracer-control-plane --test drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --lib session::lifecycle -- --test-threads=1
cargo test -p tracer-vs1-stress --test stress_drain_lifecycle -- --test-threads=1
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
```

## Results

### Integration (`drain_lifecycle`) — 14/14 PASS

```
test approval_during_concurrent_drain ... ok
test cancel_during_late_drain ... ok
test drain_phase_advances_past_prompt_return ... ok
test duplicate_terminal_event_policy ... ok
test late_metadata_event_policy ... ok
test late_non_terminal_event_policy ... ok
test later_session_not_poisoned ... ok
test multi_session_independent_drains ... ok
test normal_channel_close_does_not_increment_persist_errors ... ok
test prompt_return_before_terminal_drain ... ok
test real_storage_error_increments_persist_errors ... ok
test shutdown_all_joins_every_drain ... ok
test shutdown_during_late_drain ... ok
test terminal_persisted_before_completion_presentation ... ok
```

Wall clock ≈ 12–15s (`--test-threads=1`).

### Lifecycle unit — 5/5 PASS

### Stress (`stress_drain_lifecycle`) — 3/3 PASS

| Scenario | Assertions |
|---|---|
| Repeated prompts | false PE=0, unique sequences, clean shutdown |
| Overlapping sessions + mild slow SQLite | false PE=0, no cross-session leak, unique seq |
| Cancel + shutdown races | later session not poisoned, live_count=0 |

Wall clock ≈ 6–7s.

### VS regression — 23/23 PASS (~30s)

## False-persist-error scoreboard

| Scenario | false `persist_errors` |
|---|---|
| Normal happy lifecycle | **0** |
| Multi-session independent drains | **0** |
| Channel close / stop | **0** |
| Stress overlap (staggered + 1ms delay) | **0** |
| Forced inject (`set_test_force_persist_error(true)`) | **>0 (true positive)** |

## Orphan / isolation scoreboard

| Metric | Result |
|---|---|
| Lost terminal (happy path evidence) | 0 lost |
| Duplicate sequences | 0 |
| Orphan live sessions after `shutdown_all` | 0 |
| Cross-session event leak | 0 |
| Cross-session poison after peer force-fail | 0 |

## Conclusion

**PASS** for W2.2-C drain lifecycle hardening under fake ACP. Residual multi-session write contention is absorbed by bounded retries; only exhausted real failures increment `persist_errors`.
