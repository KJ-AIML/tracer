# Live Grok GUI tests — manual / opt-in only

**Tier:** `manual local / Windows GUI / live authenticated`  
**Harness:** `tools/tauri-e2e/live`  
**Work item:** W2.3-B (`tracer-w2-live-gui-validation`)

## Policy

- **Never** run as part of standard CI or `pnpm -r test`.
- Require **explicit operator intent**:
  - env: `TRACER_LIVE_GROK=1` **and** `TRACER_LIVE_GUI=1` **and** `TRACER_LIVE_GUI_AUTHORIZED=1` (W2.4.3-A)
  - CLI: `node tools/tauri-e2e/live/lgj.mjs run` (not dry-run)
- Credentials come from existing local auth only (never commit / print tokens).
- Never fabricate LGJ-05 (approval RR) `PASS` without observed reverse-request.
- If auth missing: classify **`BLOCKED_BY_AUTH`** with evidence; dry-run path still ships.

## How to run

```powershell
# Dry-run (safe; no GUI live spawn, no provider)
node tools/tauri-e2e/live/dry-run.mjs

# Live LGJ suite (requires operator authorization)
$env:TRACER_LIVE_GROK = "1"
$env:TRACER_LIVE_GUI = "1"
$env:TRACER_LIVE_GUI_AUTHORIZED = "1"
node tools/tauri-e2e/live/lgj.mjs run --out target/live-gui/result.json
```

## Scenario map

| ID | Intent |
|---|---|
| LGJ-01 | Live runtime readiness |
| LGJ-02 | Live prompt stream |
| LGJ-03 | Cancel mid-stream |
| LGJ-04 | Restart history (no auto re-prompt) |
| LGJ-05 | Approval RR honesty |
| LGJ-06 | Fail-closed error |
| LGJ-07 | Clean shutdown |

See `docs/modules/w2-3-b/W2_3_B_TEST_MATRIX.md` and `docs/validation/live-grok/LIVE_GUI_RESULTS.md`.
