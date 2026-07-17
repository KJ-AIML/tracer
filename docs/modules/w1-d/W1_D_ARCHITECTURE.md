# W1-D Architecture — ACP Client and Runtime Adapter

**Task:** `tracer-w1-acp-adapter`  
**Status:** implemented (Wave 1.2)

## 1. Layering (required — do not collapse)

```text
ACP transport (NDJSON JSON-RPC on stdio)
  → ACP message codec
  → session protocol state machine
  → runtime adapter
  → normalized Tracer domain events
```

| Layer | Crate / module | Responsibility |
|---|---|---|
| Transport | `tracer-acp-client::transport` + process pipes | stdin write, stdout read; partial / multi-frame; EOF |
| Codec | `tracer-acp-client::codec` / `message` | serialize outbound; deserialize inbound; structural reject |
| Session SM | `tracer-acp-client::state` | protocol phases; invalid transitions → typed errors |
| Runtime adapter | `tracer-runtime-adapter` | compose process + client; normalize; public API for W1-F |
| Domain events | `tracer-domain` (dependency) | envelopes, IDs, sequences, caps, error classes |

## 2. Process ownership

- **Sole process manager:** `tracer-process` (W1-C).
- Adapter calls `ProcessManager::spawn` / `ManagedProcess::{write_stdin,take_stdout,stop,kill_force}`.
- No second process manager. Orphan prevention = Job Object / process group from W1-C.
- stderr is process diagnostics → `runtime.process.stderr`; never ACP parse.

## 3. Transport rules (W0-B)

| Direction | Stream | Content |
|---|---|---|
| Client → agent | stdin | NDJSON JSON-RPC requests / notifications / permission responses |
| Agent → client | stdout | NDJSON JSON-RPC responses / notifications / reverse-requests |
| Agent logs | stderr | text only |

Framing: newline-delimited JSON-RPC 2.0.  
Reader thread + `FrameDecoder` handles partial reads and multiple messages per read.

## 4. Session protocol state machine

Phases include: process unavailable/starting/alive; initializing; protocol ready; authentication required/failed; creating session; session ready; prompting; streaming; awaiting approval; cancelling/cancelled; completed; failed; disconnected; runtime crashed.

### Proven distinctions

| Claim | API |
|---|---|
| process-alive ≠ protocol-ready | `is_process_alive()` vs `is_protocol_ready()` |
| process-ready ≠ authenticated | `AuthenticationState` independent of protocol ready |
| process-ready ≠ session-ready | `is_session_ready()` after `session/new` only |
| session-ready ≠ prompt-complete | `may_accept_prompt()` false while prompt active |

## 5. Normalization duties

- ACP `session/update` → `agent.message.delta`, `tool.*`, `agent.plan.updated`, …
- `session/request_permission` → **`approval.requested` only** (never auto-approve)
- Permission answers → wire response + `approval.resolved`
- Unknown vendor (`x.ai/*`) → `adapter.protocol.unknown` (no crash)
- Malformed / duplicate id → `adapter.protocol.error` (`ProtocolParseError` / `ProtocolViolation`)
- Terminal EOF / crash → `session.failed` / `runtime.process.exited` as appropriate

React / W1-F never parse raw Grok or ACP frames.

## 6. Grok boundary

Stock invocation (path-portable):

```text
grok agent --no-leader stdio
```

Helper: `grok_stdio_spawn_config(executable, cwd)`.  
Discovery of `grok` on PATH is control-plane / config owned.

**Separated steps:** discovery → process start → initialize → auth → session/new → prompt.  
Live Windows authenticated session creation is **not** assumed proven for CI.

## 7. Fake runtime (primary CI)

Driven via `fake_acp_spawn_config(node, fake-acp-runtime.js, scenario_id, cwd)`.  
Scenario catalog: W1-G `tools/fake-acp-runtime`.

## 8. Threading

- Reader thread: stdout → frames → dispatch
- Writer thread: channel → process stdin
- Public methods mostly `&self` so cancel/approval concurrent with blocking `submit_prompt`

## 9. Explicit non-goals

- No SQLite writes (control plane sole writer)
- No UI / Tauri commands (W1-F)
- No forking Grok Build
- No live Grok as standard CI gate
