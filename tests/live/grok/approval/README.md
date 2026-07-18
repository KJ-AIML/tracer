# Live Grok approval tests (W2-D / LVA) — manual / opt-in only

**Tier:** `manual local / live authenticated smoke` (TEST_STRATEGY T6)  
**Harness:** `tools/live-grok-smoke`  
**Work item:** W2-D  
**Scenarios:** LVA-01 … LVA-07  

## Policy

- **Never** run as part of standard CI (`tests/specifications/ci/matrix.yaml` forbids `live` on standard jobs).
- Require **explicit operator intent**:
  - env: `TRACER_LIVE_GROK=1` (alias `TRACER_LIVE_SMOKE=1`)
  - CLI subcommand: **`approval-run`** (not `approval-dry-run`, not LVS `run`)
- **Never auto-approve** without an LVA scenario action (allow-once / reject-once).
- **Never claim PASS** without an observed `approval.requested` reverse-request for RR-dependent scenarios.
- Never commit credentials, private prompts, or unsanitized protocol captures.

## Classifications

| Status | Use when |
|---|---|
| `PASS` | Criteria met with observation |
| `NOT_OBSERVED` | Live ran; required signal (e.g. RR) not seen |
| `BLOCKED_BY_AUTH` | Auth gate blocked session/prompt |
| `UNSUPPORTED_BY_PROMPT` | Prompt completed without permission reverse-request |
| `FAIL` | Unexpected non-auth failure |
| `NOT_RUN` | Dry-run only |

## How to run

From the Tracer repo root (this worktree):

```powershell
# CI-safe construction + unit tests (no agent stdio)
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- approval-dry-run `
  --out target/live-grok-smoke/approval-dry-run.json

# Live LVA suite (may consume provider usage)
$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke -- approval-run `
  --out target/live-grok-smoke/approval-live.json
```

## Scenario map

| Scenario | Intent | Harness action |
|---|---|---|
| LVA-01 | reverse-request observed | induce tool permission; observe `approval.requested` |
| LVA-02 | accept once | `resolve_approval` allow / allow-once |
| LVA-03 | reject once | `resolve_approval` deny / reject-once |
| LVA-04 | cancel while pending | `cancel_prompt` after RR |
| LVA-05 | no deadlock | control returns within budget after cancel/resolve |
| LVA-06 | terminal state | session completed / cancelled / failed |
| LVA-07 | clean shutdown | no orphan process |

## CI note

`cargo test -p live-grok-smoke` only exercises **dry-run / sanitize / classification** unit tests.  
It must not spawn `grok agent stdio`, resolve live approvals, or require credentials.

## See also

- `docs/validation/live-grok/LIVE_APPROVAL_VALIDATION.md`
- `docs/modules/w2-d/W2_D_COMPLETION_REPORT.md`
- `tools/live-grok-smoke/README.md`
- `tests/live/grok/README.md` (LVS smoke)
