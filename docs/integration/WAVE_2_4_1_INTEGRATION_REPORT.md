# Wave 2.4.1 Integration Report — Upgrade Fixture + Release Provenance

**Gate:** 2.4.1  
**Task:** `tracer-w2-upgrade-fixture-integration`  
**Work item:** W2.4.1-I  
**Integrator host:** `grok-build`  
**Heli session:** `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21`  
**Write target:** `tracer` (`repos/tracer` main worktree)  
**Integration branch:** `integration/tracer-w2-4-1-upgrade`  
**Date:** 2026-07-19  
**Platform:** Microsoft Windows | rustc/cargo 1.96.0 | Node v24.16.0 | pnpm 9.15.0

## 1. Gate 2.4.1 decision

| Field | Value |
|---|---|
| **Gate 2.4.1** | **PASS** |
| Upgrade (N→N+1 NSIS) | **PASS** (supersedes Gate 2.3 RC-03) |
| Data preservation | **PASS** |
| Migration interruption | **PASS** (`ROLLBACK_RECOVERY`) |
| Downgrade | **CONTROLLED_REFUSAL** |
| Release provenance | **PASS** |
| Signing | **UNSIGNED_DEVELOPMENT_RC** |
| UF-01…UF-05 | **PASS** (see matrix) |
| Uninstall / reinstall | **PASS** (data retained; history restored) |

## 2. Binding / lease

| Check | Result |
|---|---|
| Task | `tracer-w2-upgrade-fixture-integration` / `W2.4.1-I` |
| Claim | write / host `grok-build` (takeover of stale lease) |
| Session | `heli-ses-26b01af7-555d-440d-a6e0-da64824c2c21` |
| Lease | `heli-lease-50a3bc23-23a5-4e0b-82ad-6bff1c2dbbd3` |
| Target | `tracer` → writes under `repos/tracer` |
| Source branch | `agent/tracer-w2-upgrade-fixture` @ `c03502603a739195bcc126bd1f25584f3ff427d3` |
| Baseline main | `4c5f5599df16325f39da1b3165d7c02be94ac0a4` (Gate 2.3 PASS) |
| Push | **Never** |

## 3. Merge + reconciliation

| Role | SHA | Message |
|---|---|---|
| Merge (no-ff) | `c348d24` | `merge(w2.4.1-i): integrate W2.4.1-A upgrade fixture and release provenance` |
| Provenance tooling | `d4c10ba` | `fix(release): distinguish buildSourceSha from gateTipSha in provenance` |
| **N+1_BUILD_SOURCE_SHA** | `e04f81f5089d0414ef8967b0d98384d7b199b9b7` | `chore(release): pin N+1_BUILD_SOURCE_SHA for Gate 2.4.1` |
| Report commits | tip after this doc set | Gate 2.4.1 docs only |

No squash. No product/storage/migration/package/release-tooling changes after `e04f81f`.

## 4. Version N / N+1

| Field | N | N+1 |
|---|---|---|
| Semver | `0.1.0` | `0.1.1` |
| Source SHA | `4c5f5599df16325f39da1b3165d7c02be94ac0a4` | `e04f81f5089d0414ef8967b0d98384d7b199b9b7` (`buildSourceSha`) |
| Schema | 1 | 2 |
| Identifier | `dev.tracer.desktop` | `dev.tracer.desktop` (stable) |
| Fixture isolation | `dev.tracer.desktop.upgrade-fixture` + TEMP `TRACER_DATABASE_PATH` | same |
| Signing | `UNSIGNED_DEVELOPMENT_RC` | `UNSIGNED_DEVELOPMENT_RC` |
| Tag (N) | `tracer-wave2.3-windows-rc` (unchanged) | — |

N rebuilt from immutable Gate 2.3 tip in worktree `repos/worktrees/tracer-w2-4-1-n` (detached `4c5f559`). Source not modified.

### Artifact hashes (local; not committed)

| Version | Type | Filename | Size | SHA-256 |
|---|---|---|---|---|
| N | portable | `tracer-desktop.exe` | 17198080 | `77b04db25b48d24f19283b0bff2f1ac18bbea56240c70de63f4f70c50bf15f54` |
| N | nsis | `Tracer_0.1.0_x64-setup.exe` | 4128311 | `53c3513cfb33f108b6ba0a314051b18373cf334bd81242dbf39e3571a47087ea` |
| N+1 | portable | `tracer-desktop.exe` | 17107968 | `e530bae51a42f81d213e59dcd72680c14efd3814956e4fbbafb715f296acf4f2` |
| N+1 | nsis | `Tracer_0.1.1_x64-setup.exe` | 4117309 | `5ca4452b974070bcb47dc21734d995abaeb502ad1f5354391e7fc79bf5ba5e2a` |

