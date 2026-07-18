# WebView Driver Readiness (Gate 2.2.2)

**Authority:** W2.2-T WebView tooling enablement on branch `agent/tracer-w2-webview-tooling`  
**Harness:** `tools/tauri-e2e/` · provisioning `tools/tauri-driver/`  
**Date:** 2026-07-18 · **Host:** grok-build (Windows)

## Decision snapshot

| Surface | Result |
|---|---|
| Doctor | **READY** |
| L2 packaged launch smoke | **PASS** |
| L3-I WebView driver infrastructure | **PASS** |
| L3-J full GUI product journey | **NOT_STARTED** |

## Level model

| Level | Meaning | Gate 2.2.2 status |
|---|---|---|
| L0 | Frontend invoke policy | referenced (standard CI) |
| L1 | Desktop boundary journey | referenced (standard CI) |
| L2 | Built application process launch smoke | **PASS** |
| L3-I | WebView driver infrastructure | **PASS** |
| L3-J | Full GUI product journey | **NOT_STARTED** |

## Host inventory (authoring evidence)

| Item | Observed |
|---|---|
| OS | Windows 10.0.26200 / x64 |
| Rust | 1.96.0 |
| Node | v24.16.0 |
| pnpm | 9.15.0 |
| WebView2 | 150.0.4078.65 |
| Edge | 150.0.4078.65 |
| tauri-driver | present (cargo install + project bin copy) |
| msedgedriver | 150.0.4078.65 exact match (project cache) |
| Compatibility | `EDGE_DRIVER_COMPATIBLE` — rule `major(msedgedriver)==major(Edge)` |
| frontend dist | `apps/desktop/dist` after Vite build |
| app binary | `target/debug/tracer-desktop.exe` |
| port 4444 | available at preflight |
| process cleanup | taskkill /T + tasklist |

### Doctor components

```text
TAURI_DRIVER: OK
EDGE_BROWSER: OK version=150.0.4078.65
WEBVIEW2_RUNTIME: OK version=150.0.4078.65
EDGE_DRIVER: OK version=150.0.4078.65 (compatible)
APPLICATION_BINARY: OK
FRONTEND_DIST: OK
PORT_AVAILABILITY: OK 127.0.0.1:4444
PROCESS_CLEANUP_CAPABILITY: OK
Doctor classification: READY
L3-I attemptable=true
L3-J NOT_STARTED
```

Optional advisory: `tauri_cli` missing (cargo-only path still valid).

### L2 smoke

```text
node tools/tauri-e2e/l2-smoke.mjs --skip-build
→ L2 result: PASS
  frontend_build pass
  backend_build pass
  packaging_test_binary pass
  driver_startup skip (N/A for L2)
  app_launch pass
  readiness pass (process alive + main window)
  smoke pass
  app_shutdown pass
  orphan_verification pass
```

### L3-I

```text
node tools/tauri-e2e/l3i-infra.mjs
→ L3-I result: PASS
  driver_startup pass (tauri-driver + msedgedriver)
  app_launch pass (WebDriver session)
  readiness pass (title=Tracer, readyState=complete)
  smoke pass (root + __TAURI_INTERNALS__ present)
  app_shutdown pass
  driver_shutdown pass
  orphan_verification pass
```

IPC note: public `__TAURI__` not required for L3-I; Tauri 2 injects `__TAURI_INTERNALS__` without `withGlobalTauri`. Product journey work (L3-J) may still need public global or module invoke — **not claimed here**.

## Setup path used

```powershell
$env:TRACER_TAURI_E2E_SETUP = "1"
node tools/tauri-driver/setup.mjs --apply
# cargo install tauri-driver --locked
# download msedgedriver 150.x into tools/tauri-driver/.cache/ (gitignored)
```

## CI posture

| Class | Command |
|---|---|
| standard_ci | `pnpm test:tauri-e2e` (L0+L1) |
| windows_gui_runner | `pnpm test:tauri-e2e:l2` · `pnpm test:tauri-e2e:l3i` |
| manual_local | doctor + selective levels |
| L3-I isolation | **not** in `pnpm -r test` |

## Related docs

- `docs/modules/w2-2-tooling/W2_2_TOOLING_ARCHITECTURE.md`
- `docs/modules/w2-2-tooling/W2_2_TOOLING_SETUP.md`
- `docs/modules/w2-2-tooling/W2_2_TOOLING_TEST_MATRIX.md`
- `docs/integration/WAVE_2_2_2_TOOLING_REPORT.md`
- `docs/integration/W2_2_B_LAUNCH_AUTHORIZATION.md`
