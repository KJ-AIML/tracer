# W1-G Completion Report

**Task:** `tracer-w1-fake-runtime`  
**Work item:** W1-G — Fake Runtime and Contract Harness  
**Target repository:** `tracer`  
**Worktree:** `repos/worktrees/tracer-w1-g`  
**Branch:** `agent/tracer-w1-fake-runtime`  
**Mode:** write  
**Heli session:** `heli-ses-dd04adc8-0cf4-4001-9a85-f0b2c8acaa07`  
**Lease:** `heli-lease-384b8d1f-fa6f-4dbe-a360-e1cb35f5cd38`  
**Base SHA:** `e104d8d21a3370193decd9472036e037741ad3e7`  
**Deliverable SHA:** `18e2d6019c37f3676885663018ce6fa6591e5bb6`  
**Date:** 2026-07-17  
**Host:** grok-build

## Summary

Implemented a deterministic fake ACP-compatible stdio runtime, shared test-fixture loaders for the W0-D scenario catalog and expected-event packs, and a contract harness that exercises all standardCi catalog scenarios without network, live Grok, or provider credentials. Synthetic/fake evidence is explicitly separated from live parity.

## Files changed

| Path | Role |
|---|---|
| `tools/fake-acp-runtime/**` | Fake ACP NDJSON binary + scenario scripts |
| `packages/test-fixtures/**` | Catalog YAML loader, expected-events, provenance, paths |
| `tests/contract/fake-runtime/**` | Contract harness (spawn fake, drive scenarios, no-network checks) |
| `docs/modules/w1-g/**` | Module docs + this report |

No root manifests added (forbidden / request-only). No production runtime, UI, storage, or process-manager ownership. Did not touch `tests/contract/event-protocol/` (W1-B).

## Scenario coverage

All `standardCi: true` ids from `tests/specifications/scenarios/catalog.yaml`:

- happy_prompt_stream
- auth_required_session_new
- permission_allow / permission_deny
- cancel_mid_stream / cancel_while_permission_pending
- malformed_frame / unknown_vendor_notification
- eof_mid_prompt / crash_nonzero_exit
- cancel_unsupported / slow_cancel_ack
- duplicate_response_id / capability_minimal
- clean_shutdown_stdin_close

Live-only ids rejected (exit 2): `live_stock_auth_prompt`, `live_stock_auth_required_reprobe`.

## Validation performed

```text
npx --yes github:KJ-AIML/heli-harness task claim tracer-w1-fake-runtime --mode write --host grok-build
# session: heli-ses-dd04adc8-0cf4-4001-9a85-f0b2c8acaa07
npx --yes github:KJ-AIML/heli-harness target set tracer
npx --yes github:KJ-AIML/heli-harness session status

node --test tests/contract/fake-runtime/*.test.js
```

### Test results

| Suite | Result |
|---|---|
| `catalog.test.js` | Pass (6) |
| `expected-events.test.js` | Pass (5) |
| `harness.test.js` | Pass (17) |
| `no-network.test.js` | Pass (2) |
| **Total** | **30 pass / 0 fail** |

### Checks

1. Fake speaks NDJSON JSON-RPC 2.0; stderr logs only
2. Scenario selectable via `--scenario` / `TRACER_FAKE_ACP_SCENARIO`
3. Deterministic fixed UUIDs and ordering
4. Auth-required distinguishes process ready vs session ready
5. No network clients / live smoke flags in owned trees
6. Expected-events packs assert W0-A names only (no W0-B aliases)
7. Evidence: fake-runtime/synthetic never claimed as live parity

## Unverified / deferred

1. Full product normalization (adapter → Tracer envelopes) is W1-B/D/F; harness maps wire observations to product types as a test aid only.
2. Windows Job Object orphan matrix remains T5 / W1-C process manager.
3. Stock Grok live smoke remains opt-in T6 (`TRACER_LIVE_SMOKE=1`) — not implemented here.
4. Root Cargo/pnpm workspace manifests not created (request-only); packages are self-contained Node modules runnable via `node --test`.

## Git

| SHA | Message |
|---|---|
| `18e2d6019c37f3676885663018ce6fa6591e5bb6` | `feat(w1-g): deterministic fake ACP runtime and contract harness` |

Local commits only; **no push**.

## Finish sequence

```text
# after commits:
npx --yes github:KJ-AIML/heli-harness task release tracer-w1-fake-runtime --session heli-ses-dd04adc8-0cf4-4001-9a85-f0b2c8acaa07
npx --yes github:KJ-AIML/heli-harness session close --session heli-ses-dd04adc8-0cf4-4001-9a85-f0b2c8acaa07
```
