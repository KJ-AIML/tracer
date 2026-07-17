# Live Grok Smoke Result (VS1-H1)

**Work item:** VS1-H1  
**Harness:** `tools/live-grok-smoke`  
**Date:** 2026-07-17  
**Platform:** windows-x86_64  

## 1. Classification (summary)

| Mode | Classification | Evidence |
|---|---|---|
| Dry-run unit tests + CLI dry-run | **`NOT_RUN`** (construction validated) | `cargo test -p live-grok-smoke`; `target/live-grok-smoke/dry-run.json` (local) |
| Live unauthenticated path | **Not required on this host** — stock auth cache present | session/new succeeded |
| Live authenticated full LVS | **`PASS`** | `target/live-grok-smoke/live-full.json` (local, sanitized; not committed) |

**Delivery claim:**

- Harness + dry-run path shipped and verified
- Live smoke on authoring host: **PASS** for LVS-01…LVS-08
- Fake-runtime vertical slice **unaffected**
- Credentials / tokens **not** committed; evidence sanitized

## 2. Dry-run results

### 2.1 Unit tests

```text
cargo test -p live-grok-smoke
# 16 passed
```

No `grok agent stdio` process is started by unit tests.

### 2.2 CLI dry-run

```text
cargo run -p live-grok-smoke -- dry-run
```

| Field | Observed |
|---|---|
| `classification` | `NOT_RUN` |
| `classificationTier` | `manual_local_live_authenticated_smoke` |
| `dryRun` | `true` |
| `spawnPlan.args` | `["agent","--no-leader","stdio"]` |
| `spawnPlan.matchesW0bW1d` | `true` |
| `spawnPlan.productHelper` | `tracer_runtime_adapter::grok_stdio_spawn_config` |

## 3. Live results (2026-07-17, windows-x86_64)

### 3.1 Runtime

| Field | Value |
|---|---|
| Binary | PATH `grok` (user home scrubbed in evidence) |
| Version | `grok 0.2.103 (89c3d36fb6) [stable]` (probe; W0-B documented 0.2.102 — minor skew) |
| Spawn | `RuntimeAdapter::start(grok_stdio_spawn_config(...))` |
| Auth | Stock advertised `cached_token` + `grok.com`; session/new succeeded (operator cache; **no tokens printed**) |
| Opt-in | `TRACER_LIVE_GROK=1` + `run` |

### 3.2 Stage matrix

| Stage | Status | Notes |
|---|---|---|
| 1 discovery | pass | binary found; version captured |
| 2 startup | pass | process_alive=true |
| 3 initialize | pass | protocol_ready=true; session_ready=false after init |
| 4 auth requirement | pass | methods listed (ids/names only) |
| 5 session | pass | session_ready=true |
| 6 prompt | pass | cancel-path submit |
| 7 streaming | pass | normalized event types observed |
| 8 approval | pass / n/a | no approval on smoke prompt (not auto-approved) |
| 9 cancel | pass | cancel_prompt Ok; no deadlock |
| 10 shutdown | pass | alive_after_shutdown=false; no orphan |

### 3.3 Scenario matrix

| ID | Status | Detail |
|---|---|---|
| LVS-01 | **PASS** | runtime process starts |
| LVS-02 | **PASS** | protocol initialize succeeds |
| LVS-03 | **PASS** | authentication state identified |
| LVS-04 | **PASS** | session creation succeeds |
| LVS-05 | **PASS** | ≥1 normalized stream/lifecycle event |
| LVS-06 | **PASS** | terminal `session.cancelled` (cancel path) |
| LVS-07 | **PASS** | cancellation does not deadlock |
| LVS-08 | **PASS** | shutdown leaves no orphan process |

### 3.4 Observed sanitized event types

```text
runtime.process.started
session.created
runtime.process.ready
adapter.protocol.unknown
session.ready
session.prompt.submitted
adapter.protocol.error
session.cancelled
runtime.process.exited
```

Notes:

- `adapter.protocol.unknown` / `adapter.protocol.error` appeared around vendor/extension traffic during cancel path — harness continued; scenarios still met terminal + cancel criteria.
- Full raw ACP frames are **not** persisted (by design).

### 3.5 Assumption checks

| ID | Result |
|---|---|
| A-W0B-01 stock argv | **match** |
| A-W1D-01 product helper | **match** |
| A-W0B-02 initialize without forced authenticate RPC | **match** (initialize ok) |
| A-W0B-03 session/new auth gate | **match-authenticated** on this host (cache present); W0-B unauth shape still valid for clean `GROK_HOME` |
| A-W1D-02 readiness split | **match** (after initialize: protocol true, session false) |

## 4. Comparison to W0-B / W1-D

| Topic | Assumption | Observed |
|---|---|---|
| Start command | `grok agent --no-leader stdio` | Spawn plan + live process |
| Readiness | initialize = protocol ready | Confirmed |
| Auth | may gate session/new | This host had cache → session ok; gate still mapped when required |
| Cancel | no deadlock | LVS-07 Pass |
| Shutdown | no orphans | LVS-08 Pass |

## 5. Residual gaps / risks

1. Live full path used **cancel-first** prompt exercise; happy-path end_turn stream deltas may be thinner than a dedicated non-cancel run.  
2. Version skew 0.2.102 (W0-B) vs 0.2.103 (this host).  
3. `adapter.protocol.unknown` / error during cancel path — worth follow-up mapping, not a harness ownership change.  
4. Approval reverse-request not forced by default prompt.  

## 6. Impact on vertical slice

| Slice area | Impact |
|---|---|
| Fake ACP / standard CI | **None** — live remains opt-in |
| Wave 2 | **Not started** |
| Credentials in git | **None** |
