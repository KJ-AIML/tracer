# W1-D Test Matrix

## Commands

```powershell
cargo test -p tracer-acp-client
cargo test -p tracer-runtime-adapter -- --test-threads=1
cargo fmt --all --check
cargo check --workspace
cargo clippy --workspace --all-targets
```

## Codec / SM (`tracer-acp-client`)

| Test | Covers |
|---|---|
| partial + multi-msg frames | transport edge cases |
| malformed deterministic reject | ProtocolParseError taxonomy |
| readiness gates | process ≠ auth ≠ session ≠ prompt-complete |
| duplicate response id | client correlation |

## Fake scenarios (`fake_scenarios.rs`)

| Scenario / test | Covers |
|---|---|
| `happy_prompt_stream` | init+session success; stream complete |
| `auth_required_session_new` | auth required/failed; no session.ready |
| `permission_allow` / `permission_deny` | approval accept/reject |
| `cancel_mid_stream` | cancel while streaming |
| `cancel_while_permission_pending` | **permission-cancel deadlock** time-bounded |
| `malformed_frame` | protocol error; continue |
| `unknown_vendor_notification` | unknown events no crash |
| `eof_mid_prompt` | no silent complete |
| `crash_nonzero_exit` | crash path |
| `cancel_unsupported` | CapabilityUnsupported |
| `duplicate_response_id` | ProtocolViolation |
| `capability_minimal` | unsupported capability degrade |
| `clean_shutdown_stdin_close` | orphan cleanup / expected exit |
| `fresh_session_restart` | restart after shutdown |
| process_ready_not_session_ready_apis | readiness APIs |
| error_taxonomy_distinct | auth vs capability classes |
| synthetic_labeling_runtime_kind | acp-stdio metadata |
| sequence_order_monotonic | sequence order |

## Risk tests (mandatory)

| Risk | Test |
|---|---|
| Permission-cancel deadlock | `cancel_while_permission_no_deadlock` |
| Process vs session readiness | `process_ready_not_session_ready_apis`, SM unit tests |
| Orphan cleanup via W1-C | `clean_shutdown_no_orphan`, Drop/force_kill |
| Synthetic labeling | `synthetic_labeling_runtime_kind` |
| Error taxonomy distinct | `error_taxonomy_distinct` |
| Unknown events no crash | `unknown_vendor_no_crash` |

## Explicit non-tests (standard CI)

- Live Grok authenticated session
- Network / provider credentials
- Paid usage

## Evidence class

| Label | Meaning |
|---|---|
| fake-runtime | Driven by W1-G fake ACP |
| synthetic | Fixture / vendor-unknown synthetic wire |
| live-smoke | Optional; not a gate |

## Platform

Recorded on **Windows** (host grok-build). Process isolation: Job Object (W1-C).
