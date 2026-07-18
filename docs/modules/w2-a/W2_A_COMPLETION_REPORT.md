# W2-A Completion Report — Presentation Delivery Hardening

**Task:** `tracer-w2-presentation-delivery`  
**Work item:** W2-A  
**Branch:** `agent/tracer-w2-presentation-delivery`  
**Worktree:** `repos/worktrees/tracer-w2-a`  
**Base:** `56715cc` (VS1 hardened tip)  
**Date:** 2026-07-18  
**Host:** grok-build  
**Session:** `heli-ses-d76e4760-0787-409a-9437-c82a88816727`

## Decision

| Item | Result |
|---|---|
| Goal achieved | **Yes** — unbounded per-event presentation fan-out replaced by snapshot + coalesced/bounded notify |
| Preferred path | persist → projection → revision++ → bounded notify → consumer pulls snapshot |
| 12 Wave 2.1 invariants | **12/12 proven** in `presentation_delivery` (+ hub unit tests) |
| VS regression | **vs_scenarios 23/23 green** (`--test-threads=1`) |
| Desktop / multi-session / live-grok | **Not done** (W2-B / W2-C / W2-D) |
| Push / merge | **Not done** (out of scope) |

## Deliverables

### Code

| Path | Change |
|---|---|
| `crates/tracer-control-plane/src/presentation/mod.rs` | Module surface + integrator notes |
| `crates/tracer-control-plane/src/presentation/hub.rs` | `PresentationHub`, subscription, metrics, legacy bridge/forwarder, shutdown |
| `crates/tracer-control-plane/src/session_runtime.rs` | Post-persist `publish_presentation` via hub (no consumer back-pressure) |
| `crates/tracer-control-plane/src/plane.rs` | Own hub; `subscribe_presentation` / `set_presentation_sender` / `shutdown_presentation` / `refresh_snapshot_for` |
| `crates/tracer-control-plane/src/types.rs` | `PresentationSnapshot.revision`, `PresentationNotify` |
| `crates/tracer-control-plane/src/lib.rs` | Export presentation types + capacity constant |
| `crates/tracer-control-plane/tests/presentation_delivery.rs` | 19 invariant / stress tests (file SQLite + fake ACP where needed) |

### Docs

| Path | Role |
|---|---|
| `docs/modules/w2-a/W2_A_ARCHITECTURE.md` | Normative path, components, invariants, capacity, failure modes |
| `docs/modules/w2-a/W2_A_TEST_MATRIX.md` | Invariant → test map + run commands |
| `docs/modules/w2-a/W2_A_COMPLETION_REPORT.md` | This report |

## Design summary

```text
adapter events
  → bounded BRIDGE_CAPACITY (256) drain/pump (unchanged)
  → SqliteStorage::append_event (sole writer)
  → PresentationHub::publish_session_update / publish_snapshot
       • revision saturating_add (monotonic delivery generation)
       • schema version forced to SNAPSHOT_VERSION
       • capacity-1 try_send notify (coalesce on Full)
       • watch::Sender<u64> multi-consumer wake
       • legacy set_presentation_sender: capacity-1 bridge + non-blocking forwarder thread
  → consumer: subscribe + snapshot() / events_list
```

**Schema vs delivery:** `version` = wire schema; `revision` = monotonic publish generation for staleness.

## Tests run

```text
cargo test -p tracer-control-plane --test presentation_delivery -- --test-threads=1
  ✓ 19 passed

cargo test -p tracer-control-plane presentation --lib -- --test-threads=1
  ✓ 4 passed

cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
  ✓ 23 passed
```

## Invariant coverage

| # | Invariant | Evidence |
|---|---|---|
| 1 | Persistence independent of presentation | `inv01_*` |
| 2 | Slow/absent → no unbounded growth | `inv02_*`, `inv_slow_legacy_*`, burst test |
| 3 | Latest via snapshot | `inv03_*` |
| 4 | Terminal not permanently missed | `inv04_*` + sticky terminal |
| 5 | Notify duplication harmless | `inv05_*` |
| 6 | Notify loss recoverable | `inv06_*` |
| 7 | Revisions monotonic | `inv07_*` |
| 8 | Stale detection | `inv08_*` |
| 9 | Multi-consumer non-blocking | `inv09_*`, `inv_multi_consumer_*` |
| 10 | Disconnect cleanup | `inv10_*` |
| 11 | Shutdown cleanup | `inv11_*`, `inv_shutdown_*` |
| 12 | VS ordering preserved | `inv12_*` + full `vs_scenarios` |

## Assumptions

1. Consumers treat notifies as **wake-ups** and re-pull `snapshot()` / `events_list` for authority.
2. Legacy `set_presentation_sender` API shape is preserved for SOAK/UI; semantics are coalesced (not every envelope).
3. Active-session projection is control-plane wide; multi-session isolation depth is W2-C.
4. Desktop does not yet bind `revision` / `subscribe_presentation` (W2-B).

## Risks

| Risk | Mitigation |
|---|---|
| Capacity-1 Full drops intermediate notify (not replace-with-latest) | Snapshot always latest; consumer re-pulls on any wake |
| Legacy forwarder thread can block on slow unbounded user sender | Bridge capacity 1; persist path never blocks on user sender |
| Sticky terminal cleared on non-terminal same-session publish | Domain terminal statuses do not regress; command path authoritative |
| Multi-session hub single active projection | Documented; W2-C owns isolation suite |
| SOAK tests expecting per-event envelopes on presentation channel | Coalesced batches; storage `events_list` remains authoritative |

## Integration requirements

1. **W2-B (desktop):** prefer `subscribe_presentation` + `snapshot()` (or poll `tracer_presentation_snapshot`); do not reintroduce unbounded per-event UI queues.
2. **SOAK / H3:** treat presentation metrics (`notify_coalesced`, `presentation_sends`) as delivery health, not event completeness.
3. **Integrator:** keep `vs_scenarios` green after merge; optional soak under burst + slow UI.
4. Do **not** merge until path claims / neighboring Wave 2 slices are coordinated by integrator.

## Owned path compliance

| Path | Action |
|---|---|
| `crates/tracer-control-plane/src/presentation/` | **Added** |
| `crates/tracer-control-plane/src/session_runtime.rs` | Presentation publish hooks only |
| `crates/tracer-control-plane/src/plane.rs` | Presentation hooks only |
| `crates/tracer-control-plane/src/types.rs`, `lib.rs` | Minimal revision/notify exports |
| `crates/tracer-control-plane/tests/presentation_delivery.rs` | **Added** |
| `docs/modules/w2-a/` | **Added** |
| Domain / process / storage / adapter / desktop / multi-session / live-grok | **Not modified** |

## Commit SHAs

Head: `7a2ff78` on `agent/tracer-w2-presentation-delivery` (not pushed).

| Commit | Summary |
|---|---|
| `c16e63d` | feat(w2-a): bounded presentation hub with coalescing notify |
| `5ab42e0` | test(w2-a): prove presentation delivery invariants |
| `9a75cad` | docs(w2-a): architecture, test matrix, completion report |
| `ada0e32` | docs(w2-a): record commit SHAs in completion report |
| `7a2ff78` | docs(w2-a): pin final head SHA in completion report |
## Lease

Write lease acquired via takeover; released after delivery commits (see session activity).




