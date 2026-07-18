# Windows Upgrade Results (W2.4.1-A)

**Date:** 2026-07-18  
**Host:** grok-build (Windows)  
**Heli session:** `heli-ses-7c596a03-9e8d-4054-82b0-1b55164dbe6b`  
**Branch:** `agent/tracer-w2-upgrade-fixture`  
**Base SHA:** `4c5f5599df16325f39da1b3165d7c02be94ac0a4`

## Version provenance

| Field | N (prior) | N+1 (current) |
|---|---|---|
| Semver | 0.1.0 | 0.1.1 |
| Source SHA | `4c5f5599df16325f39da1b3165d7c02be94ac0a4` | branch tip (post-commit) |
| Schema | 1 | 2 |
| Identifier | `dev.tracer.desktop` | `dev.tracer.desktop` |
| Fixture isolation | `dev.tracer.desktop.upgrade-fixture` + `TRACER_DATABASE_PATH` | same |

## Artifact hashes (local; not committed)

| Version | Type | Filename | Size | SHA-256 |
|---|---|---|---|---|
| N | portable | tracer-desktop.exe | 17198080 | `7fdc5bfcf127991d8839d4a19886ed6ede101e3168a901449095f6dde67fc886` |
| N | nsis | Tracer_0.1.0_x64-setup.exe | 4127299 | `b6802deb0130340d7de8e29f4690419b82f816e644b11ab2afb5cf8d30356837` |
| N+1 | portable | tracer-desktop.exe | 17107968 | `1e89d9bcb2a902b83fbc48da94a7f5d5a6b8fea7f58b38b74dec866839163ef8` |
| N+1 | nsis | Tracer_0.1.1_x64-setup.exe | 4116383 | `a83ad47cc3910d56b124062eee632e70281784b651013b3663bd07de63e195dc` |

## Upgrade fixture

```text
pnpm test:release:upgrade -- --skip-build-n1
→ RESULT: PASS
→ migrationMode: product_sqlx
→ schema: 1 → 2
→ sessions: 3 → 4 (prior preserved + post-upgrade session)
```

JSON: `target/release-rc/windows/upgrade-fixture-results.json` (not committed)

## RC-03 with prior NSIS

```text
$env:TRACER_RC_PRIOR_NSIS="target/release-rc/upgrade-fixture/vN/Tracer_0.1.0_x64-setup.exe"
node tools/release/validate-windows.mjs --skip-build
→ RC-03 PASS (mode nsis_n_to_n1)
→ RC-01..RC-06 PASS
```

## UF-01…UF-05

| ID | Result | Classification |
|---|---|---|
| UF-01 | PASS | CONTROLLED_REFUSAL |
| UF-02 | PASS | ROLLBACK_RECOVERY |
| UF-03 | PASS | DIAGNOSTICS_NO_SILENT_RESET |
| UF-04 | PASS | IDEMPOTENT |
| UF-05 | PASS | CONTROLLED_REFUSAL |

## Provenance

```text
pnpm release:provenance && pnpm release:provenance:verify
→ PASS (UNSIGNED_DEVELOPMENT_RC)
```

## Validation suite (this host)

| Check | Result |
|---|---|
| cargo fmt --all --check | PASS (after fmt) |
| cargo check --workspace | PASS |
| cargo test --workspace | PASS |
| cargo clippy --workspace --all-targets | PASS (warnings exist in unrelated crates; -D warnings not required) |
| pnpm install --frozen-lockfile | PASS |
| pnpm -r test | PASS |
| pnpm -r build | PASS |
| pnpm test:tauri-e2e:doctor | PASS |
| pnpm test:tauri-e2e:l2 | PASS |
| pnpm test:tauri-e2e:l3i | PASS (driver skips honest) |
| cargo test -p tracer-storage upgrade_safety | PASS (7/7) |