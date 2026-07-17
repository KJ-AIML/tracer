# Fake ACP scenario driver

## Transport

- JSON-RPC 2.0 **NDJSON** on stdin/stdout
- stderr = logs only
- No ready banner; readiness = successful `initialize` response

## Selection

1. CLI: `--scenario <id>`
2. Env: `TRACER_FAKE_ACP_SCENARIO=<id>`

## Implemented scenarios

| Catalog ID | Wire behavior (summary) | Product expectation (via expected-events) |
|---|---|---|
| `happy_prompt_stream` | init → auth no-op → session/new → prompt stream + tool → `end_turn` | ready → deltas/tools → completed → exit expected |
| `auth_required_session_new` | init ok; `session/new` error `-32000` Authentication required | process ready allowed; **no** session.ready |
| `permission_allow` | `session/request_permission` → client allow → tool completed | approval.requested → resolved allow |
| `permission_deny` | permission reject → tool failed | fail closed; no tool success |
| `cancel_mid_stream` | chunks; `session/cancel` → `stopReason: cancelled` | session.cancelled / terminal status |
| `cancel_while_permission_pending` | open permission; cancel → cancelled (no deadlock) | leave awaiting_approval |
| `malformed_frame` | invalid JSON line mid-stream | `adapter.protocol.error` |
| `unknown_vendor_notification` | `x.ai/unknown_vendor_extension` then continue | `adapter.protocol.unknown` |
| `eof_mid_prompt` | partial chunk; stdout close; no end_turn | no silent session.completed |
| `crash_nonzero_exit` | partial chunk; `exit 1` | failed/disconnected; not running |
| `cancel_unsupported` | caps.cancellation=false; ignore cancel | CapabilityUnsupported + process stop fallback (product) |
| `slow_cancel_ack` | cancel delayed (`--cancel-delay-ms`, default 60s) | force-kill path within T_cancel+T_term |
| `duplicate_response_id` | two responses same id | ProtocolViolation; no double apply |
| `capability_minimal` | minimal caps; single message; no tools/plans | degrade without crash |
| `clean_shutdown_stdin_close` | stdin EOF → exit 0 | `runtime.process.exited` expected true |

## Live-only (rejected)

| ID | Reason |
|---|---|
| `live_stock_auth_prompt` | requires credentials / provider |
| `live_stock_auth_required_reprobe` | optional stock re-probe |

## Fixed identities

| Field | Value |
|---|---|
| sessionId | `11111111-1111-4111-8111-111111111111` |
| promptId | `22222222-2222-4222-8222-222222222222` |
| tool list | `call_example_list` |
| tool edit | `call_example_edit` |
| paths | `{{PROJECT_ROOT}}`, project-relative only |

## Capability profiles

Exposed under `initialize.result.agentCapabilities._meta["tracer/capabilities"]`:

| Profile | promptStreaming | cancellation | tools | plans | approvals |
|---|---|---|---|---|---|
| full | true | true | true | true | true |
| minimal | false | true | false | false | false |
| cancel_unsupported | true | false | true | true | true |

## Process vs session gates

Fake auth is a **no-op** success for CI. Scenario `auth_required_session_new` proves:

1. `initialize` can succeed (`runtime.process.ready` candidate)
2. `session/new` fails with Authentication required (live-scrubbed wire shape)
3. Session is **not** prompt-ready

## Mapping note

This driver emits **ACP wire** frames. Normalized Tracer event types (`agent.message.delta`, `approval.requested`, …) are assigned by the adapter/control plane (W1-B/D/F).  
`packages/test-fixtures` provides `mapWireObservationToProductTypes` as a **test aid only**, not a production normalizer.
