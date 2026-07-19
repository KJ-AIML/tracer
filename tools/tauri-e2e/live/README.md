# Live Grok GUI (W2.3-B / W2.4.3-A / LGJ)

**Task:** `tracer-w2-live-gui-execution`  
**Classification:** `manual_local_live_authenticated_gui`  
**Standard CI:** **excluded** (never part of `pnpm -r test`)

## Opt-in

| Mode | Command | Env | Spawns Grok? | Launches GUI? | Provider? |
|---|---|---|---|---|---|
| Unit | `node tools/tauri-e2e/live/unit.mjs` | none | No | No | No |
| Dry-run | `node tools/tauri-e2e/live/dry-run.mjs` | none (authorization **not** required) | No | No | No |
| Live | `node tools/tauri-e2e/live/lgj.mjs run` | **`TRACER_LIVE_GROK=1`** + **`TRACER_LIVE_GUI=1`** + **`TRACER_LIVE_GUI_AUTHORIZED=1`** | Yes (via bridge) | Yes | Possible |

## Authorization gate (W2.4.3-A)

Live execution requires **all** of:

1. `TRACER_LIVE_GROK=1` (or `TRACER_LIVE_SMOKE=1`)
2. `TRACER_LIVE_GUI=1`
3. `TRACER_LIVE_GUI_AUTHORIZED=1` — operator authorization (fail-closed without it)
4. Explicit `run` / `--live` subcommand

Dry-run and unit tests **never** require `TRACER_LIVE_GUI_AUTHORIZED` (provider-free).

Before any provider prompt, live mode prints a sanitized execution plan (scenario IDs, prompt budget, timeouts, approval policy).

## Scenarios (LGJ-01…07)

| ID | Meaning | Provider prompts (budget) | Honesty notes |
|---|---|---|---|
| LGJ-01 | Live runtime readiness | 0 | `BLOCKED_BY_AUTH` if session/auth gate blocks |
| LGJ-02 | Live prompt stream | 1 short | ≥1 timeline event |
| LGJ-03 | Cancel mid-stream | 1 cancellable | No deadlock budget |
| LGJ-04 | Restart history (no auto re-prompt) | 0 new | Same temp DB; must not auto re-submit |
| LGJ-05 | Approval reverse-request | max 1–2 attempts | `PASS` only if RR observed; else `NOT_OBSERVED` / `UNSUPPORTED` — **never fabricate** |
| LGJ-06 | Fail-closed error | 0 extra | Invalid path; stay tauri; no mock |
| LGJ-07 | Clean shutdown | 0 | No orphans (`tracer-desktop`, drivers, `grok`) |

**Hard max provider prompts per full suite:** ~3

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

# Safe construction check (no authorization env required)
node tools/tauri-e2e/live/dry-run.mjs --out target/live-gui/dry-run.json

# Live (may consume provider usage; needs local Grok auth + operator authorization)
$env:TRACER_LIVE_GROK = "1"
$env:TRACER_LIVE_GUI = "1"
$env:TRACER_LIVE_GUI_AUTHORIZED = "1"
node tools/tauri-e2e/live/lgj.mjs run --skip-build --out target/live-gui/live.json

# Subset
node tools/tauri-e2e/live/lgj.mjs run --journey LGJ-01,LGJ-06,LGJ-07 --skip-build
```

## Safety

1. Triple gate: env pair + operator authorization + explicit `run`/`--live`
2. Print operation class and execution plan before live path
3. Never print credentials/tokens
4. Public-safe bounded prompts only; secret-looking `--prompt` rejected
5. Sanitize artifacts (tokens, user path segments)
6. Never claim LGJ-05 PASS without observed reverse-request
7. If auth missing: `BLOCKED_BY_AUTH` with evidence; dry-run still ships
8. If authorization missing: live fails closed; journeys classified `NOT_RUN`

## Related

- L3-J (fake ACP): `tools/tauri-e2e/l3j-gui.mjs`
- Adapter live smoke: `tools/live-grok-smoke`
- Docs: `docs/modules/w2-4-3/`, `docs/validation/live-grok/AUTHORIZED_LIVE_GUI_RESULTS.md`
