# Runtime Adapter Contract v1

**Status:** Gate 0 contract (Wave 0 freeze candidate)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-architecture-contracts`  
**Applies to:** control plane ↔ agent runtime boundary via adapter modules

## 1. Purpose

The runtime adapter isolates Tracer from any single agent runtime implementation (including stock Grok Build and a future downstream runtime).

Goals:

1. Speak **ACP-compatible** JSON-RPC over **stdio** for the first milestone.
2. Negotiate **capabilities** before accepting user prompts.
3. Normalize runtime traffic into **Tracer Event Protocol v1** events.
4. Preserve raw/vendor data only as optional adapter metadata.
5. Define lifecycle, cancellation, process-exit, and error-class behavior so Wave 1 modules can implement without re-litigating interfaces.

Non-goals for v1:

- Multi-tenant remote runtimes
- ALMS multi-agent orchestration
- Replacing the process manager (spawn/kill ownership stays in process module)
- Auto-approving permissions

Related:

- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md`
- `docs/decisions/ADR-001-runtime-sidecar.md`
- `docs/decisions/ADR-002-event-normalization.md`

## 2. Layering

```text
UI (React)
  ↕ Tauri commands / event channel
Control plane
  ↕ domain services
Runtime process manager  →  OS process (spawn, pipes, exit)
Runtime adapter          →  ACP JSON-RPC framing + session semantics
Event normalizer         →  Tracer Event Protocol v1
Storage                  →  SQLite (single writer: control plane)
```

### 2.1 Ownership split

| Concern | Owner module |
|---|---|
| Spawn, env, cwd, kill, orphan prevention | Process manager |
| JSON-RPC framing, initialize, sessions, prompts | ACP client / adapter |
| Map runtime messages → Tracer events | Normalizer (adapter package) |
| Permission policy decisions | Control plane / permissions |
| Persist events & session state | Storage |
| Present timeline / approvals | UI (consumes normalized events only) |

The adapter **must not** open the Tracer SQLite database for writes.

## 3. Runtime kinds and installations

### 3.1 Runtime kind

Logical adapter name, stable string:

```text
acp-stdio          # default first-slice kind (stock or fake ACP over stdio)
```

Future kinds (not required for Gate 1): `acp-tcp`, vendor-specific kinds only if justified by evidence.

### 3.2 Runtime installation descriptor (logical)

```json
{
  "runtimeKind": "acp-stdio",
  "displayName": "ACP stdio runtime",
  "executable": "path-or-command-name",
  "args": ["--acp"],
  "env": {},
  "workingDirectoryPolicy": "project_root",
  "version": "unknown-or-semver"
}
```

Rules:

- Executable resolution prefers configured override, then PATH lookup, then documented relative workspace tooling paths.
- Never hardcode machine-specific absolute paths in committed config or tests.
- Working directory for the first slice is the opened project root unless the contract is revised.

## 4. Adapter interface (logical API)

Language-agnostic interface. Rust traits and TypeScript types in Wave 1 must match this semantics, not necessarily these exact names.

### 4.1 Types

```text
RuntimeId            Tracer UUID for a managed process/installation binding
SessionHandle        Tracer session UUID + optional runtimeSessionId
CapabilitySet        boolean/feature map from negotiation
AdapterEvent         Stream item: Result<NormalizedEvent, AdapterError>
PromptRequest        { promptId, text, attachments[] }
CancelRequest        { promptId? | agentRunId? | session scope }
ApprovalDecision     { approvalId, decision: allow|deny|cancel, reason? }
```

### 4.2 Lifecycle methods

```text
connect(runtimeConfig) -> Result<RuntimeHandle, AdapterError>
  - Assumes process manager has spawned and provided stdin/stdout/stderr handles
    OR adapter is given a ProcessHandle abstraction.
  - Performs protocol initialize / handshake.
  - Performs capability negotiation.
  - Emits runtime.process.ready (via normalizer) on success.

createSession(params: { projectId, sessionId, cwd, modelHints? })
  -> Result<SessionHandle, AdapterError>

submitPrompt(session, PromptRequest) -> Result<(), AdapterError>

cancel(session, CancelRequest) -> Result<(), AdapterError>
  - If capabilities.cancellation == false: return error class CapabilityUnsupported
    and let control plane fall back to process stop.

resolveApproval(session, ApprovalDecision) -> Result<(), AdapterError>

shutdown(session?, graceful: bool) -> Result<(), AdapterError>
  - Prefer graceful ACP shutdown when supported; always ensure process manager
    can force-kill after timeout.

events() -> Stream<AdapterEvent>
  - Continuous stream of normalized events + adapter errors for the runtime binding.
```

### 4.3 Readiness

A runtime is **ready** only when:

1. Process is alive.
2. Initialize/handshake succeeded.
3. Capability negotiation completed and was recorded.
4. `runtime.process.ready` has been emitted.

Prompts submitted before ready MUST fail with `RuntimeNotReady`.

## 5. Capability negotiation

### 5.1 Capability set (Tracer view)

| Capability key | Meaning for Tracer |
|---|---|
| `promptStreaming` | Message/progress deltas will stream |
| `cancellation` | Runtime supports cancel without killing process |
| `planUpdates` | Plan snapshots/patches available |
| `toolCalls` | Tool start/update/complete available |
| `approvals` | Permission requests will be emitted |
| `fileChangeNotifications` | File change events available |
| `terminalOutput` | Terminal stream events available |
| `sessionResume` | Runtime can resume prior runtime session ids |

Values are booleans unless later extended with structured limits (for example max payload size). Unknown capability keys from the runtime are stored under adapter metadata and ignored for product branching unless promoted into this table by contract revision.

### 5.2 Negotiation algorithm

1. Send ACP initialize (or equivalent) with Tracer client info and protocol version offer.
2. Receive server capabilities / protocol version.
3. Compute **intersection** of required and optional sets:

**Required for first vertical slice (minimum viable):**

```text
promptStreaming = true   # soft-required: if false, adapter may synthesize
                         # a single agent.message.completed from final response
toolCalls       = optional but expected for rich UI
cancellation    = optional (fallback: kill process)
approvals       = optional (if absent, no approval UI path)
```

4. Persist negotiated set on the runtime/session record.
5. Emit `runtime.process.ready` with `capabilities`.
6. If the runtime cannot satisfy a **hard** requirement defined by product policy, emit `runtime.process.failed` with `errorClass: CapabilityMismatch` and do not mark session ready.

### 5.3 Missing capability behavior

| Missing capability | Behavior |
|---|---|
| `cancellation` | `cancel()` returns `CapabilityUnsupported`; control plane stops process |
| `approvals` | Tools that would need approval either never appear or fail closed if policy requires approval for that action class |
| `planUpdates` | UI hides plan panel; no synthetic plans |
| `fileChangeNotifications` | Changes feature empty/disabled unless Tracer observes filesystem separately (not required for Gate 1) |
| `promptStreaming` | Buffer final message only; still emit `agent.message.completed` |

## 6. ACP transport assumptions (first milestone)

- Transport: JSON-RPC messages over stdio (newline-delimited or Content-Length framing—implementation must pick one and test against fake runtime + stock runtime evidence from W0-B).
- One JSON-RPC client per process.
- Notifications and responses must be correlated by protocol rules; duplicate response IDs → `adapter.protocol.error`.
- Stderr is not ACP; process manager surfaces stderr as `runtime.process.stderr`.

Exact wire field names for stock Grok Build are **evidence-owned by W0-B**. This contract freezes Tracer-side adapter semantics; W0-B fills the mapping tables under `docs/research/grok-build/`.

## 7. Normalization duties

The adapter/normalizer MUST:

1. Map supported runtime updates to Tracer event types in `TRACER_EVENT_PROTOCOL_V1.md`.
2. Assign no Tracer `sequence` itself if the control plane owns sequencing—**or** provide raw normalized payloads for the control plane to envelope. **Decision for Wave 1:** control plane assigns `eventId`, `sequence`, `timestamp` (observation time), and session identity fields; adapter supplies `type`, `payload`, and optional `adapter` metadata.
3. Preserve unsupported vendor notifications as `adapter.protocol.unknown` with safe metadata.
4. Never require React to interpret ACP methods.

### 7.1 Identity bridging

```text
Tracer sessionId     ↔  runtimeSessionId (adapter metadata)
Tracer promptId      ↔  runtime prompt/request id
Tracer toolCallId    ↔  runtime tool call id
Tracer approvalId    ↔  runtime permission request id
```

Bridges are stored so resume and debug work without leaking runtime ids into UI primary keys.

## 8. Error classes

Stable `errorClass` strings used in adapter errors, Tauri errors, and event payloads:

