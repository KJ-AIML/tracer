# Wave 1.2 Test Matrix — Gate 1.2 ACP Adapter

**Gate:** 1.2  
**Task:** `tracer-w1-d-integration`  
**Date:** 2026-07-17  
**Host platform:** Windows (NT 10.0)  
**Tooling:** rustc/cargo 1.96.0 · Node v24.16.0 · pnpm 9.15.0  
**Network / live Grok credentials:** **none** (standard CI path)  
**Live smoke:** **not performed**

## 1. Aggregate commands

| # | Command | Pass/Fail | Network | Creds | Notes |
|---|---|---|---|---|---|
| 1 | `cargo fmt --all --check` | **PASS** | No | No | Clean |
| 2 | `cargo check --workspace` | **PASS** | No* | No | Includes `tracer-acp-client`, `tracer-runtime-adapter` |
| 3 | `cargo test --workspace -- --test-threads=1` | **PASS** | No* | No | All member crates; fake scenarios serialized |
| 4 | `cargo clippy --workspace --all-targets` | **PASS** | No | No | Exit 0; no new W1-D warnings |
| 5 | `pnpm install --frozen-lockfile` | **PASS** | No | No | Lock unchanged |
| 6 | `pnpm -r test` | **PASS** | No | No | JS/TS + fake-runtime harness |
| 7 | `pnpm -r build` | **PASS** | No | No | event-types, ui, desktop |

\* First `cargo` may hit crates.io for cold cache; tests do not call provider APIs.

## 2. Test-count reconciliation

### 2.1 W1-D focused inventory (deterministic)

| Binary / target | Module | Test name | Scenario / concern | Command | Result | CI class |
|---|---|---|---|---|---|---|
| `tracer-acp-client` lib | `codec::tests` | `multi_message_and_partial` | multi-msg + partial frames | `cargo test -p tracer-acp-client` | PASS | unit |
| `tracer-acp-client` lib | `codec::tests` | `malformed_line` | malformed reject | same | PASS | unit |
| `tracer-acp-client` lib | `codec::tests` | `encode_request_round_trip` | codec round-trip | same | PASS | unit |
| `tracer-acp-client` lib | `message::tests` | `classify_request_response_notification` | JSON-RPC kinds | same | PASS | unit |
| `tracer-acp-client` lib | `message::tests` | `structural_reject` | structural reject | same | PASS | unit |
| `tracer-acp-client` lib | `client::tests` | `duplicate_response_detected` | duplicate response id | same | PASS | unit |
| `tracer-acp-client` lib | `state::tests` | `process_ready_not_authenticated_or_session_ready` | readiness boundary | same | PASS | unit |
| `tracer-acp-client` lib | `state::tests` | `session_ready_not_prompt_complete` | readiness boundary | same | PASS | unit |
| `tracer-acp-client` lib | `state::tests` | `invalid_transition_errors` | invalid SM transitions | same | PASS | unit |
| `tracer-acp-client` lib | `state::tests` | `auth_required_blocks_session_ready` | auth gate | same | PASS | unit |
| `tracer-acp-client` integration `codec_transport` | — | `partial_frame_then_complete` | transport partial | same | PASS | integration |
| `tracer-acp-client` integration | — | `multi_msg_single_read` | multi-msg single read | same | PASS | integration |
| `tracer-acp-client` integration | — | `malformed_deterministic_reject` | ProtocolParseError | same | PASS | integration |
| `tracer-acp-client` integration | — | `encode_initialize_shape` | initialize wire shape | same | PASS | integration |
| `tracer-acp-client` integration | — | `readiness_gates_proven` | process≠auth≠session≠prompt | same | PASS | integration |
| `tracer-runtime-adapter` lib | `normalize::tests` | `caps_from_fake_initialize` | initialize caps | `cargo test -p tracer-runtime-adapter --lib` | PASS | unit |
| `tracer-runtime-adapter` lib | `normalize::tests` | `unknown_vendor_maps` | unknown → adapter.protocol.unknown | same | PASS | unit |
| `tracer-runtime-adapter` lib | `normalize::tests` | `permission_request_maps_to_approval_only` | never auto-approve | same | PASS | unit |
| `fake_scenarios` | — | `happy_prompt_stream` | init+session+stream complete | `cargo test -p tracer-runtime-adapter --test fake_scenarios -- --test-threads=1` | PASS | fake-runtime |
| `fake_scenarios` | — | `process_ready_not_session_ready_apis` | process ≠ session ready APIs | same | PASS | fake-runtime |
| `fake_scenarios` | — | `auth_required_no_session_ready` | auth required; no session.ready | same | PASS | fake-runtime |
| `fake_scenarios` | — | `permission_allow` | approval allow | same | PASS | fake-runtime |
| `fake_scenarios` | — | `permission_deny` | approval deny | same | PASS | fake-runtime |
| `fake_scenarios` | — | `cancel_mid_stream` | cancel while streaming | same | PASS | fake-runtime |
| `fake_scenarios` | — | `cancel_while_permission_no_deadlock` | **permission-cancel time-bounded** | same | PASS | fake-runtime / risk |
| `fake_scenarios` | — | `unknown_vendor_no_crash` | unknown vendor safe | same | PASS | fake-runtime |
| `fake_scenarios` | — | `malformed_frame_protocol_error` | malformed → protocol error | same | PASS | fake-runtime |
| `fake_scenarios` | — | `eof_mid_prompt_no_silent_complete` | EOF terminal; no silent complete | same | PASS | fake-runtime |
| `fake_scenarios` | — | `crash_nonzero_exit` | crash terminal | same | PASS | fake-runtime |
| `fake_scenarios` | — | `cancel_unsupported_capability` | CapabilityUnsupported | same | PASS | fake-runtime |
| `fake_scenarios` | — | `duplicate_response_id_protocol_violation` | ProtocolViolation | same | PASS | fake-runtime |
| `fake_scenarios` | — | `capability_minimal` | minimal caps degrade | same | PASS | fake-runtime |
| `fake_scenarios` | — | `clean_shutdown_no_orphan` | clean shutdown / W1-C cleanup | same | PASS | fake-runtime / risk |
| `fake_scenarios` | — | `fresh_session_restart` | restart after shutdown | same | PASS | fake-runtime |
| `fake_scenarios` | — | `error_taxonomy_distinct` | auth vs capability classes | same | PASS | fake-runtime / risk |
| `fake_scenarios` | — | `synthetic_labeling_runtime_kind` | synthetic / acp-stdio label | same | PASS | fake-runtime / risk |
| `fake_scenarios` | — | `sequence_order_monotonic` | sequence order | same | PASS | fake-runtime |
| `fake_scenarios` | — | `fixture_initialize_response_capabilities` | fixture init → caps | same | PASS | fixture / unit-ish |

