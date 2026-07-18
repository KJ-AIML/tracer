# W2.2-A Test Matrix

**Task:** `tracer-w2-tauri-e2e-infrastructure`  
**Honest rule:** no false PASS when tooling blocks.

## 1. Level × suite matrix

| Level | Suite | Command | CI class | Default claim |
|---|---|---|---|---|
| L0 | Invoke policy | `node tools/tauri-e2e/run.mjs --policy-only` | standard_ci | Pass when vitest green (Gate 2.1) |
| L1 | Boundary journey | `node tools/tauri-e2e/run.mjs --boundary-only` | standard_ci | Pass when cargo test green (Gate 2.1) |
| Doctor | Env discovery | `node tools/tauri-e2e/doctor.mjs` | all | Classification report |
| L2 | App launch smoke | `node tools/tauri-e2e/l2-smoke.mjs` | windows_gui / platform_gated / manual | PASS/PARTIAL or BLOCKED_* |
| L3-I | Driver infra | `node tools/tauri-e2e/l3i-infra.mjs` | windows_gui / platform_gated / manual | PASS/PARTIAL or BLOCKED_* |
| L3-J | Product journey | — | — | **DEFERRED — not claimed** |

## 2. Stage matrix (L2 / L3-I)

| Stage id | L2 | L3-I | Failure classes |
|---|---|---|---|
| `frontend_build` | build or use dist | require dist | FAIL / BLOCKED_BY_TOOLING |
| `backend_build` | cargo build -p tracer-desktop | require binary | FAIL / BLOCKED_BY_TOOLING |
| `packaging_test_binary` | resolve exe artifact | resolve exe | FAIL / BLOCKED_BY_TOOLING |
| `driver_startup` | skip (N/A) | start tauri-driver | BLOCKED_BY_TOOLING / FAIL |
| `app_launch` | spawn binary | WebDriver new session | FAIL / BLOCKED_* |
| `readiness` | process ± main window | session + title | PARTIAL / FAIL |
| `smoke` | launch checklist | DOM/Tauri probes | PARTIAL / PASS / FAIL |
| `app_shutdown` | tree kill | delete session | FAIL |
| `driver_shutdown` | skip | stop driver | FAIL |
| `orphan_verification` | required | required | FAIL / PARTIAL if reaped |

## 3. Minimum smoke checklist (1–10)

| # | Check | L2 | L3-I |
|---|---|---|---|
| 1 | Build | yes | prerequisite |
| 2 | Launch | process spawn | via driver session |
| 3 | WebView init | main window best-effort | session ready |
| 4 | Frontend root | n/a without driver → null | `#root` / body execute |
| 5 | Tauri API detect | n/a → null | `__TAURI__.core.invoke` |
| 6 | App info | n/a → null | optional invoke |
| 7 | Initial snapshot | n/a (L1 covers plane) | optional / not required |
| 8 | Clean exit | yes | session delete |
| 9 | Driver exit | n/a | yes |
| 10 | No orphans | yes | yes |

## 4. Result classification

| Result | When |
|---|---|
| `PASS` | All required stages pass for that level |
| `PARTIAL` | Launch/driver OK but some probes incomplete (e.g. no window handle yet) |
| `BLOCKED_BY_TOOLING` | Missing driver, binary, or build tools — **not** a product bug |
| `BLOCKED_BY_WEBVIEW` | WebView runtime missing |
| `UNSUPPORTED_PLATFORM` | e.g. external driver on macOS |
| `FAIL` | Unexpected error after tools were available |

## 5. Assertions ownership

| Assertion family | Owner level |
|---|---|
| Fail-closed invoke / no silent mock | L0 |
| plane_* == Tauri handlers, fake ACP journeys | L1 |
| Real process launch + shutdown hygiene | L2 |
| Driver lifecycle + WebView bridge | L3-I |
| Product click-path (session/prompt/approval UI) | L3-J deferred |

## 6. Run recipes

```powershell
# Standard CI
node tools/tauri-e2e/run.mjs

# Doctor
node tools/tauri-e2e/doctor.mjs --json

# L2 (may build)
node tools/tauri-e2e/l2-smoke.mjs
node tools/tauri-e2e/l2-smoke.mjs --skip-build
node tools/tauri-e2e/l2-smoke.mjs --release

# L3-I
node tools/tauri-e2e/l3i-infra.mjs --json

# Everything (L2/L3-I non-fatal to L0/L1 exit)
node tools/tauri-e2e/run.mjs --all
```

## 7. Integrator root script (document only)

Prefer tools-local. Root alias to register when integrating:

```json
"test:tauri-e2e:doctor": "node tools/tauri-e2e/doctor.mjs"
```

Existing Gate 2.1 root script: `test:tauri-e2e` → `node tools/tauri-e2e/run.mjs`
