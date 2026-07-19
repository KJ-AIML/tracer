# Wave 2.4.1 Test Matrix

**Gate:** 2.4.1  
**Task:** `tracer-w2-upgrade-fixture-integration`  
**Date:** 2026-07-19  
**Host:** Windows | grok-build | fake ACP only  
**buildSourceSha:** `e04f81f5089d0414ef8967b0d98384d7b199b9b7`

## 1. Package upgrade (R01…R14)

| ID | Case | Result | Evidence |
|---|---|---|---|
| R01 | Identity stable N→N+1 | **PASS** | `dev.tracer.desktop` + isolated DB path |
| R02 | Migrations once | **PASS** | product_sqlx; schema 2 |
| R03 | Data restores | **PASS** | assertDataPreserved |
| R04 | No duplicates | **PASS** | unique session ids |
| R05 | New session after upgrade | **PASS** | post-upgrade session |
| R06 | Restart restores old+new | **PASS** | relaunch |
| R07 | No orphan handles | **PASS** | smokeLaunch orphan check |
| R08 | NSIS upgrade path | **PASS** | `/S /D=` over prior |
| R09 | Isolated paths | **PASS** | TEMP `tracer-upgrade-fixture-*` |
| R10 | Fake ACP only | **PASS** | `TRACER_FAKE_ACP_JS` |
| R11 | Uninstall retention | **PASS** | DB retained |
| R12 | Reinstall history | **PASS** | sessions=4 restored |
| R13 | Pre-upgrade capture | **PASS** | 3 sessions + approval + events |
| R14 | Schema advanced | **PASS** | 1→2 |
| R15 | GUI smoke where supported | **PASS** | packaged smoke + ready marker |
| R16 | Sanitized fingerprint only | **PASS** | size+sha256; no committed DB |

## 2. Failure / safety (UF-01…UF-05)

| ID | Case | Classification | Result |
|---|---|---|---|
| UF-01 | Future schema | CONTROLLED_REFUSAL | **PASS** |
| UF-02 | Migration interruption | ROLLBACK_RECOVERY | **PASS** |
| UF-03 | Corrupt prior DB | DIAGNOSTICS_NO_SILENT_RESET | **PASS** |
| UF-04 | Repeated launch | IDEMPOTENT | **PASS** |
| UF-05 | Downgrade N after N+1 | CONTROLLED_REFUSAL | **PASS** |

## 3. RC-01…RC-06 (with prior NSIS)

| ID | Result | Notes |
|---|---|---|
| RC-01 | **PASS** | clean install |
| RC-02 | **PASS** | fake-runtime smoke |
| RC-03 | **PASS** | supersedes Gate 2.3 PARTIAL/FIXTURE_LIMITED |
| RC-04 | **PASS** | uninstall |
| RC-05 | **PASS** | reinstall |
| RC-06 | **PASS** | failed-launch diagnostics |

Signing: **UNSIGNED_DEVELOPMENT_RC**.

## 4. Deterministic workspace

| Suite | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |
| `cargo clippy --workspace --all-targets` | PASS (soft; pre-existing warnings) |
| `pnpm install --frozen-lockfile` | PASS |
| `pnpm -r test` | PASS |
| `pnpm -r build` | PASS |
| `cargo test -p tracer-storage` | PASS |
| control-plane vs/drain/multi/presentation | PASS |
| `cargo test -p tracer-vs1-soak` | PASS |
| `pnpm test:tauri-e2e:doctor` | READY |
| `pnpm test:tauri-e2e:l2` | PASS |
| `pnpm test:tauri-e2e:l3i` | PASS |
| `pnpm test:release:upgrade` | PASS |
| `pnpm release:provenance` (+ verify) | PASS |