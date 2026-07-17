# W1-D Public Interface — for W1-F Control Plane

**Crate:** `tracer-runtime-adapter`  
**Supporting:** `tracer-acp-client` (lower-level framing / SM if needed)

## 1. Ownership

| Concern | Owner |
|---|---|
| Spawn / kill / orphan prevention | `tracer-process` via adapter composition |
| ACP framing + protocol SM | `tracer-acp-client` (used inside adapter) |
| Normalization → envelopes | `tracer-runtime-adapter` |
| Persist events / sole SQLite writer | **W1-F control plane** |
| Permission policy UI decisions | **W1-F** (adapter only forwards) |

## 2. Sync / async model

- API is **synchronous** (`&self` methods).
- Background reader/writer threads handle stdio.
- `submit_prompt` **blocks** until prompt RPC completes (end_turn / cancelled / error).
- Call `cancel_prompt` / `resolve_approval` from **another thread** while prompt blocks.
- Events: `mpsc::Receiver<AdapterEvent>` via `take_event_receiver()` or `try_recv_event` / `drain_events`.

## 3. Lifecycle API

```text
RuntimeAdapter::start(spec, project_id, session_id) -> Result<Self, AdapterError>
  // emits session.created, runtime.process.started (process alive ≠ ready)

initialize() -> Result<Capabilities, AdapterError>
  // emits runtime.process.ready

inspect_auth_requirement() / authenticate(method_id?) -> ...
  // auth distinct from process ready

create_session(SessionCreateParams { cwd, model_hints? }) -> Result<runtimeSessionId, AdapterError>
  // success: session.ready
  // AuthenticationRequired: NO session.ready

submit_prompt(PromptRequest { prompt_id?, text }) -> Result<(), AdapterError>
  // emits session.prompt.submitted + stream + terminal session.completed|cancelled|failed

resolve_approval(ApprovalDecisionRequest { approval_id, decision, option_id?, reason? })
  // decision: allow|deny|cancel — never auto

cancel_prompt() -> Result<(), AdapterError>
  // CapabilityUnsupported if caps.cancellation == false → process stop fallback

shutdown(ShutdownOptions) / force_kill()
  // graceful stdin close then force via W1-C; no orphans

inspect() -> RuntimeAdapterState
is_process_alive() / is_protocol_ready() / is_session_ready() / auth_state()
```

## 4. Spawn helpers

```rust
fake_acp_spawn_config(node, fake_js, scenario_id, cwd)  // CI
grok_stdio_spawn_config(grok_exe, cwd)                  // stock: agent --no-leader stdio
```

`RuntimeSpawnSpec` → `SpawnConfig` for process manager.

## 5. Events

`AdapterEvent::Event(EventEnvelope)` — full v1 envelope with adapter-assigned  
`eventId` / `sequence` / observation `timestamp` for the live stream.

**Note:** Contract says control plane is authoritative for storage sequencing.  
W1-F may re-envelope or adopt adapter sequences when writing SQLite. Adapter sequences are monotonic per start for stream consumers/tests.

`AdapterEvent::Error(AdapterError)` — rare side channel; most failures also emit domain events.

### Terminal guarantees

| Situation | Events / outcome |
|---|---|
| Clean shutdown | `runtime.process.exited` expected=true |
| Unexpected EOF mid-prompt | `session.failed` + disconnect; **no** silent `session.completed` |
| Crash exit | process dead; failed/exited path |
| Auth required on session/new | `AuthenticationRequired`; **no** `session.ready` |
| Duplicate response id | `adapter.protocol.error` ProtocolViolation; no double apply |
| Unknown vendor | `adapter.protocol.unknown`; continue |

## 6. Cancellation

1. If `capabilities.cancellation`: send `session/cancel`; drain within cancel budget.
2. Permission pending: also respond cancelled to reverse-request (**no deadlock**, budget `PERMISSION_CANCEL_DEADLOCK_BUDGET` = 5s).
3. If unsupported: return `CapabilityUnsupported`; W1-F must `force_kill` / process stop.

## 7. Backpressure

Event channel is unbounded `mpsc`. W1-F should drain promptly.  
No adapter-side drop of protocol frames for backpressure in v1.

## 8. Error mapping

`AdapterError { error_class, message, retryable, details }`  
→ `to_tracer_error()` for domain. Classes from `RUNTIME_ADAPTER_CONTRACT_V1` / `tracer_domain::ErrorClass`.

Distinct categories: protocol, process, authentication, permission, capability, operation.

## 9. Process cleanup

- `shutdown` / `force_kill` / `Drop` all stop the child tree via W1-C.
- `kill_on_drop: true` by default on spawn specs.

## 10. What W1-F must not do

- Parse ACP / Grok wire in React or command layer
- Auto-approve permissions
- Treat `is_process_alive` as prompt-ready
- Open SQLite from adapter crates
