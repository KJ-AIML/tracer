# Live Grok GUI (W2.3-B / LGJ)

**Task:** `tracer-w2-live-gui-validation`  
**Classification:** `manual_local_live_authenticated_gui`  
**Standard CI:** **excluded** (never part of `pnpm -r test`)

## Opt-in

| Mode | Command | Env | Spawns Grok? | Launches GUI? | Provider? |
|---|---|---|---|---|---|
| Unit | `node tools/tauri-e2e/live/unit.mjs` | none | No | No | No |
| Dry-run | `node tools/tauri-e2e/live/dry-run.mjs` | none | No | No | No |
| Live | `node tools/tauri-e2e/live/lgj.mjs run` | **`TRACER_LIVE_GROK=1`** + **`TRACER_LIVE_GUI=1`** | Yes (via bridge) | Yes | Possible |

## Scenarios (LGJ-01…07)

| ID | Meaning | Honesty notes |
|---|---|---|
| LGJ-01 | Live runtime readiness | `BLOCKED_BY_AUTH` if session/auth gate blocks |
| LGJ-02 | Live prompt stream | ≥1 timeline event |
| LGJ-03 | Cancel mid-stream | No deadlock budget |
| LGJ-04 | Restart history (no auto re-prompt) | Same temp DB; must not auto re-submit |
| LGJ-05 | Approval reverse-request | `PASS` only if RR observed; else `NOT_OBSERVED` / `UNSUPPORTED` — **never fabricate** |
| LGJ-06 | Fail-closed error | Invalid path; stay tauri; no mock |
| LGJ-07 | Clean shutdown | No orphans (`tracer-desktop`, drivers, `grok`) |

## Live bridge (minimal test-only launch config)

Product control plane spawns `node <TRACER_FAKE_ACP_JS> --scenario <id>`.  
For live GUI only, the harness sets `TRACER_FAKE_ACP_JS` to:

```text
tools/tauri-e2e/live/launch-live-grok.mjs
```

which bridges stdio to stock:

```text
grok agent --no-leader stdio
```

This is **not** a product surface and does not redesign CP/UI.

## Operator runbook

```powershell
# From tracer worktree root

# Safe construction check
node tools/tauri-e2e/live/dry-run.mjs --out target/live-gui/dry-run.json

# Live (may consume provider usage; needs local Grok auth)
$env:TRACER_LIVE_GROK = "1"
$env:TRACER_LIVE_GUI = "1"
node tools/tauri-e2e/live/lgj.mjs run --skip-build --out target/live-gui/live.json

# Subset
node tools/tauri-e2e/live/lgj.mjs run --journey LGJ-01,LGJ-06,LGJ-07 --skip-build
```

## Safety

1. Dual gate: env pair + explicit `run`/`--live`
2. Print operation class before live path
3. Never print credentials/tokens
4. Public-safe bounded prompts only; secret-looking `--prompt` rejected
5. Sanitize artifacts (tokens, user path segments)
6. Never claim LGJ-05 PASS without observed reverse-request
7. If auth missing: `BLOCKED_BY_AUTH` with evidence; dry-run still ships

## Related

- L3-J (fake ACP): `tools/tauri-e2e/l3j-gui.mjs`
- Adapter live smoke: `tools/live-grok-smoke`
- Docs: `docs/modules/w2-3-b/`, `docs/validation/live-grok/LIVE_GUI_RESULTS.md`
