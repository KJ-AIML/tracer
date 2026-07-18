# W2.3-B Test Matrix — Live Grok GUI (LGJ)

**Task:** `tracer-w2-live-gui-validation`  
**Harness:** `tools/tauri-e2e/live`

## Isolation

| Property | Value |
|---|---|
| Standard CI | **No** |
| `pnpm -r test` | **No** |
| Network | Yes (live only) |
| Credentials | Operator local auth only |
| Provider usage | Possible when live |
| Fake ACP | No (live bridge) |
| DB | Temp file SQLite |

## Dry-run matrix

| Check | Expected |
|---|---|
| Bridge script present | pass |
| Spawn plan `agent --no-leader stdio` | pass |
| Journey catalog LGJ-01…07 | pass |
| Classification | `NOT_RUN` |
| Exit | 0 if construction ok |

## Live matrix (opt-in)

| ID | Intent | PASS requires | Soft classifications |
|---|---|---|---|
| LGJ-01 | Runtime readiness | session ready via live bridge | BLOCKED_BY_AUTH, BLOCKED_BY_TOOLING |
| LGJ-02 | Prompt stream | ≥1 event type in GUI timeline | BLOCKED_BY_AUTH, FAIL |
| LGJ-03 | Cancel | returns within deadlock budget | PARTIAL (fast complete), BLOCKED_BY_AUTH |
| LGJ-04 | Restart history | history present; no auto re-prompt | PARTIAL (empty timeline), BLOCKED_BY_AUTH |
| LGJ-05 | Approval RR | `approval.requested` or approval card | **NOT_OBSERVED**, **UNSUPPORTED**, BLOCKED_BY_AUTH — never fabricate PASS |
| LGJ-06 | Fail-closed | error surface; backend=tauri; no mock | PARTIAL if banner weak |
| LGJ-07 | Shutdown | no orphans after teardown | FAIL if orphans remain |

## Suite aggregation

| Journey set | Overall |
|---|---|
| All PASS | PASS |
| All NOT_RUN | NOT_RUN |
| Any FAIL | FAIL |
| All auth-blocked (no FAIL) | BLOCKED_BY_AUTH |
| Mixed soft outcomes | PARTIAL |

## Commands

```powershell
# Dry
pnpm test:tauri-e2e:live-gui:dry

# Live
$env:TRACER_LIVE_GROK=1; $env:TRACER_LIVE_GUI=1
pnpm test:tauri-e2e:live-gui -- run --skip-build
```
