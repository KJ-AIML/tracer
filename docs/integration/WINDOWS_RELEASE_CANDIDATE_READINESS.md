# Windows Release Candidate Readiness (Gate 2.3)

**Gate:** 2.3  
**Decision:** **PASS** (UNSIGNED_DEVELOPMENT_RC; RC-03 fixture-limited)  
**Date:** 2026-07-18  
**Integrator session:** `heli-ses-9ccdc8b9-7065-43ff-b243-85efe0759187`

## Identity

| Field | Value | Status |
|---|---|---|
| productName | Tracer | PASS |
| identifier | `dev.tracer.desktop` | PASS |
| mainBinaryName | `tracer-desktop` | PASS |
| version | 0.1.0 (tauri / package / Cargo aligned) | PASS |
| bundle.targets | NSIS primary | PASS |
| certificateThumbprint | null | PASS (unsigned RC) |

## Artifacts (local only; not committed)

| Output | Path | Size | SHA-256 |
|---|---|---|---|
| Portable | `target/release/tracer-desktop.exe` | 17198080 | `a39c14cb3eee0caa72a950ae88ebab4e3aa8572ceec11c2e0207c2af25991ee5` |
| NSIS setup | `target/release/bundle/nsis/Tracer_0.1.0_x64-setup.exe` | 4127658 | `829e9a7e0342afa110899d827f6c5c4b8e66a414a59c5e498e6c62c0f1645314` |

## Signing

```text
class: UNSIGNED_DEVELOPMENT_RC
Authenticode: NotSigned (portable + NSIS)
```

Allowed for Gate 2.3 PASS. **No production SIGNED claim.**

## RC scenarios

| ID | Result |
|---|---|
| RC-01 | PASS |
| RC-02 | PASS |
| RC-03 | PARTIAL / FIXTURE_LIMITED |
| RC-04 | PASS |
| RC-05 | PASS |
| RC-06 | PASS |

## Toolchain

| Item | Version |
|---|---|
| OS | Windows 10.0.26200 (win32/x64) |
| rustc / cargo | 1.96.0 |
| Node | v24.16.0 |
| pnpm | 9.15.0 |
| Tauri CLI | `@tauri-apps/cli@2` via npx |

## Commands

```text
node tools/release/identity-check.mjs   ? PASS
pnpm release:windows                    ? PASS
pnpm test:release:windows -- --skip-build ? PASS (RC-03 honest fixture)
node tools/release/classify-signing.mjs ? UNSIGNED_DEVELOPMENT_RC
```

## W2.4.1 supersession note (additive; does not rewrite Gate 2.3 history)

Gate 2.3 recorded RC-03 as **PARTIAL / FIXTURE_LIMITED** (no prior package fixture). That historical classification remains accurate for Gate 2.3.

**Current RC-03 status after W2.4.1: PASS** — real N (0.1.0) → N+1 (0.1.1) NSIS upgrade proven on Gate 2.4.1 with data preservation. See `docs/integration/WAVE_2_4_1_INTEGRATION_REPORT.md` and `WINDOWS_UPGRADE_READINESS.md`.
