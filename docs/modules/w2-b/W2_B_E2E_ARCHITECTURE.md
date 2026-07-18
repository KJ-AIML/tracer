# W2-B E2E Architecture — Real Tauri GUI Boundary

**Task:** `tracer-w2-tauri-gui-e2e` (work item W2-B)  
**Branch:** `agent/tracer-w2-tauri-gui-e2e`  
**Scope (owned):** `apps/desktop/`, `tests/e2e/tauri/`, `tools/tauri-e2e/`, `docs/modules/w2-b/`  
**May touch:** desktop Tauri glue under `apps/desktop/src-tauri` for E2E harness hooks only  
**Forbidden:** control-plane redesign (W2-A), multi-session isolation (W2-C), live-grok-smoke (W2-D), domain/process/storage redesign, full IDE

## 1. Purpose

Deliver the **strongest practical automated desktop journey** that crosses the **actual Tauri application boundary** (composition + registered command surface + frontend invoke policy), with honest classification:

| Claim | Status |
|---|---|
| Desktop-boundary E2E (real command glue + CP + fake ACP + temp SQLite) | **Delivered / executable** |
| Frontend does not silently mock-downgrade when Tauri is selected | **Delivered / executable** |
| Full WebView GUI drive (launch window → click composer → assert DOM) | **Not delivered** — documented blocker + follow-up |

**No false full GUI E2E claim.**

## 2. Strength classification

```text
L3 Full GUI (WebDriver / Playwright + tauri-driver)
    ▲  follow-up — requires host WebView2 driver + app binary packaging
L2 App process smoke (spawn tracer-desktop binary)
    ▲  optional / platform SDK heavy — not default CI
L1 Desktop boundary journey  ← W2-B primary executable surface
    │  build_control_plane + plane_* handlers == Tauri invoke handlers
    │  fake ACP + temp file SQLite + reopen
L0 Frontend invoke policy     ← W2-B primary executable surface
    │  isTauriAvailable / resolveInvokeBackend / fail-closed Tauri errors
```

## 3. Preferred product journey (target path)

```text
launch built desktop app
→ frontend loads
→ Tauri invoke available (__TAURI__.core.invoke)
→ inspect presentation snapshot
→ start fake runtime (session_create + RuntimeCreateOptions.scenarioId)
→ create session
→ submit prompt
→ render streaming (events_list / snapshot)
→ approval or cancel
→ terminal state
→ close app
→ reopen
→ restore history (events_list + session_get)
```

### What W2-B automates today

| Step | How automated |
|---|---|
| App composition (same as Tauri `run()`) | `control_plane::build_control_plane` + env hooks |
| Command registration valid | `REGISTERED_COMMANDS` + `tracer_e2e_env` |
| Snapshot / project / session / prompt / stream | `plane_*` handlers in `desktop_boundary_journey` |
| Approval allow | concurrent prompt + `plane_approval_resolve` |
| Cancel mid-stream | `cancel_mid_stream` fake scenario |
| Terminal stop | `plane_session_stop` |
| Close → reopen → history | drop plane; reopen same `TRACER_DATABASE_PATH` |
| Heli unavailable non-fatal | empty `TRACER_HELI_PROBE_PATH` |
| No raw ACP to frontend surface | structured checks: domain event `type` (not ACP methods); adapter `runtimeMethod` provenance allowed |
| Tauri vs mock policy | vitest `invoke.policy.test.ts` |

### What remains for L3 full GUI

1. Package / build `tracer-desktop` with test env (`TRACER_DATABASE_PATH`, `TRACER_FAKE_ACP_JS`).
2. Install `tauri-driver` (or WebView2 WebDriver) on CI hosts.
3. Playwright/Selenium script against the WebView: assert DOM for status, timeline deltas, approval card.
4. Re-validate after **W2-A presentation delivery** (live event fan-out may change UI refresh cadence).

## 4. Layering

