# Tracer Vertical Slice (Gate 0 / Gate 1 Target)

**Status:** Architecture freeze candidate for Wave 0  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-architecture-contracts`

## 1. Objective

Prove one end-to-end local loop before expanding product surface area:

> Open a local repository → start an ACP-compatible agent runtime → create a session → submit a prompt → stream normalized agent/tool events → show changed files and runtime state → persist the session → stop or resume safely.

This is the **only** success criterion that unlocks broad Wave 2 feature work. Partial UI shells without a real control-plane loop do not pass Gate 1.

## 2. Product boundary (slice)

### In scope

- Register a local project (folder path)
- Launch managed sidecar runtime (stdio ACP)
- Create session bound to project
- Submit one or more prompts
- Stream normalized events to UI timeline
- Show runtime status (ready / running / failed / disconnected)
- Basic approval interrupt when runtime requests permission
- Persist events and session metadata in SQLite
- Cancel active run and stop runtime without orphans
- Reload session history after app restart

### Out of scope (explicit)

- Full IDE / multi-file editor
- Cloud multi-tenant hosting
- ALMS multi-agent orchestration
- Runtime fork / rebrand
- Production auto-update
- Collaboration / multi-user
- Replacing HeliHarness governance

## 3. Domain vocabulary

| Term | Definition |
|---|---|
| **Project** | User-registered local directory Tracer manages sessions against |
| **Runtime installation** | Configured executable + args + kind that can be spawned |
| **Runtime process** | One OS child process managed by Tracer |
| **Session** | Tracer-owned conversation/work unit bound to a project |
| **Agent run** | One prompt execution interval inside a session |
| **Event** | Normalized Tracer Event Protocol v1 envelope |
| **Tool call** | Agent-invoked tool lifecycle surfaced as events |
| **Approval** | Permission request requiring user/policy decision |
| **Adapter** | Translation layer between ACP runtime and Tracer domain |
| **Control plane** | Trusted Rust backend composing process, adapter, storage, policy |
| **Sidecar** | Runtime process separate from the UI process |

Identifiers: Tracer UUIDs for primary keys; runtime-native ids only as adapter metadata.

## 4. Logical architecture

```text
┌──────────────────────────────────────────────┐
│ Tracer Desktop UI (React / TypeScript)       │
│  projects · session · timeline · approvals   │
└─────────────────┬────────────────────────────┘
                  │ Tauri commands + tracer://events
┌─────────────────▼────────────────────────────┐
│ Control plane (Rust)                         │
│  project/session services · permissions      │
│  event sequencing · single DB writer         │
└───┬───────────────┬───────────────┬──────────┘
    │               │               │
    ▼               ▼               ▼
 process mgr    ACP adapter     SQLite storage
    │               │
    └──── stdio ────┘
            │
            ▼
   ACP-compatible runtime (stock, fake, or future downstream)
```

### Design rules (normative)

1. **Runtime independence:** UI and storage schemas do not require Grok-specific shapes.
2. **Raw preservation:** optional adapter metadata only; UI never depends on it.
3. **Single database writer:** control plane only.
4. **Process isolation:** only Rust spawns/kills runtimes and tools shells.
5. **Fail closed:** unknown permissions are not auto-approved.
6. **No silent success:** completion requires evidence events or explicit failure.

## 5. Vertical slice flow

### 5.1 Happy path

```text
1. User registers project path
2. User creates session
3. Control plane spawns runtime process
4. Adapter initializes + negotiates capabilities
5. Events: runtime.process.started → runtime.process.ready → session.ready
6. User submits prompt
7. Events: session.prompt.submitted → agent.message.* / tool.* / plan.*
8. Optional: approval.requested → user resolve → continue
9. session.completed (or ready for next prompt)
10. User stops session → graceful shutdown → runtime.process.exited expected
11. App restart → tracer_events_list replays timeline
```

### 5.2 Failure paths (must be designed, not improvised)

| Failure | User-visible outcome |
|---|---|
| Executable missing | Clear error; session failed to start |
| Init/handshake fail | Failed/disconnected; stderr available if any |
| Crash mid-prompt | Timeline shows exit; status disconnected/failed; no orphan |
| Malformed ACP message | Protocol error event; session continues if possible |
| Cancel timeout | Force kill; cancelled/stopped with honesty |
| Storage write fail | `storage.error`; do not claim persistence succeeded |

## 6. Module ownership for implementation waves

Wave 0 freezes contracts only. Wave 1 maps roughly to:

| Module | Responsibility |
|---|---|
| Desktop shell | App chrome, invoke wrapper, placeholders |
| Domain + events | Envelope types, fixtures, ser/de tests |
| Process manager | Spawn, pipes, shutdown, orphans |
| ACP adapter | JSON-RPC, normalize, cancel |
| Storage | SQLite, migrations, ordered reads |
| Control plane | Compose + Tauri commands + stream |
| Fake runtime | Deterministic CI scenarios |

Feature polish (projects UX, rich timeline, diff viewer, terminal) is Wave 2 **after** Gate 1.

## 7. Status model

Session status (control plane):

```text
creating → starting_runtime → ready ⇄ running ⇄ awaiting_approval
                │                  │
                │                  ├→ cancelling → stopped
                │                  ├→ completed
                │                  ├→ failed
                └→ failed/disconnected