| errorClass | Meaning | Typical retryable |
|---|---|---|
| `RuntimeExecutableNotFound` | Configured binary missing | no (until config fixed) |
| `RuntimeSpawnFailed` | OS spawn failure | maybe |
| `RuntimeNotReady` | Prompt/session op before ready | yes after ready |
| `RuntimeDisconnected` | Pipes closed / unexpected EOF | no for current process |
| `RuntimeCrashed` | Non-zero or signal exit unexpectedly | no for current process |
| `ProtocolInitializeFailed` | Handshake failed | no |
| `CapabilityMismatch` | Negotiated caps insufficient | no |
| `CapabilityUnsupported` | Op requires missing cap | no (use fallback) |
| `ProtocolParseError` | Malformed JSON / framing | maybe (continue if process up) |
| `ProtocolViolation` | Duplicate ids, invalid state | maybe |
| `SessionNotFound` | Unknown session handle | no |
| `PromptRejected` | Runtime refused prompt | depends |
| `CancellationFailed` | Cancel not honored in time | no → force kill |
| `ApprovalUnknown` | Unknown approval id | no |
| `PermissionDenied` | Policy denied action | no |
| `Timeout` | Operation exceeded deadline | maybe |
| `InternalAdapterError` | Bug / unexpected | no |
| `StorageError` | Persistence failure (surfaced at adapter boundary only if relevant) | depends |
| `InvalidArgument` | Caller contract breach | no |

Errors returned to the control plane SHOULD be structured:

```json
{
  "errorClass": "RuntimeNotReady",
  "message": "Cannot submit prompt before runtime.process.ready",
  "retryable": true,
  "details": {}
}
```

Do not put secrets in `message` or `details`.

## 9. Cancellation semantics

### 9.1 Cooperative cancel (preferred)

Preconditions: `capabilities.cancellation == true`, process alive, session active.

Steps:

1. Control plane marks session `cancelling` (event).
2. Adapter sends runtime cancel for the active prompt/run.
3. Adapter drains terminal runtime events until cancel acknowledged or timeout `T_cancel` (implementation default suggested: 5–15s, configurable).
4. Emit `session.cancelled`.
5. Session may remain `ready` for a new prompt if the runtime session is reusable; otherwise reinitialize per W0-B evidence.

### 9.2 Non-cooperative / fallback

If cancellation unsupported, cancel times out, or protocol is wedged:

1. Process manager graceful terminate.
2. After `T_term`, force kill.
3. Emit `runtime.process.exited` with `expected: true` if user requested stop, else `expected: false` when crash-like.
4. Emit `session.cancelled` or `session.failed` / `disconnected` appropriately.
5. Guarantee no orphaned child processes (process manager acceptance criterion).

## 10. Process-exit semantics

Adapter observes process exit via process manager signals:

| Exit | Adapter action |
|---|---|
| Expected after shutdown | Emit `runtime.process.exited` (`expected: true`); close event stream cleanly |
| Unexpected while idle | Emit exited/failed; session → `disconnected` |
| Unexpected while running | Emit exited/failed; fail active tools; session → `failed` or `disconnected`; no silent complete |
| Exit during initialize | `ProtocolInitializeFailed` or `RuntimeCrashed`; session not ready |

After exit, all subsequent adapter calls on that handle fail with `RuntimeDisconnected` until a new process is started.

## 11. Unknown and malformed traffic

| Input | Behavior |
|---|---|
| Valid JSON-RPC, unknown method/notification | Map to `adapter.protocol.unknown`; continue |
| Valid JSON, invalid against expected schema | `adapter.protocol.error`; continue if possible |
| Invalid JSON / broken framing | `ProtocolParseError`; attempt resync only if framing allows; else treat as fatal transport error |
| Duplicate response id | `ProtocolViolation` event; ignore duplicate application side effects |
| Oversized message | Reject/truncate per limits; emit protocol error; never crash UI thread |

## 12. Security and permissions

1. Fail closed on unknown permission kinds.
2. Adapter never auto-approves; it only forwards `approval.requested` and applies `resolveApproval` decisions from the control plane.
3. Destructive tool classes require explicit user or policy allow.
4. Working directory confinement: first slice assumes project root; adapter should pass cwd as agreed and not invent broader FS roots.

## 13. Test seams

The adapter must be testable against:

1. **Fake ACP runtime** (Wave 1) implementing scripted scenarios.
2. **Recorded sanitized fixtures** from W0-B.
3. Optional stock runtime smoke tests outside default CI.

Required adapter-level scenarios (acceptance sketches):

- initialize + negotiate + ready
- prompt streaming happy path
- tool + approval allow/deny
- unknown notification
- malformed JSON
- runtime EOF mid-prompt
- cancel supported
- cancel unsupported → process stop
- capability missing matrix

## 14. Out of scope (explicit)

- Forking Grok Build
- Tracer-specific ACP extensions (may be proposed later with ADR)
- Writing UI components
- SQLite schema details (storage owns them; must store envelope fields)

## 15. Wave 1 implementation mapping (informative)

Suggested crates/packages (from master plan; not a write grant):

```text
crates/tracer-process/
crates/tracer-acp-client/
crates/tracer-runtime-adapter/
packages/runtime-client/
```

These names may adjust, but the **boundaries in this contract must not**.

---

**Document control:** Wave 0 deliverable. Wire-level Grok mapping refined by W0-B without breaking Tracer semantics here.
