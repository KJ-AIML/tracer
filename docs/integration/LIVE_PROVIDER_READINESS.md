# Live Provider Readiness (Gate 1.4)

**Date:** 2026-07-18  
**Harness:** `tools/live-grok-smoke`  
**Classification:** **PROVEN_ON_AUTHORING_HOST**

## Summary decision

| Mode | Classification | Gate 1.4 action |
|---|---|---|
| Dry-run / unit | PASS (construction) | Re-executed on integrated branch |
| Live authenticated (authoring host) | **PROVEN_ON_AUTHORING_HOST** | **Evidence reused from H1** |
| Live unauthenticated clean GROK_HOME | Expected BLOCKED_BY_AUTH (W0-B) | Not forced |
| Standard CI live | **Forbidden / not claimed** | No credentials, no auto `run` |
| Cross-platform / all Grok versions | **Not claimed** | — |
| Production reliability | **Not claimed** | — |

## Live evidence policy for Gate 1.4

```text
Live evidence reused from H1 authoring-host validation.
No new provider usage consumed during Gate 1.4.
```

Source: `docs/validation/live-grok/LIVE_GROK_SMOKE_RESULT.md` (H1).

### H1 authoring-host matrix (reused)

| ID | Status |
|---|---|
| LVS-01…LVS-08 | PASS |
| Stages discovery→shutdown | pass |
| Version | `grok 0.2.103` observed (W0-B docs: 0.2.102) |
| Auth | stock cached_token path on host (no tokens in git) |
| Approval reverse-request | not forced by default smoke prompt |

## Integrated harness guarantees

| Guarantee | Evidence |
|---|---|
| Workspace member registered | `Cargo.toml` includes `tools/live-grok-smoke` |
| Unit tests never spawn live agent stdio for LVS | `cargo test -p live-grok-smoke` → 16 passed |
| Dry-run never claims live parity | classification `NOT_RUN` |
| Live `run` requires opt-in | `TRACER_LIVE_GROK=1` or `TRACER_LIVE_SMOKE=1` |
| Sanitize secrets / paths | unit tests on sanitize module |
| CI matrix documents env gate | `tests/specifications/ci/matrix.yaml` `envGate: TRACER_LIVE_SMOKE=1` |

## Version skew

| Source | Version |
|---|---|
| W0-B recon docs | 0.2.102 |
| H1 authoring host | 0.2.103 |

Do **not** claim bit-identical behavior across these without re-probe.

## Residual live risks

1. Cancel-first live prompt path may under-exercise end_turn happy stream.
2. `adapter.protocol.unknown` / error noise during cancel path (mapping follow-up).
3. Approval reverse-request not proven on default smoke.
4. Auth-cache-dependent host result; clean home still BLOCKED_BY_AUTH.

## What would change classification

| New evidence | Possible reclass |
|---|---|
| Clean GROK_HOME unauth only | BLOCKED_BY_AUTH for full prompt parity |
| Failed live run on integrated branch | CONDITIONAL / FAIL |
| Multi-host matrix | stronger claim only if green |
