# Windows Release Results (W2.3-A)

**Task:** `tracer-w2-windows-packaging`  
**Work item:** W2.3-A  
**Host:** grok-build (Windows)  
**Date:** 2026-07-18  
**Branch:** `agent/tracer-w2-windows-packaging`  
**Base SHA:** `8f3b3cb568483fde065dae77d341b38e597b23b2`

## Packaging decision

| Output | Selected | Host evidence |
|---|---|---|
| **NSIS** (`Tracer_0.1.0_x64-setup.exe`) | **Primary** | Produced |
| **Portable** (`tracer-desktop.exe`) | **Secondary** | Produced |
| **MSI** | Not selected for this RC | Not built |

## Environment

| Item | Observed |
|---|---|
| OS | Windows win32/x64 |
| Node | v24.16.0 |
| pnpm | 9.15.0 |
| Rust / cargo | 1.96.0 |
| Tauri CLI | `@tauri-apps/cli@2` via `npx` |
| Fake ACP | `tools/fake-acp-runtime/bin/fake-acp-runtime.js` present |
| Signing material env | none |

## Commands executed

```text
node tools/release/identity-check.mjs
→ RESULT: PASS

pnpm release:windows
→ result: PASS
→ signing: UNSIGNED_DEVELOPMENT_RC
→ portable: target/release/tracer-desktop.exe
→ nsis:     target/release/bundle/nsis/Tracer_0.1.0_x64-setup.exe
→ msi:      (not selected)

node tools/release/validate-windows.mjs --skip-build
→ RESULT: PASS
→ signing: UNSIGNED_DEVELOPMENT_RC
→ RC-01..RC-06 all PASS

node tools/release/classify-signing.mjs
→ class: UNSIGNED_DEVELOPMENT_RC
→ portable + NSIS Status=NotSigned
```

## Artifacts (not committed)

| Artifact | Path | Size (bytes) |
|---|---|---|
| Portable PE | `target/release/tracer-desktop.exe` | 17165312 |
| NSIS setup | `target/release/bundle/nsis/Tracer_0.1.0_x64-setup.exe` | 4116266 |
| Staged copies | `target/release-rc/windows/*.exe` | (mirrors above) |
| Manifest | `target/release-rc/windows/manifest.json` | |
| RC summary | `target/release-rc/windows/rc-summary.json` | |
| RC validation | `target/release-rc/windows/rc-validation.json` | |

## Identity

| Field | Value | Check |
|---|---|---|
| productName | Tracer | PASS |
| identifier | dev.tracer.desktop | PASS |
| mainBinaryName | tracer-desktop | PASS |
| version (tauri / package.json / Cargo) | 0.1.0 | PASS (aligned) |
| icons | ico + png set | PASS |
| bundle.active | true | PASS |
| bundle.targets | ["nsis"] | PASS |
| certificateThumbprint | null | PASS (no fake thumbprint) |
| App data (Windows) | `%LOCALAPPDATA%\dev.tracer.desktop\` | documented |

## Signing classification

```text
class: UNSIGNED_DEVELOPMENT_RC
```

- Authenticode on portable: `NotSigned`
- Authenticode on NSIS setup: `NotSigned`
- No cert env keys present
- Allowed overall **PASS** when classified (task contract)
- **No SIGNED claim**

## Scenario results RC-01..RC-06

| ID | Name | Result | Mode / notes |
|---|---|---|---|
| RC-01 | Clean install | **PASS** | `nsis_silent` `/S /D=<tmpdir>` → `tracer-desktop.exe` found |
| RC-02 | Fake-runtime smoke | **PASS** | process alive + ready marker + clean shutdown |
| RC-03 | Upgrade | **PASS** | `no_prior_fixture` — honest non-claim (no `TRACER_RC_PRIOR_INSTALL`) |
| RC-04 | Uninstall | **PASS** | `nsis_silent_uninstall` via `uninstall.exe /S` |
| RC-05 | Reinstall | **PASS** | second silent NSIS install → exe found |
| RC-06 | Failed launch diagnostics | **PASS** | missing PE spawn path yields ENOENT / diagnostic signal |

### Overall

```text
RESULT: PASS
signing: UNSIGNED_DEVELOPMENT_RC
```

## Honesty notes

1. **RC-03** did not prove multi-version upgrade against an older released RC; fixture unset → documented `no_prior_fixture` PASS.
2. **Signing** is development-unsigned; production cert + CI secrets are follow-on work.
3. **MSI** intentionally not built for this RC.
4. RC-02 is **process-level** smoke with fake ACP — not a full WebView product journey (W2.3-B).
5. `tools/tauri-e2e` core reliability remains owned by W2.3-C.

## Non-coverage

| Area | Status |
|---|---|
| L3-J full GUI product journey | Not owned (W2.3-B) |
| tauri-e2e reliability harness core | Not owned (W2.3-C) |
| Live Grok / network credentials | Out of scope |
| Production Authenticode | Out of scope |
| macOS / Linux installers | Out of scope |
