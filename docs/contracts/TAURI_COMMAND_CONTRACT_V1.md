# Tauri Command Contract v1

**Status:** Gate 0 contract (Wave 0 freeze candidate)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-architecture-contracts`  
**Applies to:** desktop UI ↔ Rust control plane request/response surface and event streaming channel

## 1. Purpose

This document freezes the **initial Tauri command names**, argument shapes, result shapes, **error classes**, and the **event stream** used by the first vertical slice.

Principles:

1. Commands are **request/response** for user-driven operations.
2. High-frequency runtime traffic uses a **stream/channel of normalized Tracer events**, not per-delta commands.
3. The frontend never calls the runtime or OS process APIs directly.
4. Errors are structured and stable for UI mapping.
5. Paths are validated on the Rust side; committed examples use placeholders, not machine-specific absolute paths.

Related:

- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`
- `docs/architecture/TRACER_VERTICAL_SLICE.md`

## 2. Transport model

### 2.1 Commands

Tauri `invoke` commands registered by the control plane (names below are the public contract).

Naming convention:

```text
tracer_<domain>_<verb>
```

Domains for v1: `project`, `session`, `runtime`, `approval`, `events` (query), `app`.

### 2.2 Event stream

After a session is active (or app-wide subscription is established), the UI receives **normalized events** via:

- Tauri events, **or**
- a channel / batched event API

**Contract name for the frontend subscription:**

```text
tracer://events
```

Payload: one `TracerEvent` envelope (protocol v1) or a batch:

```json
{
  "batch": true,
  "events": [ /* TracerEvent[] in sequence order */ ]
}
```

Rules:

- Batches preserve ascending `sequence` within a session.
- UI must handle both single and batch forms.
- Reconnect/reload uses query commands, not only the live stream.

### 2.3 Versioning

Commands may add optional fields without a major bump. Renames/removals require a contract revision. Frontend should ignore unknown result fields.

## 3. Shared types

### 3.1 Identifiers

All Tracer ids are UUID strings unless noted.

```text
ProjectId, SessionId, AgentRunId, EventId, PromptId, ApprovalId, ToolCallId
```

### 3.2 Command error envelope

Every failed command returns a structured error (exact Rust/TS encoding may wrap this):

```json
{
  "errorClass": "InvalidArgument",
  "message": "projectId is required",
  "retryable": false,
  "details": {}
}
```

UI maps `errorClass` to banners/toasts; `message` is human-readable.

### 3.3 Error classes (command surface)

Includes adapter classes plus UI-facing ones:

| errorClass | Typical commands |
|---|---|
| `InvalidArgument` | any |
| `NotFound` | get/list by id |
| `AlreadyExists` | register project / create |
| `InvalidState` | prompt while not ready, approve twice |
| `PermissionDenied` | policy deny |
| `RuntimeExecutableNotFound` | start runtime |
| `RuntimeSpawnFailed` | start runtime |
| `RuntimeNotReady` | submit prompt |
| `RuntimeDisconnected` | ops after crash |
| `RuntimeCrashed` | session ops |
| `ProtocolInitializeFailed` | start/create session |
| `CapabilityMismatch` | start |
| `CapabilityUnsupported` | cancel when unsupported |
| `CancellationFailed` | cancel/stop |
| `Timeout` | start, cancel, stop |
| `StorageError` | any persist path |
| `InternalError` | unexpected |
| `Unsupported` | feature not in slice |
| `UserCancelled` | native dialogs if used |

## 4. Project commands

### 4.1 `tracer_project_list`

**Args:** none (or `{ "includeMissing": boolean }` optional)

**Result:**

```json
{
  "projects": [
    {
      "projectId": "11111111-1111-1111-1111-111111111111",
      "name": "tracer",
      "rootPath": "<user-selected-absolute-path>",
      "status": "ready",
      "lastOpenedAt": "2026-07-17T12:00:00.000Z"
    }
  ]
}
```

`status`: `ready` \| `missing` \| `invalid`.

Note: absolute paths appear only as **user-local runtime data**, never in committed fixtures.

### 4.2 `tracer_project_register`

**Args:**

```json
{
  "rootPath": "<user-selected-absolute-path>",
  "name": "optional display name"
}
```

**Result:** `{ "project": { /* Project summary */ } }`

**Errors:** `InvalidArgument`, `NotFound` (path missing), `AlreadyExists`, `StorageError`.