### 2.2 Count summary

| Aggregation | Count | Composition |
|---|---|---|
| **A. Per-crate (subagent “15+23”)** | 15 + 23 = **38** | acp-client 15; runtime-adapter 3 lib + 20 integration = 23 |
| **B. Runtime-adapter split (“3+20”)** | **23** | lib 3 + `fake_scenarios` 20 only (excludes acp-client) |
| **C. Gate named vertical-slice scenarios** | **20** | `fake_scenarios.rs` tests (fake ACP only) |
| **D. Workspace Rust (approx non-doc)** | **~120** | foundation (~82 from Gate 1.1) + W1-D 38 |

**Why reports looked different:**  
- “15+23” = **full W1-D crate totals**.  
- “3+20” = **runtime-adapter only**, unit vs fake integration.  
Both are correct; they answer different questions. **Gate evidence = named scenarios in §2.1**, not a single integer.

### 2.3 Completion report correction

W1-D completion report claims are **not misleading** once notation is stated:

| Claim | Verdict |
|---|---|
| 15 acp-client | **Accurate** (10 lib + 5 integration) |
| 3 lib + 20 fake_scenarios | **Accurate** |
| 15+23 aggregate | **Accurate** (23 = 3+20) |

No code or report rewrite required beyond this matrix.

## 3. Named vertical-slice scenarios (fake ACP only)

