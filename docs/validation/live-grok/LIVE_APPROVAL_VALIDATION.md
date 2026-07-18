# Live Approval Validation (W2-D / LVA)

**Work item:** W2-D  
**Task:** `tracer-w2-live-approval-validation`  
**Harness:** `tools/live-grok-smoke` (`approval-dry-run` / `approval-run`)  
**Classification tier:** `manual_local_live_authenticated_smoke`  
**Standard CI:** **excluded** (opt-in only; never enable live Grok in standard CI)

## 1. Purpose

Extend the VS1-H1 live Grok smoke harness to validate **approval reverse-request** and **cancel-around-approval** behavior on the stock path:

```text
grok agent --no-leader stdio
```

This is **not** a control-plane or ACP redesign. The harness reuses W1-D
`tracer-runtime-adapter` public APIs only:

- `grok_stdio_spawn_config` / `RuntimeAdapter::start`
- `initialize` → `create_session` → `submit_prompt`
- observe `approval.requested` (normalized from `session/request_permission`)
- explicit `resolve_approval` (allow-once / reject-once) **only** as scenario actions
- `cancel_prompt` while an approval may be pending
- `shutdown` (no orphan)

## 2. Safety constraints

1. **Opt-in only:** live suite requires `TRACER_LIVE_GROK=1` (alias `TRACER_LIVE_SMOKE=1`) **and** the `approval-run` subcommand.
2. **Dry-run is CI-safe:** `approval-dry-run` never launches agent stdio and never claims live PASS.
3. **Never auto-approve:** allow/deny only via explicit LVA scenario actions (LVA-02 / LVA-03). Observe/cancel paths never call `resolve_approval` with allow.
4. **Never fabricate PASS:** if `approval.requested` was not observed, reverse-request-dependent scenarios are `NOT_OBSERVED` or `UNSUPPORTED_BY_PROMPT` — never PASS.
5. Credentials / tokens are never printed or committed. Prompts must be public-safe (default inducing prompt is public).
6. Evidence JSON is sanitized (tokens, secret-looking keys, user path segments).

## 3. Operator runbook

```powershell
# From tracer worktree root

# Unit tests (dry-run / classification only; no agent stdio)
cargo test -p live-grok-smoke

# Approval suite plan validation (safe)
cargo run -p live-grok-smoke -- approval-dry-run `
  --out target/live-grok-smoke/approval-dry-run.json

# Live LVA suite (may consume provider usage when auth is present)
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- approval-run `
  --out target/live-grok-smoke/approval-live.json

# Subset
cargo run -p live-grok-smoke -- approval-run --scenarios LVA-01,LVA-04,LVA-05

# Override inducing prompt (public-safe only)
cargo run -p live-grok-smoke -- approval-run `
  --prompt "Create a small text file named probe.txt with the word probe, then stop."
