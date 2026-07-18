# Windows GUI Runner Contract (W2.3-C)

**CI class:** `windows_gui_runner`  
**Also valid as:** `platform_gated_ci` | `manual_local`  
**Not:** standard CI, `pnpm -r test`, live-provider runners  
**Status:** Complete for W2.3-C gate

## Purpose

Contract for hosts that execute real Tauri WebView L2 / L3-I / L3-J with **fake ACP only**.

This document is the preferred delivery when platform-gated CI YAML cannot be committed safely.

## Host requirements

| Component | Requirement |
|---|---|
| OS | Windows 10/11 x64 with desktop session (GUI) |
| WebView2 | Evergreen runtime installed |
| Edge | Installed; major version known |
| msedgedriver | `major(msedgedriver) == major(Edge)` (project cache or PATH) |
| tauri-driver | `cargo install tauri-driver --locked` (or project resolve) |
| Node | ≥ 20 |
| pnpm | workspace package manager |
| Rust | `rustc` + `cargo` for building `tracer-desktop` |
| Fake ACP | `tools/fake-acp-runtime/bin/fake-acp-runtime.js` |

## Environment isolation

| Variable | Rule |
|---|---|
| `TRACER_DATABASE_PATH` | Temp file SQLite only |
| `TRACER_FAKE_ACP_JS` | Fake runtime path |
| `TRACER_HELI_PROBE_PATH` | Empty dir for non-fatal Heli |
| `TRACER_E2E_READY_MARKER` | Optional readiness file |
| `TRACER_TAURI_DRIVER_PORT` | Preferred port; free-port allocator avoids collisions |
| `TRACER_NATIVE_DRIVER` | Optional msedgedriver override |
| `TRACER_E2E_KEEP_TEMP` | Keep workdirs on demand |
| `TRACER_E2E_INJECT` | Harness inject only (`none` default) |
| `TRACER_E2E_REPEAT_RUNS` | Consecutive suite count for `repeat-gui` (default 5) |

**Forbidden on this runner:** live Grok credentials, provider keys, user profile DB, network product paths.

## Preflight

```powershell
pnpm test:tauri-e2e:doctor
# If Edge/driver mismatch:
pnpm test:tauri-e2e:setup -- --apply
# or
node tools/tauri-e2e/doctor.mjs --apply
```

Doctor must be `READY` (or `BUILD_REQUIRED` then build) before claiming L3-J PASS.

Edge auto-update resilience: when major(Edge) drifts from msedgedriver, doctor reports `INCOMPATIBLE_VERSION` + remediation. Never silent PASS.

## Execution

```powershell
# Reliability unit + inject (no GUI)
pnpm test:tauri-e2e:reliability
pnpm test:tauri-e2e:inject-fail

# One-shot full product journeys
pnpm test:tauri-e2e:gui

# Reliability batch: 5+ consecutive first-attempt fresh-env suites
pnpm test:tauri-e2e:repeat-gui -- --runs 5 --skip-build
```

Each L3-J / repeat run must:

1. Allocate a free driver port (avoid collisions; never reuse busy)  
2. Use unique temp workDir + SQLite + app-data isolation  
3. Fresh fake ACP path; no prior owned process required  
4. Record first-attempt results only (`retries=0`)  
5. Collect timing: driverStartupMs, appReadinessMs, suiteMs, shutdownMs  
6. Verify orphans=0 after teardown; sanitize artifacts  
7. Clean temp on PASS (keep on FAIL for diagnosis)

## Result honesty

| Result | Meaning |
|---|---|
| PASS | Journeys + orphans + isolation OK |
| FAIL | Product assert or harness hard fail |
| BLOCKED_BY_TOOLING | Drivers/binary missing — **not** product green |
| BLOCKED_BY_WEBVIEW | WebView unavailable |
| PARTIAL | Non-fatal incomplete probe (must not be silent PASS for product) |

Never map `BLOCKED_*` to CI green product gate.  
Never use unlimited retries to mask product assertion failures.

## Process safety

- Owned process spawn + tree kill  
- Orphan verify for `tracer-desktop`, `tauri-driver`, `msedgedriver`  
- Exit hooks must not leave app/driver running  
- Shutdown uses state-based waits (`waitUntil` process exit), not fixed sleep alone  

## Artifacts

`artifacts/tauri-e2e/<run-id>/` (gitignored): page dump, probe, report — **sanitized**.  
Audit helper fails closed if unsanitized secrets remain.

## Failure injection (harness-only)

`TRACER_E2E_INJECT` / `pnpm test:tauri-e2e:inject-fail` covers deterministic C6 modes  
(app launch, driver startup, root marker, fake runtime, SQLite, GUI assert, shutdown timeout, stale Edge).  
See `docs/modules/w2-3-c/W2_3_C_TEST_MATRIX.md`.

## Standard CI isolation

| Command | Includes L3-J? |
|---|---|
| `pnpm -r test` | **No** |
| `cargo test --workspace` | **No** |
| `pnpm test:tauri-e2e:gui` / `repeat-gui` | Explicit opt-in only |

## Packaging / live GUI

Out of scope for this contract (W2.3-A packaging, W2.3-B live GUI).
