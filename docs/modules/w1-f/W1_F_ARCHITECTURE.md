# W1-F Architecture — Control Plane

**Task:** `tracer-w1-control-plane`  
**Crate:** `crates/tracer-control-plane`  
**Desktop glue:** `apps/desktop/src-tauri/src/control_plane/`, `commands/`

## 1. Ownership

| Concern | Owner |
|---|---|
| Adapter lifecycle | W1-D `tracer-runtime-adapter` (consumed only) |
| Sole SQLite writer | **W1-F** via `tracer-storage` |
| Permission decisions | **W1-F** (never auto-approve) |
| Tauri commands | **W1-F** thin handlers |
| Presentation snapshots | **W1-F** (typed; no raw ACP) |
| Heli | W1-H read-only (`tracer-heli`) |

## 2. Component shape

```text
ControlPlane
├── RuntimeSupervisor      # start / initialize / create_session / shutdown via adapter
├── SessionCoordinator     # Tracer session + prompt lifecycle + status
├── EventIngestor          # continuous drain of adapter events
├── PersistenceCoordinator # SOLE DB writer (append_event, session status, approvals)
├── ApprovalCoordinator    # list pending / resolve once
├── CancellationCoordinator# cancel + escalate to process_stop if unsupported
├── PresentationProjector  # versioned PresentationSnapshot
└── RecoveryCoordinator    # reconcile_stale_live_sessions on open
```

## 3. Lifecycle

1. `project_register` → SQLite project row  
2. `session_create` → session row → `RuntimeAdapter::start` → **start ingestor** → `initialize` → `create_session`  
3. `session_submit_prompt` → status Running → block on adapter prompt (OS thread) while ingest continues  
4. Events persist with storage-authoritative `sequence` / `eventId`  
5. `session_stop` / Drop → graceful shutdown + force via W1-C  

## 4. Presentation

- `PresentationSnapshot` v1: session status, auth, pending approvals, heli, capabilities, latest sequence  
- Live fan-out optional (`PresentationEvent` / `tracer://events`)  
- Shell may restore from snapshot if events missed  
- **No raw ACP/Grok** in snapshot or command results  

## 5. Heli

- `probe_heli` / `refresh_heli` are read-only  
- Missing workspace → `available: false` summary; **no crash**  
- Heli ≠ ACP session  

## 6. Forbidden

- Parse ACP in React or command layer  
- Auto-approve permissions  
- Treat process-alive as session/prompt ready  
- Direct SQLite from Tauri handlers  
- Live Grok required for standard CI  
