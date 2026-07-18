# Wave 2.2.3 Test Matrix — L3-J Full GUI + regression

**Gate:** 2.2.3  
**Date:** 2026-07-18  
**Host:** Windows · grok-build · fake ACP only

## 1. Journey matrix (L3-J)

| ID | Name | Result run1 | Result run2 | Notes |
|---|---|---|---|---|
| GJ-01 | startup_tauri_mode | PASS | PASS | backend=tauri |
| GJ-02 | create_first_session | PASS | PASS | path register + session create GUI |
| GJ-03 | streaming_prompt | PASS | PASS | timeline events via GUI |
| GJ-04 | approval_accepted | PASS | PASS | Allow button |
| GJ-05 | approval_rejected | PASS | PASS | Deny button |
| GJ-06 | cancel_while_approval_pending | PASS | PASS | no deadlock |
| GJ-07 | two_session_focus_switch | PASS | PASS | multi-session open |
| GJ-08 | runtime_crash_eof | PASS | PASS | crash UI/events |
| GJ-09 | restart_history_restore | PASS | PASS | same temp DB relaunch |
| GJ-10 | heli_unavailable | PASS | PASS | non-fatal |
| GJ-11 | invoke_failure_fail_closed | PASS | PASS | no silent mock |
| GJ-12 | clean_shutdown | PASS | PASS | orphan verify clean |

**Aggregate:** L3-J **PASS** (12/12) × 2 serial full-suite runs.

## 2. Infrastructure levels

| Level | Command | Result |
|---|---|---|
| Doctor | `pnpm test:tauri-e2e:doctor` | READY |
| L0+L1 | `pnpm -r test` / `pnpm test:tauri-e2e` | PASS |
| L2 | `pnpm test:tauri-e2e:l2` | PASS |
| L3-I | `pnpm test:tauri-e2e:l3i` | PASS |
| L3-J | `pnpm test:tauri-e2e:gui` | PASS 12/12 ×2 |

## 3. Workspace / control-plane regression

| Suite | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS |
| `cargo clippy --workspace --all-targets` | PASS (warnings only) |
| `pnpm -r build` | PASS |
| `vs_scenarios` | 23 PASS |
| `drain_lifecycle` | 14 PASS |
| `multi_session` | 17 PASS |
| `presentation_delivery` | 19 PASS |
| `tracer-vs1-soak` | 8 PASS |
| desktop e2e_env unit tests | 5 PASS |

## 4. Isolation / safety checks

| Check | Result |
|---|---|
| `pnpm -r test` does not launch L2/L3-I/L3-J | PASS (evidence in log) |
| `--tracer-e2e-env` allowlist only | PASS (unit tests) |
| Relative / missing env path rejected | PASS |
| Disallowed keys (PATH, API_KEY, HOME) ignored | PASS |
| Failure artifacts retained + sanitized | PASS (controlled driver fail + unit sanitize) |
| No live credentials / network product path | PASS (fake ACP, meta flags) |

## 5. Non-claims

| Surface | Status |
|---|---|
| Live Grok / live-provider GUI | UNPROVEN / not run |
| macOS / Linux GUI | UNPROVEN |
| IDE / ALMS / plugins / marketplace | Out of scope |
