# Tauri E2E infrastructure (W2.2-A + Gate 2.1)

**Task:** `tracer-w2-tauri-e2e-infrastructure`  
**Classification:** infrastructure + packaged-app smoke + WebView **driver infrastructure**  
**Not claimed:** L3-J full GUI product journey (future W2.2-B)

## Levels (do not collapse)

| Level | Meaning | Command |
|---|---|---|
| **L0** | Frontend invoke/mock policy | `node tools/tauri-e2e/run.mjs --policy-only` |
| **L1** | Backend command-boundary (plane_* == Tauri handlers) | `node tools/tauri-e2e/run.mjs --boundary-only` |
| **L2** | Built application launch smoke | `node tools/tauri-e2e/l2-smoke.mjs` |
| **L3-I** | WebView driver infrastructure interaction | `node tools/tauri-e2e/l3i-infra.mjs` |
| **L3-J** | Full GUI product journey | **DEFERRED** — do not claim |

## Doctor

```powershell
node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/doctor.mjs --json
pnpm --filter @tracer/tauri-e2e doctor
```

**Root script name for integrator** (not always present in root `package.json`):

```text
pnpm test:tauri-e2e:doctor   →  node tools/tauri-e2e/doctor.mjs
```

Classifications: `READY | MISSING_TOOL | INCOMPATIBLE_VERSION | WEBVIEW_UNAVAILABLE | DRIVER_UNAVAILABLE | BUILD_REQUIRED | UNSUPPORTED_PLATFORM`

See `docs/validation/tauri/TAURI_E2E_DOCTOR.md`.

## Stages (L2 / L3-I)

```text
frontend build → Tauri backend build → packaging/test binary → driver startup
→ app launch → readiness → smoke → app shutdown → driver shutdown → orphan verification
```

Each stage has a distinct status: `pass | fail | skip | partial | blocked_tooling | blocked_webview | unsupported`.

## Standard CI class (L0+L1)

- network: **no**
- credentials: **no**
- live Grok: **no**
- provider: **no**
- fake ACP: **yes**
- temp file SQLite: **yes**

## L2 / L3-I CI class

- `windows_gui_runner` | `platform_gated_ci` | `manual_local`
- Never emit false `PASS` when tooling blocks → use `BLOCKED_BY_TOOLING` / `BLOCKED_BY_WEBVIEW`

## Driver safety

- Process ownership via `lib/process.mjs`
- stdout/stderr capture under unique temp dirs
- Timeouts + process-tree kill (`taskkill /T` on Windows)
- Orphan detection for `tracer-desktop`, `tauri-driver`, `msedgedriver`
- Exit hooks never leave the app/driver running

## E2E env hooks

| Variable | Purpose |
|---|---|
| `TRACER_DATABASE_PATH` | File SQLite path |
| `TRACER_FAKE_ACP_JS` | Path to fake ACP script |
| `TRACER_HELI_PROBE_PATH` | Heli probe directory |
| `TRACER_NODE_BIN` | Node for fake ACP |
| `TRACER_E2E_PROFILE` | `debug` \| `release` |
| `TRACER_E2E_APP_BINARY` | Override app binary path |
| `TRACER_TAURI_DRIVER_PORT` | Default `4444` |
| `TRACER_NATIVE_DRIVER` | Path to msedgedriver / WebKitWebDriver |

## Layout

```text
tools/tauri-e2e/
  run.mjs           orchestrator (L0/L1 + flags)
  doctor.mjs        environment discovery
  l2-smoke.mjs      L2 launch smoke
  l3i-infra.mjs     L3-I driver infrastructure
  lib/
    classify.mjs
    discover.mjs
    process.mjs
    stages.mjs
    webdriver.mjs
tools/tauri-driver/ install + start helpers
```

## Related docs

- `docs/modules/w2-2-a/W2_2_A_ARCHITECTURE.md`
- `docs/modules/w2-2-a/W2_2_A_ENVIRONMENT_MATRIX.md`
- `docs/modules/w2-2-a/W2_2_A_TEST_MATRIX.md`
- `docs/modules/w2-2-a/W2_2_A_COMPLETION_REPORT.md`
- `docs/modules/w2-b/W2_B_E2E_ARCHITECTURE.md` (Gate 2.1 L0/L1)
