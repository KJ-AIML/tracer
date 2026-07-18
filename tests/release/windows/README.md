# Windows Release Validation (W2.3-A)

Entry point for **Windows RC packaging validation** scenarios RC-01…RC-06.

Implementation lives under owned path `tools/release/` (not duplicated here).

## Run

From repo root:

```bash
# Identity + packaging
pnpm release:windows
# or portable-only:
node tools/release/windows-rc.mjs --no-bundle

# RC-01..RC-06
pnpm test:release:windows
node tools/release/validate-windows.mjs --skip-build
```

## Scenarios

| ID | Name |
|---|---|
| RC-01 | Clean install |
| RC-02 | Fake-runtime smoke |
| RC-03 | Upgrade (honest if no prior fixture) |
| RC-04 | Uninstall |
| RC-05 | Reinstall |
| RC-06 | Failed launch diagnostics |

## Signing

Results must include class:

`SIGNED` | `UNSIGNED_DEVELOPMENT_RC` | `BLOCKED`

Unsigned local development RC may **PASS** when classified as `UNSIGNED_DEVELOPMENT_RC`.

## Evidence

JSON artifacts (not committed):

- `target/release-rc/windows/rc-summary.json`
- `target/release-rc/windows/rc-validation.json`
- `target/release-rc/windows/manifest.json`

Human-readable matrix: `docs/modules/w2-3-a/W2_3_A_TEST_MATRIX.md`  
Host results: `docs/validation/release/WINDOWS_RELEASE_RESULTS.md`

## Non-ownership

- Live GUI harness journeys → W2.3-B
- `tools/tauri-e2e` core reliability → W2.3-C
