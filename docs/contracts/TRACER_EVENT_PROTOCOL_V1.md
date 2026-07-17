# Tracer Event Protocol v1

**Status:** Gate 0 contract (Wave 0 freeze candidate)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-architecture-contracts`  
**Applies to:** normalized events between the Tracer control plane and the desktop UI; storage of session history; contract tests

## 1. Purpose

This document freezes the **normalized Tracer event envelope** and the **initial event type catalog** for the first vertical slice.

Rules:

1. The UI consumes only normalized Tracer events, never raw ACP or vendor-specific runtime frames.
2. The control plane is the sole writer of normalized events into the primary SQLite database.
3. Runtime-native payloads may be preserved as optional adapter metadata for debugging.
4. Consumers **must tolerate unknown event types** and unknown payload fields without crashing.
5. Paths in examples are illustrative and relative; committed fixtures must not embed machine-specific absolute paths.

Related contracts:

- `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md` — how runtime I/O becomes normalized events
- `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` — request/response surface that may emit or stream events
- `docs/decisions/ADR-002-event-normalization.md` — why normalization is required

## 2. Envelope schema (v1)

Every normalized event uses one envelope:

| Field | Type | Required | Description |
|---|---|---|---|
| `eventVersion` | integer | yes | Protocol major version. Currently always `1`. |
| `eventId` | string (UUID) | yes | Tracer-owned unique identifier for this event row/instance. |
| `sequence` | integer ≥ 1 | yes | Monotonic sequence **within a Tracer `sessionId`**. |
| `timestamp` | string (RFC 3339) | yes | Control-plane observation time in UTC (`...Z` preferred). |
| `projectId` | string (UUID) | yes | Tracer project identifier. |
| `sessionId` | string (UUID) | yes | Tracer session identifier. |
| `agentRunId` | string (UUID) \| null | yes | Active agent run, or `null` if not applicable. |
| `type` | string | yes | Dotted event type name from the catalog (or unknown). |
| `payload` | object | yes | Type-specific body. Empty object allowed. |
| `adapter` | object \| null | no | Optional adapter metadata (raw runtime correlation). Default `null`/omitted. |
| `severity` | `"info"` \| `"warn"` \| `"error"` | no | Presentation hint. Default `"info"`. |

### 2.1 Sequence rules

- `sequence` starts at `1` for the first event of a Tracer session and increases by exactly `1` for each subsequent persisted event in that session.
- Sequence is assigned by the **control plane**, not the runtime.
- Stream consumers may buffer out-of-order delivery on the UI channel, but storage must retain control-plane order.
- Replays after restart use the same `sequence` values from storage.

### 2.2 Identifier rules

- Tracer-owned IDs (`eventId`, `projectId`, `sessionId`, `agentRunId`) use UUID string form (canonical lowercase hex with hyphens preferred).
- Runtime-native IDs live only under `adapter` or inside payload fields explicitly marked as runtime-native (for example `runtimeSessionId`).
- Never promote a Grok-specific or ACP-specific id into a Tracer primary key.

### 2.3 Unknown type and unknown field rules

**Unknown event type**

- If the normalizer cannot map a runtime notification to a known `type`, it MUST still emit a normalized envelope when the frame is structurally usable.
- Preferred type for unmapped but valid runtime traffic: `adapter.protocol.unknown` with a payload that preserves a safe summary and optional raw fragment under `adapter`.
- UI and storage MUST NOT drop envelopes solely because `type` is unrecognized.
- UI SHOULD render unknown types as a generic timeline entry (label = `type`, expandable JSON-safe payload view).

**Unknown payload fields**

- Deserializers MUST ignore unknown fields inside `payload` and at the envelope root (forward compatibility).
- Serializers for v1 producers MUST NOT omit required envelope fields.

**Malformed frames**

- If a runtime frame cannot be parsed at all, emit `adapter.protocol.error` (or `runtime.process.failed` if the transport dies) and continue the session if the process is still alive.
- Do not auto-complete or auto-fail the session solely because one frame was bad, unless the transport is dead.

### 2.4 Envelope example

```json
{
  "eventVersion": 1,
  "eventId": "550e8400-e29b-41d4-a716-446655440000",
  "sequence": 12,
  "timestamp": "2026-07-17T12:00:00.123Z",
  "projectId": "11111111-1111-1111-1111-111111111111",
  "sessionId": "22222222-2222-2222-2222-222222222222",
  "agentRunId": "33333333-3333-3333-3333-333333333333",
  "type": "agent.message.delta",
  "severity": "info",
  "payload": {
    "messageId": "44444444-4444-4444-4444-444444444444",
    "role": "assistant",
    "delta": "Inspecting the repository layout…",
    "contentType": "text/plain"
  },
  "adapter": {
    "runtimeKind": "acp-stdio",
    "runtimeSessionId": "rt-sess-abc",
    "rawRef": "optional-opaque-or-truncated-ref"
  }
}
```

## 3. Event type catalog (initial)

Types are stable strings. New types require a contract revision (minor if additive and ignored by old clients; major if envelope changes).

### 3.1 Runtime process lifecycle

| Type | When emitted | Payload summary |
|---|---|---|
| `runtime.process.started` | Child process spawned | `pid` (if known), `executable`, `args` (sanitized), `cwd` (relative or project-rooted) |
| `runtime.process.ready` | Adapter finished initialize + capability negotiation | `capabilities` object |
| `runtime.process.stderr` | Non-empty stderr chunk | `chunk` (string, may be truncated), `truncated` boolean |
| `runtime.process.exited` | Process exit observed | `exitCode` (int\|null), `signal` (string\|null), `expected` boolean |
| `runtime.process.failed` | Spawn/start failure or unrecoverable process error | `errorClass`, `message`, `retryable` boolean |

#### Example: process ready

```json
{
  "eventVersion": 1,
  "eventId": "a1000000-0000-4000-8000-000000000001",
  "sequence": 2,
  "timestamp": "2026-07-17T12:00:01.000Z",
  "projectId": "11111111-1111-1111-1111-111111111111",
  "sessionId": "22222222-2222-2222-2222-222222222222",
  "agentRunId": null,
  "type": "runtime.process.ready",
  "payload": {
    "capabilities": {
      "promptStreaming": true,
      "cancellation": true,
      "planUpdates": true,
      "toolCalls": true,
      "approvals": true,
      "fileChangeNotifications": false,
      "terminalOutput": false
    },
    "protocolVersion": "acp-negotiated"
  }
}
```

#### Example: process exited unexpectedly

```json
{
  "eventVersion": 1,
  "eventId": "a1000000-0000-4000-8000-000000000099",
  "sequence": 88,
  "timestamp": "2026-07-17T12:05:00.000Z",
  "projectId": "11111111-1111-1111-1111-111111111111",
  "sessionId": "22222222-2222-2222-2222-222222222222",
  "agentRunId": "33333333-3333-3333-3333-333333333333",
  "type": "runtime.process.exited",
  "severity": "error",
  "payload": {
    "exitCode": 1,
    "signal": null,
    "expected": false,
    "message": "Runtime process exited while a prompt was active"
  }
}
```

### 3.2 Session lifecycle

| Type | When emitted | Payload summary |
|---|---|---|
| `session.created` | Tracer session record created | `title` optional, `cwd` project root reference |
| `session.ready` | Session may accept prompts | `runtimeSessionId` optional |
| `session.prompt.submitted` | User/control-plane accepted a prompt | `promptId`, `text` (may be redacted in logs), `attachments` count |
| `session.status.changed` | High-level status transition | `from`, `to`, `reason` optional |
| `session.completed` | Agent run finished successfully | `summary` optional |
| `session.failed` | Terminal failure for session/run | `errorClass`, `message` |
| `session.cancelled` | Cancellation completed or forced | `reason`, `partial` boolean |

#### Session status values (control plane)

```text
creating
starting_runtime
ready
running
awaiting_approval
cancelling
completed
failed
disconnected
stopped
```

Transitions must be validated by the control plane (see vertical slice). Invalid transitions emit `adapter.protocol.error` or a `session.status.changed` with `reason: "invalid_transition_corrected"` only when recovery is safe; prefer failing closed for destructive ambiguity.

#### Example: prompt submitted

```json
{
  "eventVersion": 1,
  "eventId": "b2000000-0000-4000-8000-000000000010",
  "sequence": 5,
  "timestamp": "2026-07-17T12:00:05.000Z",
  "projectId": "11111111-1111-1111-1111-111111111111",
  "sessionId": "22222222-2222-2222-2222-222222222222",
  "agentRunId": "33333333-3333-3333-3333-333333333333",
  "type": "session.prompt.submitted",
  "payload": {
    "promptId": "55555555-5555-5555-5555-555555555555",
    "text": "Summarize the repository structure and list open TODOs.",
    "attachmentCount": 0
  }
}
```

### 3.3 Agent messaging and planning

| Type | Payload summary |
|---|---|
| `agent.message.delta` | Streaming assistant/user/system text fragment: `messageId`, `role`, `delta`, `contentType` |
| `agent.message.completed` | Final message boundary: `messageId`, `role`, `fullText` optional if deltas already streamed |
| `agent.progress.delta` | Progress text or percentage: `message`, `percent` optional |
| `agent.plan.updated` | Structured plan snapshot or patch: `planId`, `steps[]` with `id`, `title`, `status` |

#### Example: plan update

```json
{
  "eventVersion": 1,
  "eventId": "c3000000-0000-4000-8000-000000000020",
  "sequence": 15,
  "timestamp": "2026-07-17T12:00:10.000Z",
  "projectId": "11111111-1111-1111-1111-111111111111",
  "sessionId": "22222222-2222-2222-2222-222222222222",
  "agentRunId": "33333333-3333-3333-3333-333333333333",
  "type": "agent.plan.updated",
  "payload": {
    "planId": "plan-1",
    "revision": 2,
    "steps": [
      { "id": "s1", "title": "Map project layout", "status": "completed" },
      { "id": "s2", "title": "Collect TODOs", "status": "running" },
      { "id": "s3", "title": "Write summary", "status": "pending" }
    ]
  }
}
```

### 3.4 Tools

| Type | Payload summary |
|---|---|
| `tool.started` | `toolCallId`, `name`, `input` (JSON-safe, may be redacted), `risk` optional |
| `tool.updated` | Partial progress: `toolCallId`, `status`, `partialOutput` optional |
| `tool.completed` | `toolCallId`, `output` optional, `durationMs` optional |
| `tool.failed` | `toolCallId`, `errorClass`, `message` |

Tool `status` values: `pending` \| `running` \| `completed` \| `failed` \| `cancelled`.

### 3.5 Approvals

| Type | Payload summary |
|---|---|
| `approval.requested` | `approvalId`, `action`, `description`, `risk`, `toolCallId` optional, `defaultDecision` never auto-applied for unknown risk |
| `approval.resolved` | `approvalId`, `decision` (`allow`\|`deny`\|`cancel`), `decidedBy` (`user`\|`policy`\|`system`), `reason` optional |

**Fail closed:** unknown or unsupported permission requests must surface as `approval.requested` (or fail the tool) and must **not** be auto-approved.

### 3.6 Files, terminal, storage, adapter errors

| Type | Payload summary |
|---|---|
| `file.changed` | `path` (repo-relative), `kind` (`created`\|`modified`\|`deleted`\|`renamed`), `previousPath` optional |
| `file.diff.available` | `path`, `diffId` or inline `unifiedDiff` when small; large diffs referenced by id |
| `terminal.output` | `streamId`, `chunk`, `stream` (`stdout`\|`stderr`) |
| `terminal.exited` | `streamId`, `exitCode` |
| `storage.error` | Persistence failure that is user-visible or session-impacting |
| `adapter.protocol.error` | Protocol/parse/negotiation error that did not necessarily kill the process |
| `adapter.protocol.unknown` | Unmapped but accepted runtime notification |

## 4. Cancellation behavior (events)

When the user or control plane cancels:

1. Emit `session.status.changed` with `to: "cancelling"` (if not already terminal).
2. Adapter issues runtime cancel if capability `cancellation` is true; otherwise proceed to local stop.
3. Outstanding tools that stop should end as `tool.failed` or `tool.completed` with cancelled semantics documented in payload (`status: "cancelled"` on update events preferred).
4. When cancel finishes (runtime ack, timeout, or forced process stop), emit `session.cancelled` then terminal status `stopped` or `cancelled` via `session.status.changed`.
5. If the process dies during cancel, emit `runtime.process.exited` / `runtime.process.failed` and map session to `disconnected` or `failed` with a clear reason—never a silent success.

Cancellation must not orphan child processes (process manager responsibility; reflected in events).

## 5. Process-exit behavior (events)

| Scenario | Expected events (order illustrative) | Session outcome |
|---|---|---|
| Clean stop after complete | `session.completed` → stop runtime → `runtime.process.exited` (`expected: true`) | `completed` |
| User stop | cancel path → `session.cancelled` → `runtime.process.exited` (`expected: true`) | `stopped` / cancelled |
| Crash mid-run | `runtime.process.exited` (`expected: false`) and/or `runtime.process.failed` → `session.failed` or `session.status.changed` to `disconnected` | `failed` or `disconnected` |
| Exit before ready | `runtime.process.failed` or exited + `session.failed` | `failed` |
| Stderr then crash | zero or more `runtime.process.stderr` then exit/fail | user-visible error |

The UI must show a failed/disconnected state; "still running" after exit is a bug.

## 6. Streaming and batching

- High-frequency deltas (`agent.message.delta`, `terminal.output`, `runtime.process.stderr`) MAY be batched on the UI channel for performance, but each logical event keeps its own `eventId`/`sequence` when persisted.
- Batching must not reorder `sequence`.
- UI should handle partial streams after reconnect by reloading from storage ordered by `sequence`.

## 7. Redaction and safety

Do not put secrets, API tokens, or credentials into persisted events or fixtures.

Prompt text may be stored for product history in local DB; fixtures used in tests must use sanitized placeholders.

When truncating large chunks, set `truncated: true` and prefer storing full content behind a size-capped blob policy defined by storage (out of scope for this protocol except the flag).

## 8. Compatibility guarantees for Wave 1

Implementations **MUST**:

1. Produce and accept `eventVersion: 1` envelopes with all required fields.
2. Assign monotonic `sequence` per Tracer session.
3. Tolerate unknown `type` values.
4. Tolerate unknown fields.
5. Map process death into explicit lifecycle events (never silent).
6. Fail closed on unknown approvals.

Implementations **MUST NOT**:

1. Require UI to parse raw ACP JSON-RPC.
2. Use runtime-native ids as primary keys.
3. Auto-approve unknown destructive actions.
4. Embed machine-specific absolute paths in committed fixtures.

## 9. Schema evolution

- Additive event types and optional payload fields: allowed with minor contract bump and dual tests.
- Changing meaning of existing fields or removing required fields: major version (`eventVersion: 2`) with migration plan.
- Wave 1 agents treat this file as frozen after Gate 0 unless a formal contract-change proposal is approved.

## 10. Minimal fixture corpus (for W1-B / W1-G)

Contract tests should include at least:

1. Happy-path stream: ready → prompt → message deltas → message completed → session completed.
2. Tool start/complete with approval request/resolve.
3. Unknown notification → `adapter.protocol.unknown`.
4. Malformed frame → `adapter.protocol.error`.
5. Cancel mid-tool.
6. Unexpected process exit mid-run.
7. Replay of stored events sorted by `sequence`.

---

**Document control:** Wave 0 deliverable. Amend only through contract-change process after Gate 0.
