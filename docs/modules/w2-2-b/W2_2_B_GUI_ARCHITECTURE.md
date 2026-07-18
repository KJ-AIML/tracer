# W2.2-B GUI Architecture — Full WebView Product Journey (L3-J)

**Task:** `tracer-w2-webview-gui-journey`  
**Branch:** `agent/tracer-w2-webview-gui-journey`  
**Level:** L3-J (independent of L0–L3-I claims)

## 1. Purpose

Drive the **real Tracer desktop WebView** through product journeys GJ-01…GJ-12 using:

- actual `tracer-desktop` binary
- real WebDriver session (`tauri-driver` + matching `msedgedriver`)
- **fake ACP only**
- temp file-backed SQLite
- unique work dirs / ports / fixtures

No live Grok, no network credentials, no provider keys.

## 2. Layering

```text
pnpm test:tauri-e2e:gui
        │
        ▼
tools/tauri-e2e/l3j-gui.mjs
  doctor-ready stack → build → drivers → app env → WebDriver
        │
        ▼
tools/tauri-e2e/lib/journeys.mjs  (GJ-01…GJ-12 DOM flows)
        │
        ▼
apps/desktop WebView (React)
  data-testid="tracer-…" + a11y labels
  invokeTracer → __TAURI__.core.invoke (withGlobalTauri)
        │
        ▼
apps/desktop/src-tauri commands (existing plane_*)
        │
        ▼
tracer-control-plane + fake ACP + temp SQLite
```

**Forbidden:** fabricating PASS via harness-side `plane_*` for session/prompt/approval.

## 3. Product surfaces added (owned)

| Area | Change |
|---|---|
| `tauri.conf.json` | `withGlobalTauri: true` so frontend uses real commands |
| Stable testids | `tracer-app-ready`, projects/session/approval/composer markers |
| Project register | Path form → `tracer_project_register` (automation-friendly) |
| Session create | Fake ACP scenario select → `tracer_session_create.runtime.scenarioId` |
| Multi-session | `tracer_presentation_focus` on open/create |
| Prompt UX | Non-blocking submit so approval/cancel concurrent (deadlock-free) |
| Fail-closed | Backend badge + invoke error region; no silent mock |
| E2E hooks | `TRACER_E2E_READY_MARKER` file; `window.__TRACER_E2E__` skips confirm |

## 4. Harness lifecycle

```text
frontend build → backend build → packaging_test_binary
→ driver_startup → app_launch → readiness
→ product journeys (serial)
→ app_shutdown → driver_shutdown → orphan_verification
→ artifacts/tauri-e2e/<run-id>/ on failure
```

## 5. Selector priority

1. role + accessible name  
2. form label (`htmlFor`)  
3. `data-testid="tracer-…"`  
4. state markers (`data-session-status`, `data-event-type`)  
5. CSS last resort  

Never: generated classes, pixel coords, color-only, React internals.

## 6. Environment hooks

| Variable | Role |
|---|---|
| `TRACER_DATABASE_PATH` | Temp SQLite |
| `TRACER_FAKE_ACP_JS` | Fake ACP script |
| `TRACER_HELI_PROBE_PATH` | Empty dir → Heli unavailable |
| `TRACER_NODE_BIN` | Node for fake ACP |
| `TRACER_E2E_READY_MARKER` | Optional readiness file path |
| `TRACER_E2E_APP_BINARY` | Binary override |
| `TRACER_NATIVE_DRIVER` | msedgedriver path |
| `TRACER_TAURI_DRIVER_PORT` | Default 4444 |

## 7. CI isolation

`pnpm -r test` must **not** run L2 / L3-I / L3-J.  
Root scripts:

```text
pnpm test:tauri-e2e:gui
pnpm test:tauri-e2e:gui -- --journey GJ-03
```

## 8. Related docs

- `W2_2_B_JOURNEY_SPEC.md`
- `W2_2_B_TEST_MATRIX.md`
- `W2_2_B_COMPLETION_REPORT.md`
- `docs/validation/tauri/FULL_GUI_JOURNEY_RESULTS.md`
