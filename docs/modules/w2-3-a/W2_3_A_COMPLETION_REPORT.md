# W2.3-A Completion Report — Windows Packaging

**Task id:** `tracer-w2-windows-packaging`  
**Work item:** W2.3-A  
**Branch:** `agent/tracer-w2-windows-packaging`  
**Base SHA:** `8f3b3cb568483fde065dae77d341b38e597b23b2`  
**Head SHA:** tip after pin commit (see Commit SHAs; pin does not self-hash)  
**Session id:** `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21`  
**Host:** grok-build  
**Target:** tracer  
**Date:** 2026-07-18  

## Decision

| Item | Result |
|---|---|
| Goal achieved | **Yes** — Windows RC packaging surface with identity, NSIS + portable, RC-01..06, honest signing |
| Primary artifact | **NSIS** `Tracer_0.1.0_x64-setup.exe` |
| Secondary artifact | **Portable** `tracer-desktop.exe` |
| MSI | **Not selected** for this RC (WiX / VBScript weight deferred) |
| Packaging host run | **PASS** (`pnpm release:windows`) |
| RC-01..RC-06 | **PASS** (all six) |
| Signing class | **`UNSIGNED_DEVELOPMENT_RC`** (allowed PASS when classified) |
| False SIGNED claim | **None** |
| Live GUI product harness | **Not owned** (W2.3-B) |
| `tools/tauri-e2e` reliability core | **Not owned** (W2.3-C) |
| Wave merge / push | **Not done** (worker never pushes) |

## Packaging architecture (summary)

```text
pnpm release:windows
  → identity-check
  → tauri build (bundle.targets: ["nsis"])
  → portable PE + NSIS setup
  → stage target/release-rc/windows/
  → classify signing (UNSIGNED_DEVELOPMENT_RC | SIGNED | BLOCKED)

pnpm test:release:windows
  → RC-01 clean install (NSIS silent)
  → RC-02 fake-runtime process smoke
  → RC-03 upgrade (honest no_prior_fixture when unset)
  → RC-04 uninstall
  → RC-05 reinstall
  → RC-06 failed-launch diagnostics
```

Canonical identity:

