# W2-C Architecture — Multi-Session Runtime Isolation

**Task:** `tracer-w2-multi-session` (work item W2-C)  
**Scope:** local multi-session isolation / recovery on the control plane  
**Crate touch:** `crates/tracer-control-plane` (`plane.rs` multi-session surface only)  
**Not owned:** `session_runtime.rs` presentation fan-out (W2-A), desktop UI, live Grok, cloud collab

## 1. Purpose

Prove that **many local Tracer sessions** can coexist, switch focus, ingest, cancel, approve, persist, recover, and shut down **without cross-session poisoning**.

W2-C does **not** invent a parallel multi-runtime architecture. It documents and hardens the existing control-plane model:

| Supported | Explicit limitation |
|---|---|
| Many live sessions per `ControlPlane` | One **focused** presentation snapshot (`active_session_id`) |
| Parallel prompts **across** sessions | **One** in-flight prompt **per** session (`Ready` required) |
| Shared file/in-memory SQLite | Isolation by `session_id` keys, not separate DBs |
| Per-session adapter process + drain/pump | No multi-tenant cloud collaboration |

## 2. Component shape

```text
ControlPlane
├── sessions: Mutex<HashMap<TracerSessionId, Arc<LiveSession>>>
│     each LiveSession owns:
│       - RuntimeAdapter (own OS process)
│       - SessionRuntimeState (status, approvals, sequences, persist_failed)
│       - dual-stage ingest (OS drain → bounded bridge → async persist pump)
│       - IngestMetrics (diagnostics)
├── storage: SqliteStorage   # sole writer; partitions by session_id
├── snapshot: PresentationSnapshot  # single focused session
└── presentation_tx: optional fan-out bus (envelopes carry sessionId)
```

No separate `session/` module was required: the registry + coordinators already live on `ControlPlane`. Per-session runtime machinery remains in `session_runtime.rs` (W1-F / W2-A).

## 3. Isolation invariants

### 3.1 Identity

- **Tracer `session_id`** is the sole registry and storage partition key.
- **Runtime wire session id** (`runtime_session_id`) is observation-only; it must not be used as the control-plane lookup key.
- Fake ACP may reuse a fixed wire id across processes; CP still isolates by Tracer id.

### 3.2 Sequences

- Storage-authoritative `sequence` is **per session** (`sessions.next_sequence` + `events(session_id, sequence)` unique).
- Numeric sequence values may overlap across sessions without collision.
- Metadata updates use `update_session_preserving_sequence` + `repair_session_next_sequence` so concurrent ingest never rewinds a session’s counter (VS1-H3 lesson, multi-session hardened).

### 3.3 Approvals / cancel

- Pending approvals live on `SessionRuntimeState` per `LiveSession`.
- `approval_resolve(session_id, approval_id, …)` fails with `ApprovalUnknown` when the id is not in **that** session’s map (cross-session resolve rejected).
- Cancel clears **only** the target session’s pending approvals and adapter state.

### 3.4 Persistence failures

- `persist_failed` is **session-local**.
- A failed or crashed peer must not mark siblings failed.
- Transient UNIQUE/`next_sequence` races during create must not sticky-poison a `Ready` session:
  - clear sticky flag after successful Ready create
  - clear sticky flag when accepting a new prompt on `Ready`
  - repair `next_sequence` before / after prompt when needed
  - still refuse “complete” if the prompt did not advance durable history

### 3.5 Presentation focus

- `presentation_focus(session_id)` switches the cached snapshot **without** stopping other live sessions.
- Live focus projects from runtime state; history-only (post-stop / post-restart) projects from storage.
- Stopping the focused session clears focus fields; peers remain live.

### 3.6 Lifecycle / shutdown

- `session_stop` removes one registry entry; other live sessions continue.
- `shutdown_all` stops sessions in **sorted id order**, force-clears the registry, and resets presentation focus. Idempotent.
- `live_session_count` / `live_session_ids` support leak assertions.

## 4. Recovery model

| Path | Behavior |
|---|---|
| App restart after clean stop | Rows + events reload from SQLite; `live_session_count == 0` |
| Crash with stale Running / AwaitingApproval | `reconcile_stale_live_sessions` → terminal / disconnected; no live process |
| Interrupted mid-prompt | History remains listable per id; no peer impact |
| Spawn failure | Error class returned; subsequent `session_create` still succeeds |

## 5. Controlled rejections (not multi-runtime invention)

| Condition | Class / behavior |
|---|---|
| Second concurrent prompt on same session | `InvalidState` |
| Cross-session approval resolve | `ApprovalUnknown` |
| Prompt while not `Ready` | `InvalidState` |
| Missing live session for cancel/approve | `SessionNotFound` |
| Parallel multi-tenant / remote collab | **Out of scope** — not implemented |

## 6. Forbidden (this task)

- Presentation redesign / desktop multi-session UI
- Live Grok or credentialed providers
- Cloud collaboration
- Rewriting `session_runtime` fan-out (W2-A)
- Inventing a second runtime supervisor path when HashMap multi-session already works

## 7. Public surface added/hardened (W2-C)

| API | Role |
|---|---|
| `ControlPlane::presentation_focus` | Switch focused snapshot without stop |
| `ControlPlane::live_session_ids` | Diagnostics / isolation tests |
| `ControlPlane::live_session_count` | Leak assertions |
| `ControlPlane::shutdown_all` | Deterministic multi-session teardown |
| `session_stop` focus clear | Avoid stale focused id after stop |
| Sequence repair helpers | Session-local next_sequence safety |

## 8. Test evidence locations

- Isolation: `crates/tracer-control-plane/tests/multi_session.rs` (MS-01…MS-16)
- Stress: `tests/stress/src/stress_multi_session.rs`
- Regression: `crates/tracer-control-plane/tests/vs_scenarios.rs` (VS-01…VS-14) must stay green
- Matrix: `docs/modules/w2-c/W2_C_TEST_MATRIX.md`
