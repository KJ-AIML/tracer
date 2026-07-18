# W2-A Test Matrix — Presentation Delivery Hardening

**Task:** `tracer-w2-presentation-delivery` (work item W2-A)  
**Crate:** `crates/tracer-control-plane`  
**Primary suite:** `cargo test -p tracer-control-plane --test presentation_delivery -- --test-threads=1`  
**Regression guard:** `cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1`  
**Hub unit tests:** `cargo test -p tracer-control-plane presentation --lib -- --test-threads=1`

## 1. Required invariants (Wave 2.1 / W2-A)

| # | Invariant | Test(s) | Pass criteria |
|---|---|---|---|
| 1 | Persistence independent of presentation | `inv01_persistence_independent_of_presentation` | File SQLite + fake ACP; events persist with **no** presentation consumers |
| 2 | Slow / absent consumers → no unbounded growth | `inv02_slow_consumer_no_unbounded_growth`, `inv02_absent_consumer_no_growth`, `inv_slow_legacy_sender_does_not_block_persist` | Pending notifies ≤ `DEFAULT_NOTIFY_CAPACITY` (1); absent path `notify_sends=0`; events still persist with undrained legacy sender |
| 3 | Latest state always via snapshot | `inv03_latest_state_via_snapshot` | After multiple publishes, `snapshot()` has latest sequence + status |
| 4 | Terminal cannot be permanently missed | `inv04_terminal_cannot_be_permanently_missed` | After flood + terminal, sticky terminal + snapshot status Failed; reconnect sees Failed |
| 5 | Notification duplication harmless | `inv05_notification_duplication_harmless` | Duplicate publish only bumps revision; consumer re-pulls same logical state |
| 6 | Notification loss recoverable | `inv06_notification_loss_recoverable_via_snapshot` | After drop/unsubscribe + more publishes, new consumer snapshot is correct |
| 7 | Snapshot delivery revisions monotonic | `inv07_snapshot_revisions_monotonic` | 50 publishes: each revision > previous; schema `version` stays `SNAPSHOT_VERSION` |
| 8 | Consumer detects stale snapshot | `inv08_consumer_detects_stale_snapshot` | `is_stale(known)` true after publish; false at current revision |
| 9 | Multi-consumer cannot block publish | `inv09_multiple_consumers_cannot_block_publish`, `inv_multi_consumer_subscribe_and_snapshot` | 8 undrained consumers + 500 publishes finish < 2s; CP multi-subscribe observes session |
| 10 | Disconnect removes delivery state | `inv10_disconnect_removes_delivery_state` | Drop subscription → `consumers_removed`++; further publishes do not send |
| 11 | Shutdown clears presentation delivery | `inv11_shutdown_clears_consumers`, `inv_shutdown_presentation_after_session` | `shutdown` freezes revision; sinks cleared; CP `shutdown_presentation` after session |
| 12 | VS / soak ordering unchanged | `inv12_vs_happy_path_ordering_smoke` + **vs_scenarios** full suite | Event sequences monotonic; VS-01..14 remain green |

## 2. Supporting scenarios (delivery stress)

| Scenario | Test | Pass criteria |
|---|---|---|
| Burst beyond delivery capacity | `inv_burst_beyond_delivery_capacity_coalesces` | Burst >> capacity; received ≤ 1; snapshot holds last sequence |
| Reconnect sees latest | `inv_reconnect_sees_latest` | Drop sub1, publish more, sub2 snapshot is latest |
| Coalesce under live fake path | `inv_coalesce_under_burst_with_fake_path` | Slow drain thread + real session; events + revision still advance |
| Hub unit: revision / coalesce / disconnect / terminal | `presentation::hub::unit_tests::*` | 4 unit tests green |

## 3. Harness notes

| Need | Approach |
|---|---|
| File SQLite | `tempfile` + `ControlPlaneConfig.database_path` |
| Fake ACP | `tools/fake-acp-runtime/bin/fake-acp-runtime.js` via `stock_opts()` / config `fake_js` |
| Slow consumer | Never drain `mpsc` / sleep between `recv_timeout` |
| Absent consumer | No `subscribe` / no `set_presentation_sender` |
| Legacy SOAK path | `set_presentation_sender` → capacity-1 bridge + forwarder |
| Multi-consumer | Multiple `subscribe` / `subscribe_notify` |
| Terminal | `SessionStatus::Failed` + `terminal_sticky` |

## 4. Explicit non-goals (not in this matrix)

| Not covered | Owner / reason |
|---|---|
| Desktop shell binding of `revision` / subscribe API | W2-B |
| Multi-session isolation suite | W2-C |
| Live Grok / approval UX | W2-D |
| Unbounded re-introduction of per-event fan-out | Forbidden by design |
| Domain / process / storage / adapter redesign | Forbidden for W2-A |

## 5. How to run

```powershell
cd repos/worktrees/tracer-w2-a

# W2-A delivery suite
cargo test -p tracer-control-plane --test presentation_delivery -- --test-threads=1

# Hub unit tests
cargo test -p tracer-control-plane presentation --lib -- --test-threads=1

# Required regression (keep green)
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
```

Requires `node` on PATH for fake ACP integration tests.

## 6. Evidence (this delivery)

| Check | Result |
|---|---|
| `presentation_delivery` | **19 passed** |
| hub `unit_tests` | **4 passed** |
| `vs_scenarios` | **23 passed** |
| Network / live Grok | **none** |
| Credentials | **none** |