| Field | Value |
|---|---|
| Product | `Tracer` |
| Package id | `dev.tracer.desktop` |
| Stable exe | `tracer-desktop.exe` |
| Semver | `0.1.0` (tauri.conf / Cargo.toml / package.json) |
| App data (Windows) | `%LOCALAPPDATA%\dev.tracer.desktop\` |

## Host evidence

| Command | Result |
|---|---|
| `node tools/release/identity-check.mjs` | **PASS** |
| `pnpm release:windows` | **PASS** — NSIS + portable produced |
| `node tools/release/validate-windows.mjs --skip-build` | **PASS** — RC-01..06 |
| `node tools/release/classify-signing.mjs` | **UNSIGNED_DEVELOPMENT_RC** (NotSigned) |

Artifact sizes on this host:

| File | Bytes |
|---|---|
| `target/release/tracer-desktop.exe` | 17165312 |
| `target/release/bundle/nsis/Tracer_0.1.0_x64-setup.exe` | 4116266 |

Detailed host write-up: [`docs/validation/release/WINDOWS_RELEASE_RESULTS.md`](../../validation/release/WINDOWS_RELEASE_RESULTS.md)

### RC scenario classification

| ID | Result | Honesty |
|---|---|---|
| RC-01 | PASS | Full NSIS silent install |
| RC-02 | PASS | Ready marker + clean kill (process smoke, not GUI journey) |
| RC-03 | PASS | `no_prior_fixture` — upgrade vs older RC **not proven** |
| RC-04 | PASS | NSIS uninstaller |
| RC-05 | PASS | Second silent install |
| RC-06 | PASS | Spawn failure diagnostics |

## Deliverables

### Config

| Path | Role |
|---|---|
| `apps/desktop/src-tauri/tauri.conf.json` | `bundle.active: true`, `targets: ["nsis"]`, publisher, icons, NSIS currentUser, no cert thumbprint |
| `apps/desktop/src-tauri/capabilities/default.json` | Description only; IPC surface unchanged |
| `apps/desktop/src-tauri/icons/` | ico + png set (existing + referenced) |
| Root `package.json` | `release:windows`, `test:release:windows` |
| `pnpm-lock.yaml` | Register `tools/release` workspace package |

### Release tooling

| Path | Role |
|---|---|
| `tools/release/windows-rc.mjs` | Packaging entry (`pnpm release:windows`) |
| `tools/release/validate-windows.mjs` | RC-01..06 validation |
| `tools/release/identity-check.mjs` | Identity consistency CLI |
| `tools/release/classify-signing.mjs` | Authenticode classification CLI |
| `tools/release/lib/paths.mjs` | Path constants |
| `tools/release/lib/identity.mjs` | Identity model + checks |
| `tools/release/lib/signing.mjs` | Signing classes |
| `tools/release/lib/artifacts.mjs` | Discover + stage artifacts |
| `tools/release/package.json` | `@tracer/release` package |
| `tools/release/README.md` | Operator guide |
| `tests/release/windows/README.md` | Test entry pointing at tools |

### Docs

| Path | Role |
|---|---|
| `docs/modules/w2-3-a/W2_3_A_PACKAGING_ARCHITECTURE.md` | Architecture + decision |
| `docs/modules/w2-3-a/W2_3_A_TEST_MATRIX.md` | Test matrix RC-01..06 |
| `docs/modules/w2-3-a/W2_3_A_COMPLETION_REPORT.md` | This report |
| `docs/validation/release/WINDOWS_RELEASE_RESULTS.md` | Host evidence |

## Integration requirements

1. **Root scripts (merged with this branch):**
   ```json
   "release:windows": "node tools/release/windows-rc.mjs",
   "test:release:windows": "node tools/release/validate-windows.mjs"
   ```

2. **CI:** gate packaging/validation on **Windows** runners only. Non-Windows exits `3` (`UNSUPPORTED_PLATFORM`).

3. **Signing follow-on:** supply Authenticode cert + env (`WINDOWS_CERTIFICATE_THUMBPRINT` / Tauri signing keys) and re-run `classify-signing.mjs`. Do not claim `SIGNED` without Valid Authenticode.

4. **MSI (optional later):** extend `bundle.targets` to `["nsis", "msi"]` once WiX is available; validation already discovers `.msi` if present.

5. **L2/L3 harness:** keep discovering `tracer-desktop.exe` cargo artifact; enabling `bundle.active` does not require W2.3-C ownership changes.

6. **Do not** treat RC-02 process smoke as full GUI product acceptance (W2.3-B).

7. Build outputs under `target/` and `apps/desktop/dist/` are **not committed**.

## Owned path compliance

| Path | Action |
|---|---|
| `apps/desktop/src-tauri/tauri.conf.json` | Updated (bundle packaging) |
| `apps/desktop/src-tauri/capabilities/` | Description only |
| `apps/desktop/src-tauri/icons/` | Referenced / used by bundle |
| `tools/release/` | Added |
| `tests/release/windows/` | Added README entry |
| `docs/modules/w2-3-a/` | Architecture, matrix, completion |
| `docs/validation/release/` | Host results |
| Root `package.json` / `pnpm-lock.yaml` | Minimal scripts + workspace entry |

## Forbidden paths (not redesigned)

- Control plane / runtime / storage
- Product UI redesign
- Live GUI harness (W2.3-B)
- Ownership of `tools/tauri-e2e` core (W2.3-C)
- IDE / ALMS / plugins

## Commit SHAs

| Commit | Summary |
|---|---|
| `694d06c` | feat(w2.3-a): Windows RC packaging config and release tooling |
| `60b3f6f` | docs(w2.3-a): packaging architecture, test matrix, results, completion |

Branch tip after the SHA-pin commit is the integration head pointer; that pin commit is omitted from the table to avoid self-hash ambiguity.

## Residual risk / follow-ons

| Item | Status |
|---|---|
| Production code signing | Follow-on (cert + CI secrets) |
| Multi-version upgrade fixture | Provide `TRACER_RC_PRIOR_INSTALL` for full RC-03 proof |
| MSI channel | Deferred |
| SmartScreen reputation for unsigned NSIS | Expected friction for local RC; not a packaging defect |
