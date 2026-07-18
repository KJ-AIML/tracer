# W2.4.1 Test Matrix

**Task:** tracer-w2-upgrade-fixture (W2.4.1-A)

## Package upgrade (RC-03 / upgrade fixture)

| ID | Case | Expected | Evidence |
|---|---|---|---|
| R01 | Identity stable N→N+1 | `dev.tracer.desktop` | upgrade-fixture |
| R02 | Migrations once | idempotent schema 2 | UF-04 + sqlx |
| R03 | Data restores | sessions/events kept | assertDataPreserved |
| R04 | No duplicates | unique session ids | assertDataPreserved |
| R05 | New session after upgrade | persists | upgrade-fixture |
| R06 | Restart restores old+new | relaunch | upgrade-fixture |
| R07 | No orphan handles | clean kill | smokeLaunch |
| R08 | NSIS upgrade path | `/S /D=` over prior | upgrade-fixture |
| R09 | Isolated paths | TEMP fixture root | path record |
| R10 | Fake ACP only | `TRACER_FAKE_ACP_JS` | env |
| R11 | Uninstall retention | DB retained | upgrade-fixture |
| R12 | Reinstall history | sessions restored | upgrade-fixture |
| R13 | Pre-upgrade capture | ≥2 sessions | seed |
| R14 | Schema advanced | 1→2 via product sqlx | captureState |

## Failure cases

| ID | Case | Classification | Layer |
|---|---|---|---|
| UF-01 | Future schema | CONTROLLED_REFUSAL | storage + package |
| UF-02 | Migration interruption | ROLLBACK_RECOVERY | storage (sqlx tx) |
| UF-03 | Corrupt prior DB | DIAGNOSTICS_NO_SILENT_RESET | storage + package |
| UF-04 | Repeated launch | IDEMPOTENT | storage + package |
| UF-05 | Downgrade N after N+1 | CONTROLLED_REFUSAL | storage + package |

## Commands

```text
cargo test -p tracer-storage --test upgrade_safety
pnpm test:release:upgrade
TRACER_RC_PRIOR_NSIS=... pnpm test:release:windows -- --skip-build
pnpm release:provenance && pnpm release:provenance:verify
```