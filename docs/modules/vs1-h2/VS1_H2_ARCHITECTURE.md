# VS1-H2 Architecture — Desktop Snapshot Wiring

**Task:** `tracer-vs1-desktop-wiring` (work item VS1-H2)  
**Scope:** `apps/desktop/src/`, `apps/desktop/src-tauri/src/commands/`, `packages/ui/`, `tests/e2e/desktop/`, `docs/modules/vs1-h2/`  
**Base:** VS1 main (`15c9399`) after Gate 1.3 control-plane acceptance

## 1. Purpose

Replace vertical-slice **mock-store ownership** of the core user journey with the **real typed command + presentation snapshot** flow.

React is a pure presentation consumer:

| React may | React must not |
|---|---|
| Receive `PresentationSnapshot` | Parse raw ACP / vendor wire |
| Invoke `tracer_*` commands | Write SQLite |
| Render normalized events | Own process lifecycle |
| Map `errorClass` → banners | Auto-approve permissions |

## 2. Layering

```text
┌─────────────────────────────────────────────────────────────┐
│ React shell (AppShell, Projects, Session workspace)         │
│  - Renders StatusChip / RuntimePill / PresentationContainer │
│  - Local UI only: route, composer text, side tab            │
└───────────────────────────┬─────────────────────────────────┘
                            │ AppViewState + SnapshotJourney
┌───────────────────────────▼─────────────────────────────────┐
│ SnapshotJourney (apps/desktop/src/shared/store/snapshotStore)│
│  - bootstrapLoad / refreshSnapshot / submitPrompt / …       │
│  - Missed live events → tracer_presentation_snapshot        │
│  - History reopen → tracer_events_list                      │
└───────────────────────────┬─────────────────────────────────┘
                            │ invokeTracer(command, args)
┌───────────────────────────▼─────────────────────────────────┐
│ invoke layer (shared/commands/invoke.ts)                    │
│  - auto: Tauri if __TAURI__.core.invoke else mock           │
│  - mode force: mock | tauri | auto                          │
│  - Errors: JSON CommandError → TracerInvokeError            │
└───────────────┬─────────────────────────────┬───────────────┘
                │ tauri                       │ mock
┌───────────────▼──────────────┐  ┌───────────▼────────────────┐
│ Tauri commands (W1-F glue)   │  │ MockBackend (deterministic)│
│ tracer_presentation_snapshot │  │ browser dev + unit tests   │
│ tracer_* session/project/…   │  │ no network / no credentials│
└───────────────┬──────────────┘  └────────────────────────────┘
                │
┌───────────────▼──────────────┐
│ tracer-control-plane         │
│ PresentationSnapshot v1      │
│ (read-only from this task)   │
└──────────────────────────────┘
```

## 3. Core journey mapping

| User step | Command / mechanism |
|---|---|
| Application opens | `tracer_presentation_snapshot` + `tracer_heli_status` + `tracer_project_list` |
| Inspect snapshot | `refreshSnapshot()` |
| Inspect runtime availability | `tracer_runtime_status` + snapshot `runtimeObservation` |
| Start runtime / create session | `tracer_session_create` (control plane starts runtime) |
| Submit prompt | `tracer_session_submit_prompt` |
| Render streaming output | `tracer_events_list` + normalized `agent.message.delta` |
| Display approval | snapshot `pendingApprovals` + status `awaiting_approval` |
| Approve / reject / cancel | `tracer_approval_resolve` (`allow` \| `deny` \| `cancel`) |
| Terminal state | snapshot `sessionStatus` ∈ completed/failed/stopped/disconnected |
| Reopen history | `openSession` → snapshot + `tracer_events_list` |
| Heli unavailable | `tracer_heli_status` → non-fatal banner; load continues |

## 4. Typed snapshot contract

Aligned with `crates/tracer-control-plane/src/types.rs` `PresentationSnapshot` (camelCase JSON):

- `version`, `activeProjectId`, `activeSessionId`
- `sessionStatus`, `runtimeObservation`, `authState`
- `pendingApprovals[]`, `heli`, `lastError`
- `capabilities`, `latestSequence`, `promptInFlight`

### Runtime observation mapping

Control plane emits gate strings (`ready`, `disconnected`, `unavailable`, …).  
UI `RuntimePill` catalog is fixed (`not_started` | `starting` | `ready` | `sign_in_required` | `stopped` | `crashed` | `unavailable`).

Mapping lives in `mapRuntimeObservation()` — auth blocking forces `sign_in_required`.

### Failure mapping

`errorClass` → `NormalizedFailureKind` → global banner or session banner:

| errorClass | Presentation |
|---|---|
| RuntimeExecutableNotFound / RuntimeSpawnFailed | `runtime_missing` banner |
| RuntimeCrashed | disconnected / crashed pill |
| AuthenticationRequired | sign-in banner; composer disabled |
| StorageError | storage warning banner |
| InternalError | control_plane_down banner |

## 5. Mock fallback policy

| Environment | Backend |
|---|---|
| Tauri desktop (`__TAURI__` present) | **Real commands** (auto) |
| Browser Vite dev | MockBackend |
| Vitest unit tests | MockBackend (forced) |

Mock is for deterministic UX + offline shell only. It must not imply live model output (`demoRuntime` badge).

## 6. Accessibility

Unchanged STATE_MATRIX rules:

- Status always text + icon (StatusChip / RuntimePill)
- Composer disabled reasons visible
- Approval interrupt uses `aria-live`
- Keyboard-focusable Buttons retained

## 7. Out of scope (forbidden)

- Full IDE, editor, terminal, file explorer
- ALMS / plugins / collaboration / marketplace
- Control-plane redesign
- Live Grok / credentials in standard tests
- Wave 2 product expansion

## 8. Key files

| Path | Role |
|---|---|
| `apps/desktop/src/shared/types/snapshot.ts` | Snapshot DTOs + mappers |
| `apps/desktop/src/shared/commands/invoke.ts` | Tauri/mock invoke |
| `apps/desktop/src/shared/commands/mockBackend.ts` | Deterministic command backend |
| `apps/desktop/src/shared/store/snapshotStore.ts` | Journey controller + view state |
| `apps/desktop/src/features/sessions/SessionWorkspacePlaceholder.tsx` | Snapshot-driven session UI |
| `apps/desktop/src-tauri/src/commands/mod.rs` | Existing W1-F command glue (unchanged) |