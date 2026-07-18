# Wave 2.3 Entry Criteria

**Source gate:** 2.2.3 PASS on `integration/tracer-w2-webview-gui-journey`  
**This document does not create or claim any Wave 2.3 tasks.**  
**Date:** 2026-07-18

## Prerequisites (from Gate 2.2.3)

1. Full WebView GUI product journey (L3-J) **PASS** on integrated main (GJ-01..12, repeatable).
2. Doctor **READY**, L2 **PASS**, L3-I **PASS** remain green on authoring host.
3. Desktop product readiness **PASS** for Windows fake-ACP GUI; fail-closed policy retained.
4. L0+L1 and control-plane regression suites remain green under standard CI.
5. Live-provider GUI and cross-platform GUI remain **UNPROVEN** (honest non-claims).

## Recommended Wave 2.3 themes (guidance only)

| Theme | Entry condition | Notes |
|---|---|---|
| Live-provider GUI journeys (opt-in) | Gate 2.2.3 PASS + credentials policy | Never default CI |
| Cross-platform GUI (macOS/Linux) | Explicit product decision + drivers | Prior tooling marks macOS external driver unsupported |
| IDE / editor integration | Explicit product decision | Not started by this gate |
| ALMS / long-memory surfaces | Explicit product decision | |
| Plugins / marketplace | Explicit product decision | |
| Collab multi-user | Explicit product decision | |

## Non-goals carried forward

- No auto live Grok in standard CI  
- No folding L3-J into `pnpm -r test` without new isolation policy  
- No headless watchdog scripts as product surface  
- No silent mock fallback  
- Sequence safety remains fail-closed  

## Recommendation

**Wave 2.3 may be planned** after `main` fast-forwards to Gate 2.2.3 tip and local tag `tracer-wave2.2.3-full-gui` exists.  
Until product explicitly authorizes a Wave 2.3 task: **do not create or claim** IDE/ALMS/plugins/marketplace/live-provider-GUI work from this document alone.
