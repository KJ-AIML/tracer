# Tauri E2E Infrastructure Readiness (Gate 2.2.1)

**Authority:** Integrated from W2.2-A into `main` via Gate 2.2.1.  
**Harness root:** `tools/tauri-e2e/` · helpers `tools/tauri-driver/`  
**Date:** 2026-07-18 · **Host:** grok-build (Windows)

## Level model

| Level | Meaning | Gate 2.2.1 status |
|---|---|---|
| L0 | Frontend invoke policy (fail-closed) | **PASS** |
| L1 | Desktop boundary journey (`tracer-desktop` tests) | **PASS** |
| L2 | Built application process launch smoke | **PASS** |
| L3-I | WebView driver infrastructure (session with tauri-driver) | **BLOCKED_BY_TOOLING** |
| L3-J | Full GUI product journey (DOM session/prompt/approval) | **NOT_STARTED** |

## Explicit decisions (do not reclassify)

| Surface | Decision | Rationale |
|---|---|---|
| Doctor | **DRIVER_UNAVAILABLE** | WebView2 OK; `tauri-driver` and `msedgedriver` missing on PATH |
| L2 | **PASS** | Binary spawn, main window, clean shutdown, orphan_verification |
| L3-I | **BLOCKED_BY_TOOLING** | Harness ready; tools missing — **not** product FAIL or PASS |
| L3-J | **NOT_STARTED** | Future W2.2-B; no journey authored in this gate |

**Never convert `DRIVER_UNAVAILABLE` or `BLOCKED_BY_TOOLING` into product PASS or product FAIL.**

## Commands

```powershell
# Root aliases
pnpm test:tauri-e2e              # L0 + L1 standard CI
pnpm test:tauri-e2e:doctor       # environment doctor (advisory exit 0)

# Platform-gated
node tools/tauri-e2e/l2-smoke.mjs
node tools/tauri-e2e/l2-smoke.mjs --skip-build
node tools/tauri-e2e/l3i-infra.mjs
```

## Host evidence (this gate)

| Item | Observed |
|---|---|
| OS | Windows 10.0.26200 / x64 |
| Rust | 1.96.0 |
| Node | v24.16.0 |
| pnpm | 9.15.0 |
| WebView2 | 150.0.4078.65 |
| tauri-driver | **missing** |
| msedgedriver | **missing** |
| tauri-cli | optional missing |
| frontend dist | present after `pnpm -r build` |
| app binary | `target/debug/tracer-desktop.exe` |

### Doctor (excerpt)

```text
Doctor classification: DRIVER_UNAVAILABLE
L0/L1 attemptable=true
L2 attemptable=true (binary present)
L3-I attemptable=false — blocked until tauri-driver + native WebDriver
L3-J DEFERRED / NOT_STARTED
setup: cargo install tauri-driver --locked (+ msedgedriver on Windows)
```

### L2 smoke (excerpt)

```text
L2 result: PASS
  frontend_build pass
  backend_build pass
  packaging_test_binary pass (cargo artifact; bundle.active=false)
  app_launch pass
  readiness pass (process alive + main window)
  smoke pass (DOM deferred to L3-I)
  app_shutdown pass
  orphan_verification pass
```

### L3-I (excerpt)

```text
L3-I result: BLOCKED_BY_TOOLING
  driver_startup skip — tauri-driver not on PATH
  app_launch..smoke skip — blocked: no driver
  orphan_verification pass — no driver processes started
```

## Process ownership / cleanup audit

| Stage | Owner | Gate result |
|---|---|---|
| App spawn | L2 / L3-I runners under temp workDir | L2 owns pid |
| Tree kill on exit | `tools/tauri-e2e/lib/process.mjs` | L2 shutdown OK |
| Orphan reap | `orphan_verification` stage | PASS (L2); L3-I N/A (no start) |
| Driver lifecycle | L3-I only | Not started (blocked) |

## Config posture

| Item | Status |
|---|---|
| `apps/desktop/src-tauri/tauri.conf.json` `bundle.active` | false (intentional; L2 uses cargo binary) |
| Capabilities expansion for external driver | not required for infrastructure |
| Standard CI | L0+L1 only (`pnpm test:tauri-e2e`) |
| Auto-install browser drivers | **forbidden** by gate policy |

## Setup for future L3-I green

```powershell
cargo install tauri-driver --locked
# Install msedgedriver matching WebView2/Edge major version; place on PATH
# or set TRACER_NATIVE_DRIVER
node tools/tauri-e2e/doctor.mjs
node tools/tauri-e2e/l3i-infra.mjs
```

## Related docs

- `docs/modules/w2-2-a/W2_2_A_ARCHITECTURE.md`
- `docs/modules/w2-2-a/W2_2_A_ENVIRONMENT_MATRIX.md`
- `docs/validation/tauri/TAURI_E2E_DOCTOR.md`
- `docs/integration/W2_2_B_ENTRY_CRITERIA.md`
