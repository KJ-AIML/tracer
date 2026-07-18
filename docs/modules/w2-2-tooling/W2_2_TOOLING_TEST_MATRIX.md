# W2.2-T Test Matrix — WebView Tooling

**Task:** `tracer-w2-webview-tooling`  
**Honest rule:** no false PASS when tooling blocks; L3-J never claimed.

## 1. Level × suite matrix

| Level | Suite | Command | CI class | Gate 2.2.2 claim |
|---|---|---|---|---|
| Doctor | Env discovery | `pnpm test:tauri-e2e:doctor` | all | **READY** |
| Setup plan | Inventory | `pnpm test:tauri-e2e:setup` | manual | plan OK |
| Setup apply | Provision | `TRACER_TAURI_E2E_SETUP=1` + setup `--apply` | manual opt-in | drivers installed |
| L0 | Invoke policy | `pnpm test:tauri-e2e` / `--policy-only` | standard_ci | referenced |
| L1 | Boundary | `pnpm test:tauri-e2e` / `--boundary-only` | standard_ci | referenced |
| L2 | App launch smoke | `pnpm test:tauri-e2e:l2` | windows_gui / platform_gated | **PASS** |
| L3-I | Driver infra | `pnpm test:tauri-e2e:l3i` | windows_gui / platform_gated | **PASS** |
| L3-J | Product journey | — | — | **NOT_STARTED** |

## 2. Doctor component matrix

| Component | Pass condition | Failure codes |
|---|---|---|
| TAURI_DRIVER | binary found (PATH / cargo bin / project bin) | `TAURI_DRIVER_NOT_FOUND` |
| EDGE_BROWSER | Edge present + major known (Windows) | `EDGE_BROWSER_NOT_FOUND`, `EDGE_BROWSER_VERSION_UNKNOWN` |
| WEBVIEW2_RUNTIME | registry pv present | `WEBVIEW2_NOT_FOUND` |
| EDGE_DRIVER | major match verified | `EDGE_DRIVER_NOT_FOUND`, `EDGE_DRIVER_VERSION_MISMATCH`, `EDGE_DRIVER_VERSION_UNVERIFIED` |
| APPLICATION_BINARY | cargo artifact present | `APP_BINARY_NOT_FOUND` |
| FRONTEND_DIST | `apps/desktop/dist/index.html` | `FRONTEND_DIST_NOT_FOUND` |
| PORT_AVAILABILITY | bind free on driver port | `PORT_IN_USE`, `PORT_CHECK_FAILED` |
| PROCESS_CLEANUP_CAPABILITY | taskkill/tasklist or kill | `PROCESS_CLEANUP_UNAVAILABLE` |

## 3. Stage matrix (L2 / L3-I)

| Stage id | L2 | L3-I | Notes |
|---|---|---|---|
| `frontend_build` | yes | require dist | Vite build or existing dist |
| `backend_build` | cargo build | require binary | `tracer-desktop` |
| `packaging_test_binary` | resolve exe | resolve exe | `bundle.active=false` intentional |
| `driver_startup` | skip N/A | start tauri-driver | native driver path passed |
| `app_launch` | spawn binary | WebDriver new session | |
| `readiness` | process ± main window | title + readyState poll | |
| `smoke` | launch checklist | root + Tauri IPC surface | not product journey |
| `app_shutdown` | tree kill | delete session | |
| `driver_shutdown` | skip | stop driver | |
| `orphan_verification` | required | required | fail if leftovers |

## 4. L3-I smoke checklist

| # | Check | Evidence |
|---|---|---|
| 1 | Frontend dist | file present |
| 2 | App binary | file present |
| 3 | Driver ready | HTTP `/status` |
| 4 | Session | WebDriver session id |
| 5 | Title | `"Tracer"` |
| 6 | Root | `#root` or `body` |
| 7 | Tauri IPC | `__TAURI_INTERNALS__` and/or `__TAURI__.core.invoke` |
| 8 | readyState | `complete` (non-product property) |
| 9 | Session delete | HTTP 200 |
| 10 | Driver stop + no orphans | pid dead; orphan list empty |

## 5. Result classification

| Result | When |
|---|---|
| `PASS` | All required stages pass |
| `PARTIAL` | Driver/root OK but IPC surface incomplete |
| `BLOCKED_BY_TOOLING` | Missing driver/binary/build tools |
| `BLOCKED_BY_WEBVIEW` | WebView runtime missing |
| `UNSUPPORTED_PLATFORM` | e.g. external driver on macOS |
| `FAIL` | Unexpected error after tools available |

## 6. CI isolation assertions

| Assertion | Expected |
|---|---|
| `pnpm -r test` runs `@tracer/tauri-e2e` `test` script | `node ./run.mjs` → L0+L1 only |
| L3-I not default recursive | true — requires `test:l3i` / root `test:tauri-e2e:l3i` |
| Driver binaries gitignored | `.cache/`, `bin/`, `*.exe` under tools/tauri-driver |
| Apply not automatic in CI | requires `TRACER_TAURI_E2E_SETUP` or `--apply` |

## 7. Host evidence template (Gate 2.2.2)

Record at completion:

```text
Doctor: READY
L2: PASS
L3-I: PASS
L3-J: NOT_STARTED
```

Commands used:

```powershell
node tools/tauri-driver/setup.mjs --apply   # with TRACER_TAURI_E2E_SETUP=1
node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/l2-smoke.mjs --skip-build
node tools/tauri-e2e/l3i-infra.mjs
```
