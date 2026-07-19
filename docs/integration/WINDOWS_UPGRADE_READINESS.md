# Windows Upgrade Readiness (Gate 2.4.1)

**Gate:** 2.4.1  
**Decision:** **PASS**  
**Date:** 2026-07-19  
**Session:** `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21`  
**buildSourceSha:** `e04f81f5089d0414ef8967b0d98384d7b199b9b7`

## Posture

| Concern | Result |
|---|---|
| Real N→N+1 NSIS upgrade | **PASS** |
| Data preservation | **PASS** |
| Migration interruption | **PASS** / `ROLLBACK_RECOVERY` |
| Downgrade | **CONTROLLED_REFUSAL** |
| Uninstall data retention | **PASS** |
| Reinstall history restore | **PASS** |
| Fixture isolation | TEMP only; never operator `%LOCALAPPDATA%\dev.tracer.desktop` |
| Runtime | fake ACP only |
| Credentials / live Grok | none |

## Versions

| | N | N+1 |
|---|---|---|
| Semver | 0.1.0 | 0.1.1 |
| Schema | 1 | 2 |
| Source | `4c5f559…` | `e04f81f…` |
| Identifier | `dev.tracer.desktop` | `dev.tracer.desktop` |

## RC-03 supersession (non-destructive)

- Gate 2.3 historical docs retain **PARTIAL / FIXTURE_LIMITED** for RC-03.
- **Current RC-03 status after W2.4.1: PASS** (prior NSIS `Tracer_0.1.0_x64-setup.exe` → `Tracer_0.1.1_x64-setup.exe`).

## Commands

```text
pnpm test:release:upgrade -- --skip-build-n1
$env:TRACER_RC_PRIOR_NSIS="target/release-rc/upgrade-fixture/vN/Tracer_0.1.0_x64-setup.exe"
node tools/release/validate-windows.mjs --skip-build
cargo test -p tracer-storage --test upgrade_safety
```

## Tip pin

Report commit (pre-finalize tip candidate): `530b4d891a994644f088dd774c2a9640616d2b3c`. Final `gateTipSha` is main after FF + any pin commit.
