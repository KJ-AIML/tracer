# W2.2-B Entry Criteria — Full WebView GUI Product Journey

**Source gate:** 2.2.1 PASS on `integration/tracer-w2-2-1`  
**This document does not create or claim task `tracer-w2-webview-gui-journey` (W2.2-B).**  
**Date:** 2026-07-18

## Prerequisites (from Gate 2.2.1)

1. Drain lifecycle contract integrated: prompt return ≠ ingestion complete; false PE matrix green under fake ACP.
2. Tauri E2E infrastructure present: doctor, L2 smoke, L3-I harness with honest classifications.
3. L0+L1 desktop boundary remain green under standard CI.
4. L2 app launch smoke **PASS** on at least one Windows GUI agent host.
5. L3-I classified **BLOCKED_BY_TOOLING** or better — harness exists; driver install is operator concern.
6. L3-J remains **NOT_STARTED** until a dedicated product-journey task is explicitly authorized.

## Recommended W2.2-B scope (guidance only)

| Theme | Entry condition | Notes |
|---|---|---|
| Install `tauri-driver` + matching `msedgedriver` | Gate 2.2.1 PASS | Operator; do not auto-download in product CI without policy |
| L3-I green on authoring host | Drivers on PATH; doctor `READY` (or L3-I-ready) | Prove session + basic WebDriver probes |
| L3-J product journeys | L3-I green + product scripts | Session create, prompt, approval, multi-session focus via DOM |
| Cross-platform GUI | Explicit product decision | macOS external driver currently unsupported in W2.2-A matrix |
| Live Grok through GUI | Credentials + intentional opt-in | Not default CI |

## Non-goals carried forward

- No auto live Grok in standard CI
- No converting `BLOCKED_BY_TOOLING` / `DRIVER_UNAVAILABLE` into product PASS/FAIL
- No IDE / editor / ALMS / plugins / collab / marketplace scope from this entry doc
- No headless watchdog scripts as product surface
- Sequence safety and fail-closed invoke policy remain mandatory

## Recommended entry command posture

```powershell
pnpm test:tauri-e2e:doctor          # expect READY once drivers installed
node tools/tauri-e2e/l3i-infra.mjs  # expect PASS before claiming L3-J work
# Then author L3-J journeys under a new authorized task only
```

## Recommendation

**W2.2-B may begin** only after Gate 2.2.1 lands on `main` (local tag `tracer-wave2.2.1-e2e-foundation`) **and** product explicitly authorizes the full GUI journey task.  
Until then: **do not create or claim** `tracer-w2-webview-gui-journey`.