Validation: path exists, is directory; Git repo preferred but not hard-required for register in v1 (record `isGit` boolean if detected).

### 4.3 `tracer_project_get`

**Args:** `{ "projectId": "..." }`

**Result:** `{ "project": { /* detail */ } }`

### 4.4 `tracer_project_remove`

**Args:** `{ "projectId": "...", "deleteHistory": false }`

**Result:** `{ "removed": true }`

Does not delete the user's source tree on disk. Optional history deletion only affects Tracer storage.

## 5. Session commands

### 5.1 `tracer_session_list`

**Args:** `{ "projectId": "...", "limit": 50, "cursor": null }`

**Result:**

```json
{
  "sessions": [
    {
      "sessionId": "22222222-2222-2222-2222-222222222222",
      "projectId": "11111111-1111-1111-1111-111111111111",
      "title": "Summarize repository",
      "status": "completed",
      "createdAt": "2026-07-17T12:00:00.000Z",
      "updatedAt": "2026-07-17T12:10:00.000Z"
    }
  ],
  "nextCursor": null
}
```

### 5.2 `tracer_session_create`

Creates a Tracer session and starts the runtime path for the vertical slice (may be split later; v1 allows combined for speed).

**Args:**

```json
{
  "projectId": "11111111-1111-1111-1111-111111111111",
  "title": "optional",
  "runtime": {
    "runtimeKind": "acp-stdio",
    "executableOverride": null,
    "extraArgs": []
  }
}
```

**Result:**

```json
{
  "session": {
    "sessionId": "22222222-2222-2222-2222-222222222222",
    "projectId": "11111111-1111-1111-1111-111111111111",
    "status": "starting_runtime",
    "capabilities": null
  }
}
```

Side effects: process spawn, initialize, capability negotiation; stream emits `session.created`, runtime lifecycle events, eventually `session.ready` or failure events.

**Errors:** project errors, runtime spawn/init classes, `StorageError`, `Timeout`.

### 5.3 `tracer_session_get`

**Args:** `{ "sessionId": "..." }`

**Result:** session detail including `status`, `capabilities`, `lastError` optional, runtime summary.

### 5.4 `tracer_session_submit_prompt`

**Args:**

```json
{
  "sessionId": "22222222-2222-2222-2222-222222222222",
  "text": "Summarize the repository structure.",
  "attachments": []
}
```

**Result:**

```json
{
  "promptId": "55555555-5555-5555-5555-555555555555",
  "agentRunId": "33333333-3333-3333-3333-333333333333",
  "accepted": true
}
```

Preconditions: session `ready` or equivalent accepting state; runtime ready.

**Errors:** `InvalidState`, `RuntimeNotReady`, `RuntimeDisconnected`, `PromptRejected`, `StorageError`.

Streaming content arrives only via `tracer://events`.

### 5.5 `tracer_session_cancel`

**Args:**

```json
{
  "sessionId": "...",
  "scope": "active_run"
}
```

`scope`: `active_run` \| `session` (v1 implementations may treat both as cancel active work).

**Result:**

```json
{
  "accepted": true,
  "mode": "cooperative"
}
```

`mode`: `cooperative` \| `process_stop` \| `already_terminal`.

**Errors:** `InvalidState`, `CapabilityUnsupported` (if caller required cooperative-only), `CancellationFailed`, `Timeout`.

### 5.6 `tracer_session_stop`

Stops runtime for the session (stronger than cancel). Always aims for no orphans.

**Args:** `{ "sessionId": "...", "force": false }`

**Result:** `{ "stopped": true }`

### 5.7 `tracer_session_subscribe`

Ensures the UI receives events for a session (if subscription is not app-global).

**Args:** `{ "sessionId": "..." }`

**Result:** `{ "subscribed": true, "channel": "tracer://events" }`

Implementations may no-op if global broadcast already includes the session.

## 6. Event query commands

### 6.1 `tracer_events_list`

Reload/history API.

**Args:**

```json
{
  "sessionId": "...",
  "afterSequence": 0,
  "limit": 200
}
```

**Result:**

```json
{
  "events": [ /* TracerEvent envelopes ordered by sequence asc */ ],
  "latestSequence": 42
}
```

### 6.2 `tracer_events_get`

**Args:** `{ "sessionId": "...", "eventId": "..." }`

