# Wave 1.1 Test Matrix

**Gate:** 1.1  
**Task:** `tracer-w1-1-integration`  
**Date:** 2026-07-17  
**Host platform:** Windows 10/11 (NT 10.0.26200.0)  
**Tooling:** rustc/cargo 1.96.0 · Node v24.16.0 · pnpm 9.15.0  
**Network / live Grok credentials:** **none** (standard CI path)

## 1. Aggregate commands

| # | Command | Pass/Fail | Network | Creds | Notes |
|---|---|---|---|---|---|
| 1 | `cargo fmt --all --check` | **PASS** | No | No | After integrator fmt |
| 2 | `cargo check --workspace` | **PASS** | No* | No | *crates.io for first lock; offline thereafter. Tauri needs `apps/desktop/dist` |
| 3 | `cargo test --workspace` | **PASS** | No* | No | All member crates |
| 4 | `cargo clippy --workspace --all-targets` | **PASS** | No | No | Warnings only (not fail) |
| 5 | `pnpm install` | **PASS** | Yes (registry) | No | First install |
| 6 | `pnpm install --frozen-lockfile` | **PASS** | No | No | After lockfile commit |
| 7 | `pnpm -r run build` | **PASS** | No | No | event-types, ui, desktop |
| 8 | `pnpm -r run test` | **PASS** | No | No | Includes fake-runtime harness |

\* First `cargo`/`pnpm` may hit package registries; tests themselves do not call provider APIs.

## 2. Per-crate / per-package results

### Rust

| Package | Unit / lib tests | Integration / other | Result |
|---|---|---|---|
| `tracer-domain` | 27 | envelope_roundtrip 14 | **PASS** |
| `tracer-process` | 2 | lifecycle 13 | **PASS** (incl. Windows Job Object) |
| `tracer-storage` | 1 path | foundation 12 + VS-10 1 | **PASS** |
| `tracer-storage-integration` | — | VS-10 1 | **PASS** |
| `tracer-heli` | 6 | status_fixtures 5 + doctest 1 | **PASS** |
| `tracer-desktop` | 0 | — | **PASS** (compiles) |

Approximate Rust test count (non-doc): **~82** automated tests, **0** failed.

### JavaScript / TypeScript

| Package | Tests | Result |
|---|---|---|
| `@tracer/event-types` | 11 | **PASS** |
| `@tracer/ui` | 3 | **PASS** |
| `@tracer/desktop` | 4 | **PASS** |
| `@tracer/contract-fake-runtime` | 30 | **PASS** |
| `@tracer/test-fixtures` | (no test script) | N/A — used by harness |
| `@tracer/fake-acp-runtime` | (no unit script; covered by contract harness) | **PASS** via harness |

## 3. Cross-module risk matrix (required 14)

| # | Risk | Evidence | Status |
|---|---|---|---|
| 1 | process-ready ≠ authenticated / session-ready | `tracer-process` lifecycle: `spawn_emits_started_and_is_process_alive_not_protocol_ready`; `process_event_type_hints_never_ready`; `ReadinessView::may_accept_prompt` requires protocol+session | **PROVEN** |
| 2 | auth errors ≠ protocol errors | `tracer-domain` `ErrorCategory` / `ErrorClass` separation; event-types category tests; storage maps only storage classes | **PROVEN** (type + unit) |
| 3 | permission cancel no deadlock | Fake harness `cancel_while_permission_pending: no deadlock` | **PROVEN** (synthetic ACP) |
| 4 | EOF terminal state | Fake harness `eof_mid_prompt`; process `graceful_stdin_close_exits` | **PROVEN** |
| 5 | crash terminal state | Fake harness `crash_nonzero_exit`; process `nonzero_exit_observed` / force-kill | **PROVEN** |
| 6 | unknown vendor metadata deserializes | Domain envelope unknown type/extension tests; event-types unknown vendor fixture; storage `unknown_event_type_and_payload_preserved`; fake `unknown_vendor_notification` | **PROVEN** |
| 7 | SQLite ordered replay | storage_foundation `ordered_event_replay`, `batch_append_assigns_contiguous_sequences` | **PROVEN** |
| 8 | restart restores session | VS-10 persistence reload (crate + integration); `reload_after_reopen`; reconcile stale running | **PROVEN** |
| 9 | Windows orphan Job Object | `force_kill_reaps_grandchild_no_orphan` (Windows host) | **PROVEN** (platform: Windows) |
| 10 | standard CI no Grok credentials | Fake `no-network` suite; env check shows no GROK/XAI keys; live scenarios excluded from fake | **PROVEN** |
| 11 | synthetic ≠ live labels | expected-events packs refuse live-parity claim; catalog separates standardCi vs live-only | **PROVEN** |
| 12 | missing Heli workspace no crash | `tracer-heli` fixtures `no_workspace` + status tests | **PROVEN** |
| 13 | shell not DB writer | Desktop React has no sql.js/sqlite; Tauri stub has no DB commands; storage writer policy docs + `writer_policy` test | **PROVEN** (design + smoke) |
| 14 | fake runtime without W1-D | Contract harness drives fake over stdio NDJSON with in-harness client only; no adapter crate dependency | **PROVEN** |

## 4. Focused scenario coverage (W1-G catalog / standardCi)

| Scenario ID | Harness | Result |
|---|---|---|
| `happy_prompt_stream` | fake-runtime | PASS |
| `auth_required_session_new` | fake-runtime | PASS |
| `permission_allow` | fake-runtime | PASS |
| `permission_deny` | fake-runtime | PASS |
| `cancel_mid_stream` | fake-runtime | PASS |
| `cancel_while_permission_pending` | fake-runtime | PASS |
| `malformed_frame` | fake-runtime | PASS |
| `unknown_vendor_notification` | fake-runtime | PASS |
| `eof_mid_prompt` | fake-runtime | PASS |
| `crash_nonzero_exit` | fake-runtime | PASS |
| `cancel_unsupported` | fake-runtime | PASS |
| `slow_cancel_ack` | fake-runtime | PASS |
| `duplicate_response_id` | fake-runtime | PASS |
| `capability_minimal` | fake-runtime | PASS |
| `clean_shutdown_stdin_close` | fake-runtime | PASS |

Live-only scenarios: **not executed** (correctly rejected by fake).

## 5. Skips / platform notes

| Item | Handling |
|---|---|
| Unix process isolation specifics | Not run on this Windows integrator host; Windows Job Object path exercised instead |
| Live T6 Grok smoke | Out of Gate 1.1 scope; optional later |
| Full VS-01…VS-14 via Tauri control plane | Deferred to W1-F / Gate 1 |
| Clippy `-D warnings` | Not required; default clippy **PASS** with warnings |

## 6. Reproduction (local, no credentials)

```text
# from repos/tracer on main after Wave 1.1
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
pnpm install --frozen-lockfile
pnpm -r run build
pnpm -r run test
```

If `cargo check -p tracer-desktop` fails on missing `frontendDist`, run `pnpm --filter @tracer/desktop build` first (or ensure `apps/desktop/dist/index.html` exists).
