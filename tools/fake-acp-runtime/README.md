# fake-acp-runtime (W1-G)

Deterministic **fake ACP-compatible** process for Tracer CI and contract harness tests.

## Purpose

- Speak **JSON-RPC 2.0 NDJSON** on stdin/stdout (same transport family as stock Grok ACP stdio).
- Drive **scripted scenarios** from `tests/specifications/scenarios/catalog.yaml`.
- Support **standard CI without network, credentials, or model spend**.
- Separate **synthetic / fake-runtime** evidence from **live** stock Grok parity.

This binary is **not** a production runtime and **must not** call providers.

## Scenario selection

```bash
node tools/fake-acp-runtime/bin/fake-acp-runtime.js --scenario happy_prompt_stream
# or
TRACER_FAKE_ACP_SCENARIO=auth_required_session_new node tools/fake-acp-runtime/bin/fake-acp-runtime.js
```

List scenarios:

```bash
node tools/fake-acp-runtime/bin/fake-acp-runtime.js --list-scenarios
```

## Transport

| Stream | Role |
|---|---|
| stdin | Client → agent requests/notifications |
| stdout | Agent → client responses/notifications |
| stderr | Logs only (never mixed JSON-RPC) |

Readiness = successful `initialize` response (no ready banner). Tracer synthesizes `runtime.process.ready`.

## Implemented catalog IDs

| ID | Behavior |
|---|---|
| `happy_prompt_stream` | init → session → prompt stream + tool → complete |
| `auth_required_session_new` | init ok; `session/new` returns Authentication required |
| `permission_allow` / `permission_deny` | reverse-request permission |
| `cancel_mid_stream` | cancel during agent stream |
| `cancel_while_permission_pending` | cancel with open permission request |
| `malformed_frame` | invalid JSON line mid-stream |
| `unknown_vendor_notification` | unmapped `x.ai/*` notification |
| `eof_mid_prompt` | close stdout mid-prompt |
| `crash_nonzero_exit` | exit 1 mid-run |
| `cancel_unsupported` | cancellation capability false; ignore cancel |
| `slow_cancel_ack` | delay cancel handling (force-kill path) |
| `duplicate_response_id` | duplicate JSON-RPC response id |
| `capability_minimal` | minimal caps; single message complete |
| `clean_shutdown_stdin_close` | exit 0 on stdin EOF |

**Not implemented (live-only):** `live_stock_auth_prompt`, `live_stock_auth_required_reprobe`.

## Evidence provenance

| Label | Meaning for this tool |
|---|---|
| `fake-runtime` | Output produced by this binary |
| `synthetic` | Structural wire shapes (e.g. vendor-unknown scenario) |
| `live-scrubbed` | Gate 0 fixtures only (auth-required wire shape mirrored, not re-captured live) |
| `live-authenticated` | **Never** claimed by this tool |

## Injectable delays

```text
--chunk-delay-ms / TRACER_FAKE_ACP_CHUNK_DELAY_MS
--cancel-delay-ms / TRACER_FAKE_ACP_CANCEL_DELAY_MS
```

`slow_cancel_ack` defaults to a long cancel delay (60s) so process managers exercise force-kill; harness tests inject a shorter delay when only sequencing is required.

## Fixed identities

See `src/constants.js` — stable UUIDs aligned with `tests/fixtures/acp/*`.

## Related

- Specs: `tests/specifications/scenarios/catalog.yaml`
- Expected product events: `tests/specifications/expected-events/`
- Harness: `tests/contract/fake-runtime/`
- Module docs: `docs/modules/w1-g/`