```

Without `TRACER_LIVE_GROK=1`, `approval-run` exits with an error and instructs the operator to use `approval-dry-run`.

## 4. Scenarios (LVA-01…LVA-07)

| ID | Intent | PASS requires |
|---|---|---|
| **LVA-01** | Reverse-request observed | `approval.requested` seen for inducing prompt |
| **LVA-02** | Accept once | RR observed **and** `resolve_approval(allow / allow-once)` Ok |
| **LVA-03** | Reject once | RR observed **and** `resolve_approval(deny / reject-once)` Ok |
| **LVA-04** | Cancel while approval pending | RR observed **and** `cancel_prompt` returns (or capability-unsupported mapped honestly) |
| **LVA-05** | No deadlock | Prompt/control returns; cancel/resolve path itself does not hang beyond budget |
| **LVA-06** | Terminal session state | `session.completed` / `session.cancelled` / `session.failed` observed |
| **LVA-07** | Clean shutdown | Process not alive after shutdown/force (no orphan) |

Default inducing prompt (public-safe):

```text
Create a new text file named tracer-lva-probe.txt in the current working directory
containing the single word probe, then stop. Use a file tool if available.
```

Operators may override via `--prompt`. Private / secret-looking prompts are rejected.

## 5. Classification vocabulary

| Classification | Meaning |
|---|---|
| `PASS` | Scenario criteria met **with observation** (for RR paths: `approval.requested` seen) |
| `NOT_OBSERVED` | Live ran but reverse-request (or required signal) was not seen within wait budget |
| `BLOCKED_BY_AUTH` | Session/prompt path blocked on authentication |
| `UNSUPPORTED_BY_PROMPT` | Provider completed the inducing prompt **without** a permission reverse-request (prompt/tool policy did not surface RR) |
| `FAIL` | Unexpected product/runtime failure (spawn crash, resolve failed after RR, orphan, deadlock) |
| `NOT_RUN` | Dry-run / plan only (construction validated; live not executed) |
| `PARTIAL` | Mixed outcomes (some PASS, some NOT_OBSERVED / UNSUPPORTED, no FAIL) |

**Honesty rule:** overall suite `PASS` only when **all** LVA-01…LVA-07 are `PASS`. Non-observation is never upgraded to PASS.

## 6. Stage plan (approval suite)

| Stage | Dry-run | Live |
|---|---|---|
| discovery | version probe / plan | locate binary |
| startup | spawn plan via `grok_stdio_spawn_config` (not launched) | `RuntimeAdapter::start` |
| initialize | not launched | ACP initialize |
| auth_requirement | not launched | inspect auth methods/state (no tokens) |
| session | not launched | `create_session` or `BLOCKED_BY_AUTH` |
| prompt | not launched | approval-inducing `submit_prompt` attempts |
| stream | not launched | drain normalized event types |
| approval | not launched | observe RR; resolve only on LVA-02/03 actions |
| cancel | not launched | cancel-while-pending path (LVA-04/05) |
| shutdown | not launched | graceful + force; orphan check (LVA-07) |

## 7. Dry-run evidence (CI-safe)

```text
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- approval-dry-run
```

Expected dry-run report fields:

| Field | Expected |
|---|---|
| `workItem` | `W2-D` |
| `suite` | `lva` |
| `dryRun` | `true` |
| `classification` | `NOT_RUN` |
| `scenarios[].id` | LVA-01…LVA-07 each `NOT_RUN` |
| `spawnPlan.args` | `["agent","--no-leader","stdio"]` |
| `spawnPlan.matchesW0bW1d` | `true` |
| `spawnPlan.productHelper` | `tracer_runtime_adapter::grok_stdio_spawn_config` |

Local artifact (not committed): `target/live-grok-smoke/approval-dry-run.json`.

## 8. Live results (authoring host)

**Date:** 2026-07-18  
**Platform:** windows-x86_64  
**Runtime:** `grok 0.2.103 (89c3d36fb6)`  
**Policy:** Do not fabricate live PASS. Optional live only if operator auth present.

| Mode | Classification | Notes |
|---|---|---|
| Unit tests + `approval-dry-run` | **`NOT_RUN`** (construction Pass) | Always green without credentials |
| Live `approval-run` | **`PARTIAL`** | Auth + session ok; RR not observed |

### 8.1 Live scenario matrix (this host)

| ID | Status | Detail |
|---|---|---|
| LVA-01 | **NOT_OBSERVED** | `approval.requested` not seen within wait budget |
| LVA-02 | **NOT_OBSERVED** | cannot exercise allow-once without RR |
| LVA-03 | **NOT_OBSERVED** | cannot exercise reject-once without RR |
| LVA-04 | **NOT_OBSERVED** | cannot exercise cancel-while-pending without RR |
| LVA-05 | **PASS** | no deadlock; control returned within budget |
| LVA-06 | **PASS** | terminal `session.cancelled` observed |
| LVA-07 | **PASS** | shutdown leaves no orphan |

**Caps advertised:** `approvals=true`, `cancellation=true`.  
**Auth:** session created successfully (tokens never printed).  
**Evidence (local only):** `target/live-grok-smoke/approval-live.json` (sanitized; not committed).

When live is not executed on a host (no opt-in, no binary, or no auth):

- Document classification as `NOT_RUN`, `BLOCKED_BY_AUTH`, `NOT_OBSERVED`, or `UNSUPPORTED_BY_PROMPT` as appropriate.
- **Do not** claim LVA reverse-request parity without observed `approval.requested`.
- Fake-runtime vertical slice remains unaffected.

## 9. Product assumptions checked

| ID | Source | Statement |
|---|---|---|
| A-W0B-01 | W0-B process lifecycle | Stock start is `grok agent --no-leader stdio` |
| A-W1D-01 | W1-D spawn helper | Product helper emits that argv (no reimplementation) |
| A-W1D-02 | W1-D public interface | process alive ≠ protocol ready ≠ session ready |
| A-W1D-03 | W1-D normalize | `session/request_permission` → `approval.requested` (never auto-approve) |
| A-W1D-04 | W1-D adapter | `resolve_approval` / `cancel_prompt` are explicit operator/CP actions |

## 10. Out of scope

- Enabling live Grok in standard CI  
- Control-plane / ACP / process-manager redesign  
- Desktop product work  
- Committing secrets, tokens, private prompts, or unsanitized captures  

## 11. Related docs

- Plan (LVS smoke): `docs/validation/live-grok/LIVE_GROK_SMOKE_PLAN.md`
- Result (LVS smoke): `docs/validation/live-grok/LIVE_GROK_SMOKE_RESULT.md`
- Completion: `docs/modules/w2-d/W2_D_COMPLETION_REPORT.md`
- Harness README: `tools/live-grok-smoke/README.md`
- Manual test policy: `tests/live/grok/README.md`, `tests/live/grok/approval/README.md`
