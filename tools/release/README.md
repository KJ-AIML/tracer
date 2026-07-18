# Tracer Windows Release (W2.3-A)

Windows **release candidate** packaging and validation for Tracer desktop.

## Packaging decision

| Output | Status | Notes |
|---|---|---|
| **NSIS** (`*-setup.exe`) | **Primary RC** | `bundle.targets: ["nsis"]`, per-user install |
| **Portable** (`tracer-desktop.exe`) | **Secondary** | `target/release/tracer-desktop.exe` after cargo/tauri release build |
| **MSI** | **Not selected** for this RC | Documented deferral (WiX + VBScript feature weight) |

## Identity

| Field | Value |
|---|---|
| Product name | `Tracer` |
| Package id | `dev.tracer.desktop` |
| Stable exe | `tracer-desktop.exe` (`mainBinaryName`) |
| Semver | `0.1.0` (tauri.conf / Cargo.toml / package.json) |
| App data (Windows) | `%LOCALAPPDATA%\dev.tracer.desktop\` |

Icons: `apps/desktop/src-tauri/icons/` (ico + png set).

## Commands

From **repo root**:

```bash
# Identity only
node tools/release/identity-check.mjs

# Build Windows RC (NSIS + portable when tooling allows)
pnpm release:windows
# equivalent:
node tools/release/windows-rc.mjs

# Portable-only (no NSIS bundler)
node tools/release/windows-rc.mjs --no-bundle

# Reuse existing artifacts
node tools/release/windows-rc.mjs --skip-build

# Validate RC-01..RC-06
pnpm test:release:windows
node tools/release/validate-windows.mjs --skip-build

# Signing class
node tools/release/classify-signing.mjs
```

## Signing classes

```text
SIGNED | UNSIGNED_DEVELOPMENT_RC | BLOCKED
```

- Never claim **SIGNED** without Valid Authenticode on artifacts.
- Local RC without certs → **UNSIGNED_DEVELOPMENT_RC** (allowed to **PASS** when classified).
- Mixed/broken signatures or cert env without verifiable artifacts → **BLOCKED**.

## Scenarios

| ID | Name | Notes |
|---|---|---|
| RC-01 | Clean install | NSIS silent `/S` when installer present; else portable presence |
| RC-02 | Fake-runtime smoke | Launch with `TRACER_FAKE_ACP_JS` + temp DB |
| RC-03 | Upgrade | Honest `no_prior_fixture` PASS when no older RC |
| RC-04 | Uninstall | NSIS uninstaller or portable delete procedure |
| RC-05 | Reinstall | Second silent install / portable re-launch |
| RC-06 | Failed launch diagnostics | Missing exe / spawn error evidence |

## Artifacts

Build outputs under `target/` (not committed):

- `target/release/tracer-desktop.exe` — portable
- `target/release/bundle/nsis/*-setup.exe` — NSIS
- `target/release-rc/windows/manifest.json` — staged manifest
- `target/release-rc/windows/rc-summary.json`
- `target/release-rc/windows/rc-validation.json`

## Non-goals

- Live GUI product harness (W2.3-B)
- Ownership of `tools/tauri-e2e` core (W2.3-C)
- Code signing cert procurement
- MSI production for this RC
- macOS / Linux installers
