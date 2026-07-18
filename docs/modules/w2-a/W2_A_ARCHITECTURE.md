# W2-A Architecture — Presentation Delivery Hardening

**Task:** `tracer-w2-presentation-delivery` (work item W2-A)  
**Crate:** `crates/tracer-control-plane`  
**Module:** `src/presentation/`  
**Base:** VS1 hardened tip (`56715cc`)

## 1. Purpose

Replace **unbounded presentation fan-out** (per-event `std::sync::mpsc::Sender` after every successful persist) with a **bounded, coalescing, snapshot-first** delivery path.

Slow or absent UI consumers must not:

- block persistence,
- force retention of every intermediate notification,
- permanently miss terminal session state.

## 2. Preferred path (normative)

```text
persist normalized event (SQLite, storage-authoritative sequence)
        │
        v
update canonical presentation projection (PresentationHub)
        │
        v
increment snapshot revision (monotonic u64)
        │
        v
bounded / coalescing notification signal (capacity 1 + watch)
        │
        v
consumer requests latest snapshot (and/or events_list)
```

Notifications are **wake-ups**, not a second event log. Persisted history remains in SQLite via `tracer-storage`.

## 3. Components

| Component | Role |
|---|---|
| `PresentationHub` | Canonical in-memory projection; publish; subscribe; shutdown |
| `PresentationSnapshot.revision` | Monotonic delivery generation (distinct from schema `version`) |
| `PresentationNotify` | Coalesced change signal (`revision`, status, sequence, `terminal`) |
| `PresentationSubscription` | Consumer handle; drop removes delivery state |
| Legacy `set_presentation_sender` | Capacity-1 bridge + forwarder thread; SOAK-compatible API |

```text
ControlPlane
├── PersistenceCoordinator (unchanged sole SQLite writer)
├── SessionCoordinator / LiveSession ingest pump
└── PresentationHub  (W2-A)
    ├── HubState { snapshot, revision, terminal_sticky }
    ├── watch::Sender<u64>          # multi-consumer revision watch
    ├── notify sinks (SyncSender cap=1)
    └── legacy event sinks (SyncSender cap=1 → optional forwarder)
```

## 4. Invariants

| # | Invariant | Mechanism |
|---|---|---|
| 1 | Persistence independent of presentation | Publish is post-persist; hub failure/absent is ignored |
| 2 | Slow/absent consumers → no unbounded growth | Capacity-1 `try_send`; full → coalesce/drop |
| 3 | Latest state via snapshot | `PresentationHub::snapshot` always holds latest projection |
| 4 | Terminal cannot be permanently missed | Sticky terminal + snapshot status + reconnect pull |
| 5 | Notification duplication harmless | Consumers re-pull snapshot; duplicates only re-wake |
| 6 | Notification loss recoverable | Snapshot refresh / `events_list` |
| 7 | Snapshot versions monotonic | `revision` saturating_add on each publish |
| 8 | Stale detection | Compare known `revision` to hub / `is_stale` |
| 9 | Multi-consumer cannot block persist | `try_send` only; never blocks publish path |
| 10 | Disconnect removes delivery state | `PresentationSubscription::Drop` / `unsubscribe` |
| 11 | Shutdown clears presentation delivery | `shutdown` / `shutdown_presentation` drops sinks + joins forwarders |
| 12 | VS / soak ordering unchanged | Persist path and sequences untouched; presentation is side-channel |

## 5. Schema vs delivery version

| Field | Meaning |
|---|---|
| `PresentationSnapshot.version` | **Schema** version (`SNAPSHOT_VERSION = 1`) — wire shape |
| `PresentationSnapshot.revision` | **Delivery** generation — monotonic per hub publish |

Desktop / TS may ignore `revision` until W2-B binds it; serde default `0` keeps older consumers safe.

## 6. Wire-up (plane hooks — integrator)

Minimal `plane.rs` changes (owned for W2-A hooks only):

1. `ControlPlane` owns `PresentationHub` (replaces `presentation_tx` + snapshot mutex).
2. `session_create` → `live.start_ingestor(storage, Some(hub.clone()))`.
3. `refresh_snapshot_for` → builds snapshot fields → `hub.publish_snapshot`.
4. Public APIs:
   - `snapshot()` → hub
   - `subscribe_presentation()` → subscription
   - `set_presentation_sender(tx)` → `attach_legacy_sender` (coalesced)
   - `shutdown_presentation()` → hub shutdown
   - `presentation_hub()` → tests / metrics

Post-persist projection update lives in `session_runtime::publish_presentation` (not in plane).

## 7. Explicit non-goals (forbidden / deferred)

- Desktop shell binding depth (W2-B)
- Multi-session isolation redesign (W2-C)
- Live Grok / approval UX (W2-D)
- Replacing SQLite event history with ephemeral notifications
- Second unbounded buffer on the ingest path
- Domain / process / storage / adapter redesign

## 8. Capacity constants

| Constant | Value | Notes |
|---|---|---|
| `BRIDGE_CAPACITY` | 256 | Persist path (unchanged, W1-F) |
| `DEFAULT_NOTIFY_CAPACITY` | 1 | Presentation notify / legacy bridge |

## 9. Failure modes

| Case | Behavior |
|---|---|
| No consumers | Publish updates projection only; metrics `notify_sends = 0` |
| Slow consumer | Intermediate notifies dropped; snapshot remains correct |
| Consumer disconnect mid-stream | Sink removed; no leak of delivery slots |
| Hub shutdown | Further publish no-ops revision; sinks cleared |
| Non-active session update | `publish_session_update` skips revision bump for other sessions |