```text
┌──────────────────────────────────────────────────────────────┐
│ tools/tauri-e2e/run.mjs  (orchestrator, CI class report)     │
└────────────────────────────┬─────────────────────────────────┘
          ┌──────────────────┴──────────────────┐
          ▼                                     ▼
┌─────────────────────────┐       ┌─────────────────────────────┐
│ L0 invoke.policy.test.ts│       │ L1 desktop_boundary_journey │
│ resolveInvokeBackend    │       │ plane_* == tauri commands   │
│ no silent mock fallback │       │ fake ACP + temp SQLite      │
└───────────┬─────────────┘       └──────────────┬──────────────┘
            │                                    │
            ▼                                    ▼
┌─────────────────────────┐       ┌─────────────────────────────┐
│ apps/desktop invoke.ts  │       │ apps/desktop/src-tauri      │
│ SnapshotJourney (H2)    │       │ commands + control_plane    │
└─────────────────────────┘       └──────────────┬──────────────┘
                                                 │
                                                 ▼
                                  ┌─────────────────────────────┐
                                  │ tracer-control-plane        │
                                  │ (read-only from W2-B)       │
                                  └─────────────────────────────┘
```

## 5. E2E harness hooks (test-only env)

Consumed by `apps/desktop/src-tauri/src/control_plane/mod.rs` and `lib.rs::run()`:

| Env | Effect |
|---|---|
| `TRACER_DATABASE_PATH` | File SQLite path (persist + reopen) |
| `TRACER_FAKE_ACP_JS` | Absolute path to fake ACP script |
| `TRACER_HELI_PROBE_PATH` | Heli workspace probe root |
| `TRACER_NODE_BIN` | Node binary for fake ACP (default `node`) |

Command `tracer_e2e_env` returns a read-only diagnostic blob (paths + registered command list). Safe in production (no secrets).

## 6. Frontend invoke policy (fail-closed)

| Mode | Tauri present? | Backend | On invoke failure |
|---|---|---|---|
| `auto` | yes | **tauri** | `TracerInvokeError` — **never** mock |
| `auto` | no | mock (if installed) | mock handles / Unsupported |
| `tauri` | no | **tauri** | InternalError (*no silent mock downgrade*) |
| `tauri` | yes | tauri | structured error from IPC |
| `mock` | yes/no | mock | deterministic unit path |

## 7. Snapshot contract dependency (Gate 2.1)

Desktop boundary tests design against **current** `PresentationSnapshot` v1 fields from control plane (W1-F / VS1-H2).

**Integration note:** Gate 2.1 must re-validate after **W2-A presentation delivery** if snapshot shape, live event channel, or multi-subscriber semantics change. W2-B does not redesign the control plane.

## 8. CI class

```text
network: no
credentials: no
live Grok: no
provider: no
fake ACP: yes
temp file SQLite: yes
```

## 9. Key files

| Path | Role |
|---|---|
| `apps/desktop/src-tauri/src/control_plane/mod.rs` | Composition + E2E env |
| `apps/desktop/src-tauri/src/commands/mod.rs` | Tauri commands + `plane_*` testable handlers |
| `apps/desktop/src-tauri/src/lib.rs` | Register handlers including `tracer_e2e_env` |
| `apps/desktop/src-tauri/tests/desktop_boundary_journey.rs` | L1 executable journey |
| `apps/desktop/src/shared/commands/invoke.ts` | Fail-closed Tauri policy |
| `apps/desktop/src/shared/commands/invoke.policy.test.ts` | L0 policy tests |
| `tools/tauri-e2e/run.mjs` | Orchestrator |
| `tests/e2e/tauri/README.md` | Entry pointer |

## 10. Out of scope

- Control-plane redesign / presentation fan-out (W2-A)
- Multi-session CP isolation (W2-C)
- Live Grok smoke (W2-D)
- Full IDE surfaces
- ALMS / plugins / marketplace / cloud
