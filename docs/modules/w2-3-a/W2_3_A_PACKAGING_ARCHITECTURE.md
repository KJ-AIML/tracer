# W2.3-A Packaging Architecture — Windows RC

**Task:** `tracer-w2-windows-packaging`  
**Work item:** W2.3-A  
**Branch:** `agent/tracer-w2-windows-packaging`  
**Owned paths:**  
`apps/desktop/src-tauri/tauri.conf.*`, `icons/`, `capabilities/`,  
`tools/release/`, `tests/release/windows/`,  
`docs/modules/w2-3-a/`, `docs/validation/release/`  
(+ minimal root `package.json` scripts)

**Forbidden:** control-plane/runtime/storage redesign; product UI redesign; live GUI harness (W2.3-B); reliability harness ownership of `tools/tauri-e2e` core (W2.3-C); IDE/ALMS/plugins

## 1. Purpose

Produce a **Windows release candidate** packaging surface that is:

1. **Identity-stable** — Tracer name, package id, exe name, semver, icons, app-data paths  
2. **Honest about signing** — never claim Authenticode without evidence  
3. **Command-driven** — `pnpm release:windows`  
4. **Validatable** — RC-01…RC-06 without owning full GUI product harness  

## 2. Packaging decision

| Output | Selected | Role |
|---|---|---|
| **NSIS** (Tauri `nsis` target) | **Yes — primary** | User-facing installer (`*-setup.exe`), per-user `currentUser` mode |
| **Portable binary** | **Yes — secondary** | `tracer-desktop.exe` from `target/release/` after release build; no installer registry footprint |
| **MSI** (WiX) | **No — not this RC** | Deferred: WiX toolchain + Windows VBScript optional feature; re-enable by extending `bundle.targets` |

**Rationale**

- NSIS is first-class in Tauri 2, downloads bundler tools into `target/.tauri/` when `useLocalToolsDir: true`, and supports silent install for automation.  
- Portable binary covers smoke/CI hosts that need a runnable artifact without installer elevation.  
- MSI is optional product surface — not required for RC PASS.  
- Tauri has **no** separate bundle type named `portable`; portable = release PE + docs.

```text
                    ┌─────────────────────────────┐
  pnpm release:windows │ tools/release/windows-rc.mjs │
                    └──────────────┬──────────────┘
                                   │
              identity check ──────┤
                                   ▼
                    ┌─────────────────────────────┐
                    │  tauri build (apps/desktop) │
                    │  targets: ["nsis"]          │
                    └──────────────┬──────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
   target/release/      target/release/bundle/nsis/   (msi not built)
   tracer-desktop.exe   Tracer_*_x64-setup.exe
        portable               primary
              │                    │
              └──────────┬─────────┘
                         ▼
              target/release-rc/windows/
                manifest.json + rc-summary.json
                         │
                         ▼
              signing classify → UNSIGNED_DEVELOPMENT_RC | SIGNED | BLOCKED
```

## 3. Identity model

| Field | Canonical value | Sources |
|---|---|---|
| Product display name | `Tracer` | `tauri.conf.json` `productName`, window title |
| Package id (reverse DNS) | `dev.tracer.desktop` | `tauri.conf.json` `identifier` |
| Stable exe stem | `tracer-desktop` | `mainBinaryName` + Cargo package name |
| Windows PE name | `tracer-desktop.exe` | cargo/tauri output (L2/L3 harness compatible) |
| Semver | `0.1.0` | tauri.conf / `apps/desktop/package.json` / `src-tauri/Cargo.toml` |
| Publisher | `Tracer` | bundle.publisher |
| Icons | ico + png set | `apps/desktop/src-tauri/icons/` |
| App data (Windows) | `%LOCALAPPDATA%\dev.tracer.desktop\` | Tauri identifier → LocalAppData |
| Storage relative (when wired) | `tracer\tracer.db` under platform app-data root | `tracer-storage` path helpers (not redesigned here) |

**Semver rule:** all three version sources must match; `tools/release/identity-check.mjs` fails closed on drift.

**Exe name rule:** keep `tracer-desktop` (not renamed to `Tracer.exe`) so existing `tools/tauri-e2e` discovery stays green without W2.3-C ownership changes.

## 4. Tauri configuration posture

File: `apps/desktop/src-tauri/tauri.conf.json`

| Key | W2.3-A value | Notes |
|---|---|---|
| `bundle.active` | `true` | Packaging enabled; L2 still uses **cargo** binary path |
| `bundle.targets` | `["nsis"]` | MSI not selected |
| `bundle.useLocalToolsDir` | `true` | NSIS tools cache under `target/.tauri/` |
| `bundle.windows.certificateThumbprint` | `null` | No fake signing |
| `bundle.windows.nsis.installMode` | `currentUser` | No admin required for default install |
| `bundle.windows.webviewInstallMode` | `downloadBootstrapper` | Win10/11 typically already have WebView2 |

Capabilities: `capabilities/default.json` remains minimal (`core:default`, `shell:allow-open`) — packaging does not expand IPC.

## 5. Commands

| Command | Entry |
|---|---|
| `pnpm release:windows` | `node tools/release/windows-rc.mjs` |
| `pnpm test:release:windows` | `node tools/release/validate-windows.mjs` |
| Identity | `node tools/release/identity-check.mjs` |
| Signing class | `node tools/release/classify-signing.mjs` |
| Portable only | `node tools/release/windows-rc.mjs --no-bundle` |
| Skip rebuild | `… --skip-build` |

## 6. Signing model

```text
SIGNED                     — Valid Authenticode on all inspected artifacts
UNSIGNED_DEVELOPMENT_RC    — NotSigned local/dev RC (allowed PASS when classified)
BLOCKED                    — cannot classify, mixed signatures, or cert env without valid sig
```

Classification uses `Get-AuthenticodeSignature` on Windows.  
**No certificate is embedded or claimed by this task.**

## 7. Validation boundary

| Layer | Owner |
|---|---|
| RC packaging + RC-01…06 process/installer checks | **W2.3-A** (`tools/release`) |
| Full WebView GUI product journeys | W2.3-B |
| `tools/tauri-e2e` reliability core | W2.3-C |
| Control plane / storage path product wiring | other waves |

RC-02 smoke launches the **real PE** with `TRACER_FAKE_ACP_JS` + temp `TRACER_DATABASE_PATH` and verifies process liveness + clean teardown — not DOM journey coverage.

## 8. Artifacts (not committed)

```text
target/release/tracer-desktop.exe
target/release/bundle/nsis/*-setup.exe
target/release-rc/windows/manifest.json
target/release-rc/windows/rc-summary.json
target/release-rc/windows/rc-validation.json
```

## 9. Integrator notes

1. Root scripts added: `release:windows`, `test:release:windows` (document for merge).  
2. Enabling `bundle.active: true` does **not** require L2/L3 harness changes (they build via cargo).  
3. CI should gate Windows packaging on Windows runners only.  
4. Production signing is a **follow-on** (cert + CI secrets) — out of W2.3-A scope.  
5. MSI can be added later by setting `"targets": ["nsis", "msi"]` once WiX is available.
