# live-grok-smoke (VS1-H1)

Manual, **opt-in** live validation harness for the stock Grok ACP stdio path:

```text
grok agent --no-leader stdio
```

**Classification:** `manual local / live authenticated smoke`  
**Not part of standard CI.** Never stores credentials. Never prints auth tokens.

## Opt-in

| Mode | Command | Env | Spawns Grok? | Provider usage? |
|---|---|---|---|---|
| Dry-run | `cargo run -p live-grok-smoke -- dry-run` | none | No | No |
| Discover | `cargo run -p live-grok-smoke -- discover` | none | Version probe only | No |
| Live run | `cargo run -p live-grok-smoke -- run` | **`TRACER_LIVE_GROK=1`** (or `TRACER_LIVE_SMOKE=1`) | Yes | Possible |

## Stages

1. binary discovery  
2. process startup  
3. protocol initialization  
4. authentication requirement  
5. authenticated session creation  
6. prompt submission  
7. streaming  
8. approval if requested  
9. cancellation  
10. shutdown  

## Scenarios (LVS)

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

## Examples

```powershell
# Safe construction check (CI-friendly unit tests also cover this)
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- dry-run --out target/live-grok-smoke/dry-run.json

# Live unauthenticated probe (stops / blocks at session auth gate)
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- run --through session --allow-unauth --out target/live-grok-smoke/live.json

# Full live authenticated smoke (requires operator-provided auth in GROK_HOME / stock login)
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- run --out target/live-grok-smoke/live-full.json
```

## Product APIs reused

- `tracer_runtime_adapter::grok_stdio_spawn_config` / `grok_stdio_args`
- `RuntimeAdapter::start` → `initialize` → `create_session` → `submit_prompt` / `cancel_prompt` → `shutdown`

Sources of truth: W0-B research under `docs/research/grok-build/`, W1-D public interface under `docs/modules/w1-d/`.

## Auth unavailable

If `session/new` returns authentication required:

- Overall classification: **`BLOCKED_BY_AUTH`**
- Dry-run + unauthenticated process evidence preserved
- **Do not** claim live authenticated parity
- **Do not** fail the fake-runtime vertical slice

## Evidence

JSON reports are sanitized (tokens, user path segments, secret-looking keys redacted).  
Write with `--out <path>`; do not commit private evidence files.
