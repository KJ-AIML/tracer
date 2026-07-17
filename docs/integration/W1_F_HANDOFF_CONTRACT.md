# W1-F Handoff Contract — Runtime Adapter Public API (Gate 1.2)

**Status:** Gate 1.2 output — **implementation-backed**  
**Producer:** `tracer-w1-d-integration` (W1-D land on main)  
**Consumer:** W1-F control plane (`tracer-w1-control-plane`, **not started by this gate**)  
**Primary crate:** `tracer-runtime-adapter`  
**Supporting crate:** `tracer-acp-client` (framing / SM; prefer adapter for product ops)  
**Date:** 2026-07-17

## 1. Authorization

| Field | Value |
|---|---|
| Gate 1.2 | **PASS** |
| W1-F may claim | **YES** |
| Raw ACP / Grok required by React or W1-F | **NO** — consume normalized `EventEnvelope` only |
| SQLite ownership | **W1-F sole writer** — adapter must not open DB |

## 2. Ownership (reaffirmed)

| Concern | Owner |
|---|---|
| Spawn / kill / orphan prevention / Job Object | `tracer-process` via adapter composition |
| ACP NDJSON framing + protocol SM | `tracer-acp-client` (inside adapter) |
| Normalization → Tracer Event Protocol v1 | `tracer-runtime-adapter` |
| Persist events / sessions | **W1-F + `tracer-storage`** |
| Permission UI decisions | **W1-F** (adapter only forwards reverse-requests) |
| Tauri commands | **W1-F** |
| React presentation | W1-A shell (consumes commands/events; no raw ACP) |

## 3. Sync model

- Public API is **synchronous** (`&self` methods).
- Background reader/writer threads handle process stdio.
- `submit_prompt` **blocks** until prompt RPC completes (`end_turn` / cancelled / error).
- Call `cancel_prompt` / `resolve_approval` from **another thread** while prompt blocks.
- Events: `std::sync::mpsc` via `take_event_receiver` or poll helpers.
- Event channel is **unbounded** — W1-F must drain promptly (no adapter-side drop for backpressure in v1).

## 4. Types

```rust
// crate: tracer_runtime_adapter

pub enum AdapterEvent {
    Event(Box<tracer_domain::EventEnvelope>),
    Error(AdapterError),
}

pub struct PromptRequest {
    pub prompt_id: Option<String>,
    pub text: String,
}

pub struct ApprovalDecisionRequest {
    pub approval_id: String,
    /// "allow" | "deny" | "cancel" (also accept allow_once / allow_always synonyms)
    pub decision: String,
    pub option_id: Option<String>,
    pub reason: Option<String>,
}

pub struct SessionCreateParams {
    pub cwd: String,
    pub model_hints: Option<serde_json::Value>,
}

pub struct ShutdownOptions {
    pub graceful: bool,
    pub graceful_timeout: std::time::Duration,
    pub force_timeout: std::time::Duration,
}

pub struct RuntimeAdapterState {
    pub readiness: AdapterReadiness,
    pub capabilities: Option<tracer_domain::Capabilities>,
    pub runtime_kind: String,
    pub runtime_session_id: Option<String>,
    pub auth_methods: Vec<serde_json::Value>,
    pub shutdown_requested: bool,
}

pub struct AdapterError {
    // fields: error_class, message, retryable, details
    // maps via to_tracer_error() -> tracer_domain::TracerError
}

pub type AdapterHandle = RuntimeAdapter;

pub const DEFAULT_RPC_TIMEOUT: Duration;              // 20s
pub const DEFAULT_CANCEL_TIMEOUT: Duration;           // 10s
pub const PERMISSION_CANCEL_DEADLOCK_BUDGET: Duration; // 5s
```

Spawn helpers:

```rust
pub fn fake_acp_spawn_config(
    node: impl Into<PathBuf>,
    fake_js: impl Into<PathBuf>,
    scenario_id: impl Into<String>,
    cwd: impl Into<PathBuf>,
) -> RuntimeSpawnSpec;

pub fn grok_stdio_spawn_config(
    grok_exe: impl Into<PathBuf>,
    cwd: impl Into<PathBuf>,
) -> RuntimeSpawnSpec; // stock: grok agent --no-leader stdio

pub struct RuntimeSpawnSpec { /* kind, executable, args, cwd, env, ... */ }
// converts to tracer_process::SpawnConfig
```

## 5. Lifecycle operations (implementation-backed)

All ops below are **present on `RuntimeAdapter`** as of Gate 1.2 tip. None marked unavailable.

### 5.1 start

```rust
RuntimeAdapter::start(
    spec: RuntimeSpawnSpec,
    project_id: ProjectId,
    session_id: SessionId,
) -> Result<RuntimeAdapter, AdapterError>
```

- Spawns via `tracer-process`.
- Emits process-started style events; **does not** complete ACP initialize.
- After start: `is_process_alive()` may be true; `is_protocol_ready()` / `is_session_ready()` remain false until later steps.

### 5.2 initialize

```rust
fn initialize(&self) -> Result<Capabilities, AdapterError>
```

- ACP `initialize` RPC + capability negotiation.
- On success: `is_protocol_ready() == true`; emits `runtime.process.ready` (via normalizer path).
- **Still not** session-ready.

### 5.3 inspect auth

```rust
fn inspect_auth_requirement(&self) -> AuthenticationState
fn auth_state(&self) -> AuthenticationState
```

- Auth is distinct from process/protocol readiness.

### 5.4 authenticate

```rust
fn authenticate(&self, method_id: Option<&str>) -> Result<(), AdapterError>
```

