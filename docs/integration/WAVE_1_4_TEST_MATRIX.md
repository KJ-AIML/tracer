# WAVE 1.4 Test Matrix

Platform: Windows | Default CI class: standard CI | network: no | credentials: no | live Grok: no

## Sequence / soak (Gate 1.4 critical)

| Requirement | Named test | Layers | Command | CI class | Result |
|---|---|---|---|---|---|
| Sequence monotonic under burst >256 | soak01_event_burst_beyond_bridge | CP→bridge→SQLite→metrics | `cargo test -p tracer-vs1-soak -- --test-threads=1` | standard CI | **PASS** (accepted=607, persisted=607, loss=0, dups=0) |
| Slow DB backpressure | soak02_slow_database_backpressure | delay hook + burst | same | standard CI | **PASS** |
| Slow presentation non-blocking | soak03_slow_presentation_does_not_block_persist | fan-out + persist | same | standard CI | **PASS** |
| Concurrent command races | soak04_concurrent_commands | multi-task CP | same | standard CI | **PASS** |
| Restart / recovery | soak05_restart_recovery | file SQLite reopen | same | standard CI | **PASS** |
| Repeated sessions | soak06_repeated_sessions | sequential live sessions | same | standard CI | **PASS** |
| Sticky persist_failed isolation | soak07_persist_failed_does_not_poison_later_sessions | per-session state | same | standard CI | **PASS** |
| Stress sequential | stress_sequential_sessions_time_capped | CP sessions | `cargo test -p tracer-vs1-stress` | standard CI | **PASS** (20 sessions) |
| Threshold constants | soak_thresholds_documented | unit | soak package | standard CI | **PASS** |

## VS-01…14 (control plane vertical slice)

| Requirement | Named test | Command | DB | Result |
|---|---|---|---|---|
| VS-01 happy path | vs01_successful_run | `cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1` | memory | **PASS** |
| VS-01 file-backed | vs01_file_backed_successful_run | same | file temp | **PASS** |
| VS-02 auth required | vs02_authentication_required | same | memory | **PASS** |
| VS-03 auth failure | vs03_authentication_failure_distinct | same | memory | **PASS** |
| VS-04 capability | vs04_unsupported_capability_controlled | same | memory | **PASS** |
| VS-05 cancel/approval | vs05_cancel_before_approval_no_deadlock | same | memory | **PASS** |
| VS-05 file-backed | vs05_file_backed_cancel_before_approval_no_deadlock | same | file temp | **PASS** |
| VS-06 approval allow | vs06_approval_accepted_once | same | memory | **PASS** |
| VS-07 approval deny | vs07_approval_rejected_once | same | memory | **PASS** |
| VS-08 EOF | vs08_runtime_eof_terminal | same | memory | **PASS** |
| VS-08 file-backed | vs08_file_backed_runtime_eof_terminal | same | file temp | **PASS** |
| VS-09 crash | vs09_runtime_crash_distinct | same | memory | **PASS** |
| VS-09 file-backed | vs09_file_backed_runtime_crash_distinct | same | file temp | **PASS** |
| VS-10 malformed | vs10_malformed_protocol_distinct | same | memory | **PASS** |
| VS-11 unknown vendor | vs11_unknown_vendor_preserved | same | memory | **PASS** |
| VS-12 restart | vs12_restart_restores_history | same | file reopen | **PASS** |
| VS-13 interrupt | vs13_interrupted_session_recovery | same | file reopen | **PASS** |
| VS-14 heli missing | vs14_heli_unavailable_runtime_usable | same | memory | **PASS** |
| File reopen / migrations | file_backed_reopen_migrations_and_ordering | same | file reopen | **PASS** |
| Aggregate VS suite | 23 tests | same | mixed | **23 passed** |

## Desktop / presentation (H2)

| Requirement | Named test | Command | Live Grok | Result |
|---|---|---|---|---|
| Typed snapshot store | snapshotStore.test.ts (14) | `pnpm -r test` / apps/desktop vitest | no | **PASS** |
| Mock store journey | mockStore.test.ts (4) | same | no | **PASS** |
| Desktop production build | tsc + vite | `pnpm -r build` | no | **PASS** |
| Full GUI E2E | n/a | future | no | **deferred** (not claimed) |

## Live harness (H1) — CI exclusion

| Requirement | Named test | Command | Opt-in | Result |
|---|---|---|---|---|
| Dry-run unit tests | 16 tests | `cargo test -p live-grok-smoke` | none | **PASS** (no process spawn for live agent) |
| CLI dry-run | dry-run | `cargo run -p live-grok-smoke -- dry-run` | none | **PASS** (`classification=NOT_RUN`) |
| Live authenticated LVS | H1 authoring host | `TRACER_LIVE_GROK=1 run` | **required** | **Reused H1 PASS**; **no Gate 1.4 rerun** |

## Workspace aggregate

| Requirement | Command | Result |
|---|---|---|
| Format | `cargo fmt --all --check` | **PASS** |
| Check | `cargo check --workspace` | **PASS** |
| Test | `cargo test --workspace` | **PASS** |
| Clippy | `cargo clippy --workspace --all-targets` | **PASS** (style warnings only) |
| JS install | `pnpm install --frozen-lockfile` | **PASS** |
| JS test | `pnpm -r test` | **PASS** |
| JS build | `pnpm -r build` | **PASS** |
| Fake ACP contract | tests/contract/fake-runtime | **30 passed** |
| event-types | packages/event-types | **11 passed** |
| ui | packages/ui | **3 passed** |

## Explicit non-runs / non-claims

| Item | Status |
|---|---|
| Live Grok provider usage in Gate 1.4 | **Not consumed** |
| Standard CI live job | **Must not exist / must stay gated** |
| Full Tauri GUI E2E | **Not run** |
| Cross-platform live matrix | **Not claimed** |
