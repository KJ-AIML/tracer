# live-grok-smoke (VS1-H1 + W2-D)

Manual, **opt-in** live validation harness for the stock Grok ACP stdio path:

```text
grok agent --no-leader stdio
```

**Classification:** `manual local / live authenticated smoke`  
**Not part of standard CI.** Never stores credentials. Never prints auth tokens.  
Never auto-approves without an explicit LVA scenario action.

## Opt-in

| Mode | Command | Env | Spawns Grok? | Provider usage? |
|---|---|---|---|---|
| Dry-run (LVS) | `cargo run -p live-grok-smoke -- dry-run` | none | No | No |
| Discover | `cargo run -p live-grok-smoke -- discover` | none | Version probe only | No |
| Live LVS run | `cargo run -p live-grok-smoke -- run` | **`TRACER_LIVE_GROK=1`** | Yes | Possible |
| Approval dry-run (LVA) | `cargo run -p live-grok-smoke -- approval-dry-run` | none | No | No |
| Live approval (LVA) | `cargo run -p live-grok-smoke -- approval-run` | **`TRACER_LIVE_GROK=1`** | Yes | Possible |

Alias: `TRACER_LIVE_SMOKE=1` is accepted for the live env gate.

## Stages

1. binary discovery  
2. process startup  
3. protocol initialization  
4. authentication requirement  
5. authenticated session creation  
6. prompt submission  
7. streaming  
8. approval if requested (observe; resolve only on LVA-02/03 actions)  
9. cancellation  
10. shutdown  

## Scenarios

### LVS (VS1-H1 smoke) — `run` / `dry-run`

| ID | Meaning |
|---|---|
| LVS-01 | runtime process starts |
| LVS-02 | protocol initialize succeeds |
| LVS-03 | authentication state identified correctly |
| LVS-04 | session creation succeeds |
| LVS-05 | prompt streams at least one normalized event |
| LVS-06 | completion or controlled terminal result |
| LVS-07 | cancellation does not deadlock |
| LVS-08 | runtime shutdown leaves no orphan process |

### LVA (W2-D approval) — `approval-run` / `approval-dry-run`

| ID | Meaning |
|---|---|
| LVA-01 | approval reverse-request observed (`approval.requested`) |
| LVA-02 | accept once (allow-once) after RR |
| LVA-03 | reject once (reject-once) after RR |
| LVA-04 | cancel while approval pending |
| LVA-05 | no deadlock (control returns within budget) |
| LVA-06 | terminal session state observed |
| LVA-07 | shutdown leaves no orphan process |

LVA honesty classifications: `PASS` | `NOT_OBSERVED` | `BLOCKED_BY_AUTH` | `UNSUPPORTED_BY_PROMPT` | `FAIL` | `NOT_RUN` | `PARTIAL`.  
**Never claim LVA PASS without observed `approval.requested` for reverse-request-dependent scenarios.**

## Examples

```powershell
# Safe construction check (CI-friendly unit tests also cover this)
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- dry-run --out target/live-grok-smoke/dry-run.json
cargo run -p live-grok-smoke -- approval-dry-run --out target/live-grok-smoke/approval-dry-run.json

# Live unauthenticated LVS probe (stops / blocks at session auth gate)
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- run --through session --allow-unauth --out target/live-grok-smoke/live.json

# Full live authenticated LVS smoke
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- run --out target/live-grok-smoke/live-full.json

# Live LVA approval reverse-request suite
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- approval-run --out target/live-grok-smoke/approval-live.json
```

## Product APIs reused

- `tracer_runtime_adapter::grok_stdio_spawn_config` / `grok_stdio_args`
- `RuntimeAdapter::start` → `initialize` → `create_session` → `submit_prompt` / `cancel_prompt` / `resolve_approval` → `shutdown`

Sources of truth: W0-B research under `docs/research/grok-build/`, W1-D public interface under `docs/modules/w1-d/`.

## Auth unavailable

If `session/new` returns authentication required:

- Overall classification: **`BLOCKED_BY_AUTH`**
- Dry-run + unauthenticated process evidence preserved
- **Do not** claim live authenticated parity or LVA reverse-request PASS
- **Do not** fail the fake-runtime vertical slice

## Evidence

JSON reports are sanitized (tokens, user path segments, secret-looking keys redacted).  
Write with `--out <path>`; do not commit private evidence files.

## Docs

- LVS plan/result: `docs/validation/live-grok/LIVE_GROK_SMOKE_*.md`
- LVA validation: `docs/validation/live-grok/LIVE_APPROVAL_VALIDATION.md`
- W2-D completion: `docs/modules/w2-d/W2_D_COMPLETION_REPORT.md`
