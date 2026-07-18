# Wave 2.3 Test Matrix

**Gate:** 2.3  
**Date:** 2026-07-18  
**Host:** Windows | grok-build | fake ACP (live NOT_RUN)

## 1. GUI reliability (L3-J ? 5)

| Run | Result | Journeys | Retries | Product fails |
|---|---|---|---|---|
| 1 | PASS | 12/12 | 0 | 0 |
| 2 | PASS | 12/12 | 0 | 0 |
| 3 | PASS | 12/12 | 0 | 0 |
| 4 | PASS | 12/12 | 0 | 0 |
| 5 | PASS | 12/12 | 0 | 0 |

**Aggregate:** orphans=0, port collisions=0, temp cleanup failures=0, unsanitized=0.

| Inject / selftest | Result |
|---|---|
| `pnpm test:tauri-e2e:inject-fail` | PASS 113/113 |
| `pnpm test:tauri-e2e:reliability` | PASS 18/18 |

## 2. Windows RC-01...RC-06

| ID | Name | Classification | Notes |
|---|---|---|---|
| RC-01 | Clean install | **PASS** | silent NSIS ? exe present |
| RC-02 | Fake-runtime smoke | **PASS** | process + ready marker |
| RC-03 | Upgrade | **PARTIAL / FIXTURE_LIMITED** | no prior package fixture |
| RC-04 | Uninstall | **PASS** | uninstall.exe /S |
| RC-05 | Reinstall | **PASS** | second silent install |
| RC-06 | Failed launch diagnostics | **PASS** | diagnostic spawn path |

Signing: **UNSIGNED_DEVELOPMENT_RC**. Overall RC: **PASS** (with RC-03 honesty).

## 3. Live GUI LGJ-01...07

| ID | Classification |
|---|---|
| LGJ-01...07 | **NOT_RUN** |
| Unit / dry-run | PASS / constructionPass |

## 4. Infrastructure + CI isolation

| Level | Command | In `pnpm -r test`? | Result |
|---|---|---|---|
| Doctor | `pnpm test:tauri-e2e:doctor` | no | READY |
| L0+L1 | `pnpm -r test` / `run.mjs` | **yes** | PASS |
| L2 | `pnpm test:tauri-e2e:l2` | **no** | PASS |
| L3-I | `pnpm test:tauri-e2e:l3i` | **no** | PASS |
| L3-J | `pnpm test:tauri-e2e:gui` | **no** | PASS |
| Live LGJ | `pnpm test:tauri-e2e:live-gui` | **no** | NOT_RUN |

## 5. Deterministic workspace

| Suite | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |
| `cargo clippy --workspace --all-targets` | PASS |
| `pnpm install --frozen-lockfile` | PASS |
| `pnpm -r test` | PASS |
| `pnpm -r build` | PASS |
| `vs_scenarios` | PASS |
| `drain_lifecycle` | PASS (14) |
| `multi_session` | PASS (17) |
| `presentation_delivery` | PASS (19) |
| `tracer-vs1-soak` | PASS (8) |