Hash differences vs Gate 2.3 published NSIS/portable hashes are expected (timestamped NSIS metadata / rebuild clock). Immutable source SHA for N remains `4c5f559`.

## 5. Storage migration audit (Part 5)

| # | Requirement | Evidence | Result |
|---|---|---|---|
| 1 | Additive migration `002_schema_v2_upgrade_marker.sql` | migration file | PASS |
| 2 | Logical schema 1 → 2 | `SCHEMA_LOGICAL_VERSION` / fixture | PASS |
| 3 | Upgrade marker key written | `upgrade_marker_w2_4_1=schema_v2` | PASS |
| 4 | Future schema refused | `refuse_unsupported_future_schema` | PASS |
| 5 | No silent corrupt reset | UF-03 + bytes unchanged | PASS |
| 6 | Transactional / rollback safe | sqlx + UF-02 | PASS |
| 7 | Idempotent re-run | UF-04 + `run_migrations` | PASS |
| 8 | Sessions/events preserved | `schema_v1_to_v2_preserves_sessions` + fixture | PASS |
| 9 | No secrets columns | `no_secrets_columns_in_schema` | PASS |
| 10 | Error classes not weakened | `StorageErrorClass::MigrationFailed` / diagnostics | PASS |
| 11 | Downgrade = CONTROLLED_REFUSAL | UF-05 + `classify_downgrade_open` | PASS |
| 12 | Product sqlx applies 002 once | `migrationMode: product_sqlx` | PASS |
| 13 | Numeric guard `SCHEMA_LOGICAL_VERSION_NUM=2` | `lib.rs` | PASS |
| 14 | Fresh DB lands on schema 2 | `fresh_db_is_schema_v2` | PASS |
| 15 | Isolation never touches operator DB | TEMP fixture + `operatorAppDataAvoided` | PASS |

## 6. Version / identity audit (Part 6)

`checkIdentity()` PASS: Tracer / `dev.tracer.desktop` / `tracer-desktop` / `0.1.1` aligned across `tauri.conf.json`, `Cargo.toml`, `apps/desktop/package.json`. Fixture id `dev.tracer.desktop.upgrade-fixture` documented. Version drift fails provenance generation.

## 7. Upgrade + RC-03 supersession

```text
pnpm test:release:upgrade -- --skip-build-n1
→ RESULT: PASS (R01…R14 + UF-01…UF-05)
→ schema 1→2; sessions 3→4; uninstall retain + reinstall restore

TRACER_RC_PRIOR_NSIS=…/Tracer_0.1.0_x64-setup.exe
node tools/release/validate-windows.mjs --skip-build
→ RC-01…RC-06 PASS (RC-03 mode nsis_n_to_n1)
```

**Historical Gate 2.3 RC-03 remains PARTIAL/FIXTURE_LIMITED in Gate 2.3 docs.**  
**Current RC-03 status after W2.4.1: PASS.**

## 8. Provenance + hygiene

| Check | Result |
|---|---|
| `pnpm release:provenance` | PASS |
| `pnpm release:provenance:verify` | PASS |
| `buildSourceSha` | `e04f81f…` |
| `gateTipSha` | tip after report commits |
| Hygiene | No absolute developer home paths; no live credentials. Substring hits (`TRACER_E2E_*` env names; `sk-` inside unrelated strings) classified benign. |

## 9. Regression summary

| Suite | Result |
|---|---|
| cargo fmt / check / test / clippy | PASS (clippy warnings pre-existing; not `-D`) |
| pnpm install / `-r test` / `-r build` | PASS |
| tracer-storage (+ upgrade_safety 7/7) | PASS |
| control-plane vs/drain/multi/presentation | PASS |
| tracer-vs1-soak | PASS |
| tauri-e2e doctor / L2 / L3-I | READY / PASS / PASS |
| release upgrade + provenance | PASS |

## 10. Residual risks

1. Production Authenticode / CI secrets still out of scope (`UNSIGNED_DEVELOPMENT_RC`).  
2. Live Grok GUI remains NOT_RUN (separate authorization).  
3. Clippy debt in unrelated crates; soft clippy PASS only.  
4. NSIS hashes are rebuild-clock sensitive — always record SHA-256 per build.  
5. Staged binaries under `target/` must never be committed.

## 11. Finalize plan

| Step | Result |
|---|---|
| FF merge → `main` | after report commits |
| Tag | `tracer-wave2.4.1-upgrade-verified` (annotated, local) |
| Keep | `tracer-wave2.3-windows-rc` untouched |
| Lease release | `tracer-w2-upgrade-fixture-integration` |
| Push | **Never** |