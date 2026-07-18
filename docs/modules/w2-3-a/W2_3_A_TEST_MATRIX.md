# W2.3-A Test Matrix — Windows RC

**Task:** `tracer-w2-windows-packaging`  
**Work item:** W2.3-A  
**Date:** 2026-07-18

## 1. Packaging decision (under test)

| Output | In matrix? |
|---|---|
| NSIS installer | Yes (primary when produced) |
| Portable `tracer-desktop.exe` | Yes (secondary / fallback) |
| MSI | No (not selected for this RC) |

## 2. Preflight checks

| ID | Check | Command / method | Pass criteria |
|---|---|---|---|
| PF-01 | Identity consistency | `node tools/release/identity-check.mjs` | productName, identifier, mainBinaryName, triple version match, icons present, bundle.active |
| PF-02 | Platform gate | validate / release scripts | Non-Windows → `UNSUPPORTED_PLATFORM` (exit 3), not false PASS |
| PF-03 | Signing classification | `node tools/release/classify-signing.mjs` | Class ∈ {SIGNED, UNSIGNED_DEVELOPMENT_RC, BLOCKED}; never invent SIGNED |
| PF-04 | Fake ACP present | path probe | `tools/fake-acp-runtime/bin/fake-acp-runtime.js` exists for RC-02 |

## 3. Scenario matrix RC-01…RC-06

| ID | Name | Mode when NSIS present | Mode when portable only | Pass criteria | Fail criteria |
|---|---|---|---|---|---|
| **RC-01** | Clean install | NSIS silent `/S /D=<tmpdir>` | Portable binary present (honest substitute) | Installer exits 0 and installed exe found **or** portable present | No artifact / install fails without exe |
| **RC-02** | Fake-runtime smoke | Launch installed or portable exe | Launch portable | Process stays alive for smoke window; clean kill; no orphan PID | Early exit, spawn fail, orphan remains |
| **RC-03** | Upgrade | Over `TRACER_RC_PRIOR_INSTALL` if set | N/A semantics | Prior fixture upgraded **or** honest `no_prior_fixture` PASS | Prior fixture set but upgrade fails |
| **RC-04** | Uninstall | NSIS `uninstall.exe /S` | Portable delete procedure documented | Uninstaller exit 0 **or** portable mode documented PASS | NSIS mode without uninstaller and uninstall attempted |
| **RC-05** | Reinstall | Second silent install | Portable re-launch | Second install/relaunch succeeds | Reinstall fails |
| **RC-06** | Failed launch diagnostics | Missing PE / spawn error path | Same | Diagnostics capture spawn error or non-zero | No diagnostic signal |

### RC-02 environment (smoke)

```text
TRACER_FAKE_ACP_JS=<repo>/tools/fake-acp-runtime/bin/fake-acp-runtime.js
TRACER_DATABASE_PATH=<tmpdir>/tracer-rc.db
TRACER_E2E_READY_MARKER=<tmpdir>/ready.txt
```

Ready marker is **optional** for PASS (cold WebView may exceed short window); process liveness + clean teardown are required.

### RC-03 honesty rule

If `TRACER_RC_PRIOR_INSTALL` is unset or missing → scenario result **PASS** with  
`mode=no_prior_fixture` and explicit non-claim of multi-version upgrade proof.

## 4. Signing × overall result

| Signing class | RC scenarios all PASS | Allowed overall? |
|---|---|---|
| `SIGNED` | Yes | **PASS** |
| `UNSIGNED_DEVELOPMENT_RC` | Yes | **PASS** (classified) |
| `BLOCKED` | Yes | **BLOCKED** (not product PASS) |
| any | Any FAIL | **FAIL** |

## 5. Build matrix

| Build path | Command | Expected artifacts |
|---|---|---|
| Full RC | `pnpm release:windows` | NSIS setup + portable (when bundler succeeds) |
| Portable only | `node tools/release/windows-rc.mjs --no-bundle` | `target/release/tracer-desktop.exe` |
| Skip build | `… --skip-build` | Uses existing tree |

`PARTIAL` overall packaging result is allowed when only portable is produced (NSIS tooling unavailable) — still must classify signing.

## 6. Non-coverage (explicit)

| Area | Status |
|---|---|
| L3-J full GUI product journey | Not owned (W2.3-B) |
| Doctor / L2 / L3-I harness core | Referenced only (W2.2 / W2.3-C) |
| Live Grok / network credentials | Out of scope |
| macOS DMG / Linux AppImage | Out of scope |
| Production cert issuance | Out of scope |

## 7. Execution entry

```bash
# From repo root on Windows
node tools/release/identity-check.mjs
pnpm release:windows
pnpm test:release:windows
```

Evidence JSON: `target/release-rc/windows/rc-validation.json`  
Host write-up: `docs/validation/release/WINDOWS_RELEASE_RESULTS.md`