**Result:** `{ "event": { /* envelope */ } }`

## 7. Approval commands

### 7.1 `tracer_approval_list_pending`

**Args:** `{ "sessionId": "..." }`

**Result:** `{ "approvals": [ { "approvalId", "action", "description", "risk", "createdAt" } ] }`

### 7.2 `tracer_approval_resolve`

**Args:**

```json
{
  "sessionId": "...",
  "approvalId": "...",
  "decision": "allow",
  "reason": "optional"
}
```

`decision`: `allow` \| `deny` \| `cancel`.

**Result:** `{ "resolved": true }`

**Errors:** `NotFound`, `InvalidState` (already resolved), `PermissionDenied` (policy blocks allow), adapter errors.

Unknown approval kinds must not have been auto-allowed; resolving with `allow` still goes through policy checks.

## 8. Runtime diagnostic commands (minimal)

### 8.1 `tracer_runtime_status`

**Args:** `{ "sessionId": "..." }` optional; omit for global

**Result:**

```json
{
  "processes": [
    {
      "sessionId": "...",
      "state": "ready",
      "pid": 12345,
      "runtimeKind": "acp-stdio",
      "capabilities": { "cancellation": true }
    }
  ]
}
```

`pid` may be null on platforms/restrictions; do not require it for correctness.

### 8.2 `tracer_runtime_describe_installations`

**Args:** none

**Result:** discovered/configured runtime installations (no secrets).

## 9. App commands

### 9.1 `tracer_app_info`

**Result:** `{ "appVersion", "eventProtocolVersion": 1, "commandContractVersion": "1.0.0", "platform" }`

### 9.2 `tracer_app_open_path_dialog` (optional in slice)

Native folder picker helper.

**Args:** `{ "title": "Open repository" }`

**Result:** `{ "path": "<user-selected>" }` or cancelled → `UserCancelled`.

## 10. Command catalog summary

| Command | Slice priority |
|---|---|
| `tracer_project_list` | required |
| `tracer_project_register` | required |
| `tracer_project_get` | required |
| `tracer_project_remove` | optional for Gate 1, required Gate 2 |
| `tracer_session_list` | required |
| `tracer_session_create` | required |
| `tracer_session_get` | required |
| `tracer_session_submit_prompt` | required |
| `tracer_session_cancel` | required |
| `tracer_session_stop` | required |
| `tracer_session_subscribe` | required if not global stream |
| `tracer_events_list` | required |
| `tracer_events_get` | optional |
| `tracer_approval_list_pending` | required if approvals capability used |
| `tracer_approval_resolve` | required if approvals capability used |
| `tracer_runtime_status` | required |
| `tracer_runtime_describe_installations` | optional |
| `tracer_app_info` | required |
| `tracer_app_open_path_dialog` | recommended |

## 11. State preconditions (normative)

| Command | Allowed session statuses (non-exhaustive) |
|---|---|
| `tracer_session_submit_prompt` | `ready` (and not `cancelling`) |
| `tracer_session_cancel` | `running`, `awaiting_approval` |
| `tracer_session_stop` | any non-terminal or disconnect recovery |
| `tracer_approval_resolve` | `awaiting_approval` with matching pending id |

On violation → `InvalidState` with message stating current status.

## 12. Cancellation and process-exit as seen by UI

1. User invokes `tracer_session_cancel` or `tracer_session_stop`.
2. UI immediately shows cancelling/stopping from optimistic UI **or** waits for `session.status.changed` (prefer event-authoritative status).
3. Live stream delivers cancel/exit related events per event protocol.
4. Final `tracer_session_get` reflects terminal status.
5. If process crashes without user action, stream delivers `runtime.process.exited` / `failed`; commands return `RuntimeDisconnected` / `RuntimeCrashed` thereafter.

## 13. Security notes

- All commands that mutate state run in Rust and re-validate ids and status.
- Frontend-supplied paths are canonicalized and checked.
- No command accepts raw ACP payloads from the UI.
- No command returns secrets or full environment dumps.

## 14. Testing expectations

Contract tests (Wave 1) should mock the control plane and assert:

- command names and required args
- errorClass stability
- event subscription delivers envelopes with `eventVersion: 1`
- invalid state matrix for prompt/cancel/approve

---

**Document control:** Wave 0 deliverable. Feature modules must not invent parallel invoke names for the same operations.
