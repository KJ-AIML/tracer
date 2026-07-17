# Live Grok tests — manual / opt-in only

**Tier:** `manual local / live authenticated smoke` (TEST_STRATEGY T6)  
**Harness:** `tools/live-grok-smoke`  
**Work item:** VS1-H1  

## Policy

- **Never** run as part of standard CI (`tests/specifications/ci/matrix.yaml` forbids `live` on standard jobs).
- Require **explicit operator intent**:
  - env: `TRACER_LIVE_GROK=1` (alias `TRACER_LIVE_SMOKE=1`)
  - CLI subcommand: `run` (not `dry-run`)
- Never commit credentials, private prompts, or unsanitized protocol captures.

## How to run

From the Tracer repo root (this worktree):

```powershell
# Dry-run (safe; no live spawn of agent stdio)
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- dry-run

# Live (may consume provider usage when auth is present)
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- run --allow-unauth --out target/live-grok-smoke/result.json
```

See also:

- `docs/validation/live-grok/LIVE_GROK_SMOKE_PLAN.md`
- `docs/validation/live-grok/LIVE_GROK_SMOKE_RESULT.md`
- `tools/live-grok-smoke/README.md`

## Scenario map

| Scenario | Harness stage(s) |
|---|---|
| LVS-01 runtime process starts | startup |
| LVS-02 protocol initialize succeeds | initialize |
| LVS-03 authentication state identified | auth_requirement |
| LVS-04 session creation succeeds | session |
| LVS-05 prompt streams normalized event | prompt + stream |
| LVS-06 completion / terminal result | prompt / stream terminal |
| LVS-07 cancellation does not deadlock | cancel |
| LVS-08 shutdown leaves no orphan | shutdown |

## CI note

`cargo test -p live-grok-smoke` only exercises **dry-run / sanitize / discovery construction** unit tests.  
It must not spawn `grok agent stdio` or require credentials.
