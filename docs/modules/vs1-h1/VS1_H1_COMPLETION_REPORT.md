# VS1-H1 Completion Report — Live Grok Smoke Harness

**Work item:** VS1-H1  
**Heli task:** `tracer-vs1-live-smoke`  
**Branch:** `agent/tracer-vs1-live-smoke`  
**Base:** `15c9399c28f79bdf9c125c26f52d7bf956fb4722`  
**Date:** 2026-07-17  

## 1. Outcome

Delivered a **manual, opt-in** live validation harness for stock Grok ACP stdio (`grok agent --no-leader stdio`), reusing W1-D `tracer-runtime-adapter` public APIs. Dry-run and construction tests are green. Live authenticated full parity is **not claimed** without operator auth; unauthenticated live is classified **`BLOCKED_BY_AUTH`** without failing the fake-runtime vertical slice.

## 2. Owned paths touched

```text
tools/live-grok-smoke/          # CLI harness crate
tests/live/grok/                # manual test docs + policy
docs/validation/live-grok/      # plan + result
docs/modules/vs1-h1/            # this report
Cargo.toml                      # minimal workspace member registration
```

## 3. Deliverables

| Deliverable | Path |
|---|---|
| Plan | `docs/validation/live-grok/LIVE_GROK_SMOKE_PLAN.md` |
| Result | `docs/validation/live-grok/LIVE_GROK_SMOKE_RESULT.md` |
| Completion | `docs/modules/vs1-h1/VS1_H1_COMPLETION_REPORT.md` |
| Harness | `tools/live-grok-smoke/` |
| Opt-in docs | `tools/live-grok-smoke/README.md`, `tests/live/grok/README.md` |

## 4. Harness design

### Stages

1. binary discovery  
2. process startup  
3. protocol initialization  
4. authentication requirement  
5. authenticated session creation  
6. prompt submission  
7. streaming  
8. approval if requested (observe; never auto-approve)  
9. cancellation  
10. shutdown  

### Scenarios

LVS-01 … LVS-08 as specified in the work item.

### Safety

- Opt-in: `TRACER_LIVE_GROK=1` (or `TRACER_LIVE_SMOKE=1`) **and** `run` subcommand  
- Dry-run never launches agent stdio  
- Sanitized JSON evidence; secret keys / bearer / sk- tokens / user path segments redacted  
- Public-safe default prompt only; rejects prompt text that looks like secrets  
- Classification tier always labeled `manual_local_live_authenticated_smoke`  

### Product reuse

- `grok_stdio_spawn_config` / `grok_stdio_args`  
- `RuntimeAdapter::{start, initialize, create_session, submit_prompt, cancel_prompt, shutdown, inspect, auth_state, …}`  

No ACP reimplementation; no control-plane or process-manager redesign.

## 5. Validation performed

```text
cargo test -p live-grok-smoke
cargo run -p live-grok-smoke -- dry-run
```

Optional (operator):

```text
$env:TRACER_LIVE_GROK=1
cargo run -p live-grok-smoke -- run --allow-unauth
```

## 6. Live result classification

| Path | Classification |
|---|---|
| Dry-run / unit tests | `NOT_RUN` (construction Pass) |
| Live full LVS (authoring host 2026-07-17) | **`PASS`** (LVS-01…LVS-08; sanitized evidence local only) |
| Live without auth (clean GROK_HOME) | Still expected `BLOCKED_BY_AUTH` per W0-B; not forced on this host |

## 7. Assumptions

1. W1-D adapter on this base SHA is the correct product API for stock spawn + ACP lifecycle.  
2. W0-B findings for `0.2.102` remain representative of PATH `grok` on the operator machine.  
3. Standard CI will pick up `cargo test -p live-grok-smoke` only as dry-run unit tests; live `run` stays gated.  
4. Credentials exist only in operator environment (`GROK_HOME` login / stock auth) — never in-repo.

## 8. Risks

| Risk | Mitigation |
|---|---|
| Operators treat dry-run as live parity | Classification `NOT_RUN` + docs |
| Accidental CI live spend | Env gate + CI matrix `forbids: [live]` on standard jobs |
| Auth block misread as slice failure | Explicit `BLOCKED_BY_AUTH`; result docs |
| Version skew vs W0-B | Discovery records version in evidence |

## 9. Integration requirements

1. Keep `tools/live-grok-smoke` as optional workspace member (already registered).  
2. Do not wire `run` into standard CI workflows.  
3. Optional nightly/manual job may set `TRACER_LIVE_GROK=1` with secrets from a secret store.  
4. Control plane / desktop may later call the same adapter APIs; this harness is not a product dependency.

## 10. Forbidden work (confirmed not done)

- Control-plane redesign  
- ACP adapter redesign  
- Process-manager redesign  
- Desktop product work  
- Committed credentials  
- Wave 2 product features  
- Push to remote  

## 11. Commits

Local commits only on `agent/tracer-vs1-live-smoke` — see git log for SHAs after commit.