| # | Scenario / test | Covers | Result |
|---|---|---|---|
| 1 | `happy_prompt_stream` | init + session success; stream complete | **PASS** |
| 2 | `process_ready_not_session_ready_apis` | process alive ≠ session ready | **PASS** |
| 3 | `auth_required_no_session_ready` | auth gate; no session.ready | **PASS** |
| 4 | `permission_allow` | approval accept | **PASS** |
| 5 | `permission_deny` | approval reject | **PASS** |
| 6 | `cancel_mid_stream` | cancel while streaming | **PASS** |
| 7 | `cancel_while_permission_no_deadlock` | cancel-while-approval; budget ≤ 5s; no deadlock | **PASS** |
| 8 | `unknown_vendor_no_crash` | unknown events safe | **PASS** |
| 9 | `malformed_frame_protocol_error` | protocol error path | **PASS** |
| 10 | `eof_mid_prompt_no_silent_complete` | EOF mid-prompt terminal | **PASS** |
| 11 | `crash_nonzero_exit` | crash terminal | **PASS** |
| 12 | `cancel_unsupported_capability` | CapabilityUnsupported | **PASS** |
| 13 | `duplicate_response_id_protocol_violation` | duplicate id | **PASS** |
| 14 | `capability_minimal` | minimal capability degrade | **PASS** |
| 15 | `clean_shutdown_no_orphan` | clean shutdown / orphan path via W1-C | **PASS** |
| 16 | `fresh_session_restart` | restart after shutdown | **PASS** |
| 17 | `error_taxonomy_distinct` | distinct error classes | **PASS** |
| 18 | `synthetic_labeling_runtime_kind` | synthetic labeling | **PASS** |
| 19 | `sequence_order_monotonic` | monotonic sequences | **PASS** |
| 20 | `fixture_initialize_response_capabilities` | fixture initialize → caps | **PASS** |

Supporting SM/codec proofs (acp-client 15) and normalize unit tests (3) reinforce readiness boundaries and mapping without live network.

## 4. Mandatory risk tests

| Risk | Test | Result |
|---|---|---|
| Permission-cancel deadlock (time-bounded) | `cancel_while_permission_no_deadlock` (`PERMISSION_CANCEL_DEADLOCK_BUDGET` = 5s) | **PASS** |
| Process vs session readiness | `process_ready_not_session_ready_apis` + SM unit tests | **PASS** |
| Orphan cleanup via W1-C | `clean_shutdown_no_orphan` + process `force_kill_reaps_grandchild_no_orphan` | **PASS** |
| Synthetic labeling | `synthetic_labeling_runtime_kind` | **PASS** |
| Error taxonomy distinct | `error_taxonomy_distinct` | **PASS** |
| Unknown events no crash | `unknown_vendor_no_crash` | **PASS** |
| EOF terminal | `eof_mid_prompt_no_silent_complete` | **PASS** |
| Crash terminal | `crash_nonzero_exit` | **PASS** |

## 5. Foundation regression (Gate 1.1 surface)

### Rust (workspace)

| Package | Tests (this run) | Result |
|---|---|---|
| `tracer-domain` | 27 lib + 14 envelope_roundtrip | **PASS** |
| `tracer-process` | 2 lib + 13 lifecycle | **PASS** (incl. Windows Job Object) |
| `tracer-storage` | 1 path + 12 foundation + 1 VS-10 | **PASS** |
| `tracer-storage-integration` | 1 VS-10 | **PASS** |
| `tracer-heli` | 6 lib + 5 fixtures + 1 doctest | **PASS** |
| `tracer-acp-client` | 10 lib + 5 integration | **PASS** |
| `tracer-runtime-adapter` | 3 lib + 20 fake_scenarios | **PASS** |
| `tracer-desktop` | 0 (compiles) | **PASS** |

### JavaScript / TypeScript

| Package | Tests | Result |
|---|---|---|
| `@tracer/event-types` | 11 | **PASS** |
| `@tracer/ui` | 3 | **PASS** |
| `@tracer/desktop` | 4 | **PASS** |
| `@tracer/contract-fake-runtime` | 30 | **PASS** |

## 6. Explicit non-tests (standard CI)

| Item | Status |
|---|---|
| Live Grok authenticated session | **Not run** |
| Network / provider credentials | **Not used** |
| Paid usage | **Not used** |
| `TRACER_LIVE_SMOKE` | **Unset** |

## 7. Evidence class legend

| Label | Meaning |
|---|---|
| unit | In-process crate unit tests |
| integration | Crate integration binary without OS agent spawn (codec buffers) |
| fake-runtime | Driven by W1-G `fake-acp-runtime` over real process pipes |
| fixture | Static ACP fixture parse |
| risk | Mandatory Gate risk |
| live-smoke | Optional; not a gate (not performed) |

## 8. Clippy inheritance note

| Area | Warnings | Gate impact |
|---|---|---|
| W1-D crates (`tracer-acp-client`, `tracer-runtime-adapter`) | **None observed** | Clean |
| Inherited Gate 1.1 | domain `new_without_default` / `too_many_arguments` / `should_implement_trait`; process `manual_ok_err`; storage `wrong_self_convention` | Documented; not treated as Gate 1.2 fail |
