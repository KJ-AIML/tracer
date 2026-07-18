# W2.3-B Live Grok GUI Plan

**Work item:** W2.3-B  
**Task:** `tracer-w2-live-gui-validation`  
**Harness:** `tools/tauri-e2e/live`  
**Classification:** manual local · Windows GUI · live authenticated · network · credentials from existing local auth  
**Standard CI:** **excluded** (opt-in only; never part of `pnpm -r test`)

## 1. Purpose

Validate the **product Tauri GUI** path against stock Grok ACP stdio:

```text
grok agent --no-leader stdio
```

for live journeys LGJ-01…LGJ-07 without redesigning control plane or product UI.

## 2. Scope (owned paths)

```text
tools/tauri-e2e/live/          # LGJ harness + test-only live bridge
tools/live-grok-smoke/         # cross-ref only (adapter LVS/LVA unchanged)
tests/live/gui/                # manual test policy
docs/modules/w2-3-b/           # plan / matrix / completion
docs/validation/live-grok/     # LIVE_GUI_RESULTS.md
```

Minimal **test-only** launch config is in scope (`launch-live-grok.mjs` as `TRACER_FAKE_ACP_JS` substitute).  
Whole `tools/tauri-e2e/` (W2.3-C) is **not** owned — only `live/` subfolder.

## 3. Safety constraints

1. Dual opt-in: `TRACER_LIVE_GROK=1` **and** `TRACER_LIVE_GUI=1` **and** explicit `run`/`--live`.
2. Dry-run never launches GUI live path and never spawns Grok agent stdio.
3. Print **operation class** before provider-capable path; require intent.
4. Never print credentials/tokens; never commit private prompts.
5. Public-safe bounded prompts only; reject secret-looking `--prompt`.
6. Sanitize artifacts (tokens, user path segments).
7. Approval RR (`LGJ-05`): **never fabricate PASS** without observed reverse-request.
8. Auth missing → `BLOCKED_BY_AUTH` with evidence; dry-run still ships.

## 4. Architecture

```text
lgj.mjs (opt-in)
  → tauri-driver + msedgedriver
  → tracer-desktop --tracer-e2e-env=<file>
       TRACER_DATABASE_PATH=temp
       TRACER_FAKE_ACP_JS=tools/tauri-e2e/live/launch-live-grok.mjs
  → GUI create session → node bridge → grok agent --no-leader stdio
  → LGJ-01…07 real GUI steps
  → shutdown + orphan check (incl. grok)
```

The bridge is **not** a product surface. Product still uses the existing fake-ACP spawn shape (`node <script> --scenario <id>`); the script substitutes stock Grok.

## 5. Stage / journey plan

| ID | Stage | Success | Auth? |
|---|---|---|---|
| LGJ-01 | Live runtime readiness | Tauri ready + session `ready` via live bridge | Yes for PASS |
| LGJ-02 | Live prompt stream | ≥1 timeline event after public-safe prompt | Yes |
| LGJ-03 | Cancel | Cancel control returns within deadlock budget | Yes |
| LGJ-04 | Restart history | Same DB restores history; **no auto re-prompt** | Yes |
| LGJ-05 | Approval RR | RR observed → PASS; else NOT_OBSERVED / UNSUPPORTED | Yes |
| LGJ-06 | Fail-closed | Invalid path; backend stays tauri; no mock | No |
| LGJ-07 | Clean shutdown | No orphans after teardown | No (if started) |

## 6. Classification vocabulary

```text
PASS | PARTIAL | NOT_RUN | NOT_OBSERVED | UNSUPPORTED
| BLOCKED_BY_AUTH | BLOCKED_BY_TOOLING | BLOCKED_BY_PRODUCT_GAP | FAIL
```

## 7. Operator runbook

```powershell
node tools/tauri-e2e/live/dry-run.mjs --out target/live-gui/dry-run.json

$env:TRACER_LIVE_GROK = "1"
$env:TRACER_LIVE_GUI = "1"
node tools/tauri-e2e/live/lgj.mjs run --out target/live-gui/live.json
```

Root aliases (not wired into `pnpm -r test`):

```text
pnpm test:tauri-e2e:live-gui:dry
pnpm test:tauri-e2e:live-gui
```

## 8. Out of scope

- Standard CI live enablement  
- CP / product UI redesign  
- Fabricated approval PASS  
- Commit secrets / tokens / private prompts  
- Folding live tests into `pnpm -r test`  
- Owning whole `tools/tauri-e2e/` (W2.3-C)
