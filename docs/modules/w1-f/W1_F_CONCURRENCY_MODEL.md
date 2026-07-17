# W1-F Concurrency Model

## 1. Goals

- Event ingestion **continues** while `submit_prompt` blocks.
- Cancel and approve run **concurrently** with a blocking prompt (adapter `&self` API).
- No deadlock on VS-05 (cancel-before-approval); time-bounded by adapter budget + soft budget.
- Never hold a control-plane lock across long-running adapter RPCs if that lock is needed for approve/cancel.

## 2. Drain strategy (unbounded adapter channel)

```text
RuntimeAdapter event mpsc (unbounded)
        | take_event_receiver (once)
        v
OS drain thread  --tokio::sync::mpsc (BRIDGE_CAPACITY=256)-->  async persist pump (Tokio)
        |                                      |
        | continuous drain                     v
        | (Sender::blocking_send)       SqliteStorage::append_event
        |                               (storage sequence authoritative)
        v
   presentation fan-out (optional, post-persist)
```

Mitigation path (Gate 1.3):

```text
adapter unbounded receiver -> continuously drained -> bounded internal handoff -> immediate persistence -> presentation
```

- Adapter channel is unbounded (W1-D). W1-F drains promptly into a **bounded** bridge.
- Bridge is `tokio::sync::mpsc::channel(256)` — not unbounded->unbounded secondary buffering.
- Full bridge applies backpressure via `blocking_send`; stop uses `try_send` to remain responsive.
- Async pump awaits `recv` (does not block Tokio workers on `std::mpsc::recv`).
- If storage fails: set `persist_failed`; **do not claim session.completed**.
- Presentation is after successful persist; disconnected fanout does not block the pump.

## 3. Prompt / cancel / approve

| Op | Threading |
|---|---|
| `submit_prompt` | OS worker thread; control plane awaits join |
| `cancel_prompt` | Separate OS/blocking path; concurrent with submit |
| `resolve_approval` | Separate path; concurrent with submit |
| Event persist | Async pump on caller's multi-thread Tokio runtime |

Prefer `#[tokio::test(flavor = "multi_thread")]` so the pump runs while workers block.

## 4. Locks

- `SessionRuntimeState` uses `std::sync::Mutex` for short critical sections only.
- Async methods **must drop** `MutexGuard` before any `.await` (Tauri `Send` futures).
- Adapter shared locks are inside W1-D; W1-F does not nest them under long awaits.

## 5. Cancellation budgets

- Cooperative cancel: adapter `DEFAULT_CANCEL_TIMEOUT` / capability path.
- Permission pending: `PERMISSION_CANCEL_DEADLOCK_BUDGET` (5s) inside adapter.
- W1-F soft budget: cancel path + 5s; escalate to `force_kill` when `CapabilityUnsupported` and config allows.
- VS-05: assert cancel returns within ~8s; clear pending approvals (no stale actionable approvals).

## 6. Evidence

See `crates/tracer-control-plane/tests/vs_scenarios.rs` VS-05, VS-04, VS-06/07.