```

UI must provide non-color-only indicators for: empty, loading, running, failed, disconnected, completed (UX details owned by W0-C; statuses above are backend truth).

## 8. Persistence requirements (slice)

Minimum durable records:

- Project
- Session (status, timestamps, runtime binding metadata)
- Events (full envelope JSON or columnar equivalent preserving order)
- Runtime process summary (for diagnostics)
- Approval decisions (audit)

Not required for Gate 1: full artifact store, usage billing, multi-device sync.

## 9. Runtime strategy for the slice

1. Default development/CI path: **fake ACP runtime** with scripted scenarios.
2. Optional smoke: **stock ACP-compatible runtime** (for example Grok Build) when available—mapping evidence from W0-B.
3. Downstream fork (`tracer-agent-runtime`) only after runtime adoption gate (see master plan); not needed to complete this slice.

See `docs/decisions/ADR-001-runtime-sidecar.md`.

## 10. Contract surfaces frozen by W0-A

| Document | Freezes |
|---|---|
| `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md` | Envelope, types, unknown/cancel/exit event behavior |
| `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md` | Adapter API, capabilities, error classes |
| `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` | Command names, stream channel, command errors |
| `docs/decisions/ADR-001-runtime-sidecar.md` | Process isolation decision |
| `docs/decisions/ADR-002-event-normalization.md` | Normalization decision |

W0-B supplies empirical ACP mapping. W0-C maps UI states. W0-D defines acceptance tests. Integration order: **W0-A → W0-B → W0-C / W0-D** (C/D after A+B integrated preferred; C/D must not contradict A).

## 11. Gate 0 exit (architecture completeness)

Gate 0 architecture/contract portion is complete when:

- [x] Event envelope v1 specified with examples
- [x] Adapter interface and capability negotiation specified
- [x] Tauri command names and error classes specified
- [x] Unknown-event, cancellation, process-exit behavior specified
- [x] ADRs for sidecar and normalization recorded
- [x] Vertical slice scope and vocabulary unambiguous
- [ ] Human maintainer approval (coordinator/human)
- [ ] Cross-wave contradiction pass after W0-B/C/D land

## 12. Gate 1 exit (implementation target)

Gate 1 requires running software evidence:

- App starts from clean checkout instructions
- Fake runtime flow without network/paid APIs
- UI does not parse raw ACP
- Events persisted and reloadable
- Crash and stop paths leave no orphans
- Limitations documented per platform

## 13. Risks and open points

| Risk | Mitigation |
|---|---|
| Stock runtime ACP differs from assumptions | W0-B evidence; adapter metadata; fake runtime for CI |
| Framing ambiguity (newline vs Content-Length) | Decide in W1 adapter using W0-B; contract allows one choice with tests |
| Windows process kill / signal differences | Process manager acceptance matrix; document platform gaps |
| Event volume performance | Batching allowed; virtualization in Wave 2 timeline |
| Contract drift across parallel agents | Path ownership + Gate 0 freeze + change process |

Open points deferred (must not block Gate 0 docs):

- Exact stock CLI flags (W0-B)
- Pixel-level UI layout (W0-C)
- Full CI matrix YAML (W0-D)
- SQLite physical schema (W1-E)

## 14. Path policy

All repository paths in docs and code are relative to the Tracer repo root or workspace root as appropriate.

Do not commit:

- drive letters
- usernames
- home directories
- machine-local absolute paths in fixtures

User-selected absolute paths exist only in local application data at runtime.

---

**Document control:** Wave 0 architecture deliverable for `tracer-w0-architecture-contracts`.