- Optional; fake path is no-op success when not required.
- Failures map to authentication error classes (not generic protocol collapse).

### 5.5 create session

```rust
fn create_session(&self, params: SessionCreateParams) -> Result<String /* runtimeSessionId */, AdapterError>
```

- Success: `session.ready` event; `is_session_ready() == true`.
- Auth required: `Err(AuthenticationRequired)` path; **no** `session.ready`.

### 5.6 submit prompt

```rust
fn submit_prompt(&self, prompt: PromptRequest) -> Result<(), AdapterError>
```

- **Blocks** until terminal prompt outcome.
- Emits `session.prompt.submitted` + stream events + terminal `session.completed` | cancelled | failed as appropriate.
- Concurrent cancel/approval requires other threads.

### 5.7 resolve approval

```rust
fn resolve_approval(&self, decision: ApprovalDecisionRequest) -> Result<(), AdapterError>
```

- Decisions: allow / deny / cancel (never auto-approved by adapter).
- Emits `approval.resolved` after wire response.

### 5.8 cancel

```rust
fn cancel_prompt(&self) -> Result<(), AdapterError>
```

- If `capabilities.cancellation`: send `session/cancel` notification; drain within cancel budget.
- Pending permission: also respond cancelled to reverse-request (**no deadlock**, budget `PERMISSION_CANCEL_DEADLOCK_BUDGET`).
- If cancellation unsupported: `Err(CapabilityUnsupported)` — W1-F must `force_kill` / process stop.

### 5.9 subscribe events

```rust
fn take_event_receiver(&self) -> Option<Receiver<AdapterEvent>>  // once
fn try_recv_event(&self) -> Option<AdapterEvent>
fn drain_events(&self) -> Vec<AdapterEvent>
fn wait_event(&self, timeout: Duration, pred: impl FnMut(&AdapterEvent) -> bool)
    -> Result<AdapterEvent, AdapterError>
fn collect_event_types(&self, timeout: Duration) -> Vec<String>
```

- Preferred long-lived consumer: `take_event_receiver` once, then recv loop.
- Poll helpers for tests / simple control-plane pumps.

### 5.10 inspect state

```rust
fn inspect(&self) -> RuntimeAdapterState
fn is_process_alive(&self) -> bool
fn is_protocol_ready(&self) -> bool
fn is_session_ready(&self) -> bool
fn auth_state(&self) -> AuthenticationState
// AdapterReadiness::may_accept_prompt via inspect().readiness
```

### 5.11 shutdown

```rust
fn shutdown(&self, opts: ShutdownOptions) -> Result<(), AdapterError>
fn force_kill(&self) -> Result<(), AdapterError>
// Drop also stops child tree via W1-C (kill_on_drop path)
```

## 6. Readiness contract (must not collapse)

| Gate | Meaning | API |
|---|---|---|
| Process alive | OS child running | `is_process_alive()` |
| Protocol ready | initialize + caps done | `is_protocol_ready()` |
| Authenticated | auth state | `auth_state()` / `inspect_auth_requirement()` |
| Session ready | session/new success | `is_session_ready()` |
| Prompt complete | prompt RPC finished | not `phase.is_prompt_active()` / prompt result |

**W1-F must not** treat process-alive as prompt-ready.

## 7. Terminal guarantees

| Situation | Outcome |
|---|---|
| Clean shutdown | process exit expected; cleanup via W1-C |
| Unexpected EOF mid-prompt | failed/disconnect path; **no** silent `session.completed` |
| Crash nonzero exit | process dead; failed/exited path |
| Auth required on session/new | auth error; **no** `session.ready` |
| Duplicate response id | protocol violation / error event; no double apply |
| Unknown vendor notification | `adapter.protocol.unknown`; continue |
| Cancel while permission | time-bounded; no deadlock |

## 8. Error mapping

`AdapterError` carries `tracer_domain::ErrorClass` (+ message, retryable, details).  
Use `to_tracer_error()` for domain-shaped errors.

Distinct categories exercised in tests: protocol, process, authentication, permission, capability, operation.

## 9. What W1-F must not do

1. Parse ACP or Grok wire in React or Tauri command layer.
2. Auto-approve permissions.
3. Treat `is_process_alive` as protocol/session/prompt ready.
4. Open SQLite from adapter crates (already forbidden; do not add).
5. Require live Grok for standard CI acceptance.
6. Start work that rewrites W1-D ownership without contract change.

## 10. Suggested first W1-F acceptance slice

1. Wire Tauri commands to adapter ops above (fake spawn config for CI).
2. Fan-out `AdapterEvent::Event` envelopes to UI + storage append (sole writer).
3. VS-01 happy path against fake ACP end-to-end.
4. VS-02 auth gate; VS-06 crash honesty; permission cancel path.

## 11. Evidence pointers

| Artifact | Path |
|---|---|
| Public interface (module) | `docs/modules/w1-d/W1_D_PUBLIC_INTERFACE.md` |
| Architecture | `docs/modules/w1-d/W1_D_ARCHITECTURE.md` |
| Adapter implementation | `crates/tracer-runtime-adapter/src/adapter.rs` |
| Fake vertical slice tests | `crates/tracer-runtime-adapter/tests/fake_scenarios.rs` |
| Gate 1.2 report | `docs/integration/WAVE_1_2_INTEGRATION_REPORT.md` |
| Gate 1.2 test matrix | `docs/integration/WAVE_1_2_TEST_MATRIX.md` |
| Runtime adapter contract | `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md` |
