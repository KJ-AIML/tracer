# Wave 2.4 Entry Criteria

**Source gate:** 2.3 PASS on `integration/tracer-w2-3` ? `main`  
**This document does not create or claim any Wave 2.4 tasks.**  
**Date:** 2026-07-18

## Prerequisites (from Gate 2.3)

1. Windows RC packaging **PASS** with honest signing class (`UNSIGNED_DEVELOPMENT_RC` acceptable).  
2. GUI reliability **PASS** (?5 consecutive first-attempt L3-J; zeros on product fails/orphans/ports/temp/unsanitized).  
3. Live GUI harness **present** with dual opt-in; LGJ may remain **NOT_RUN**.  
4. Standard CI isolation retained: `pnpm -r test` = L0+L1 only.  
5. Deterministic workspace suites green (cargo fmt/check/test/clippy, control-plane, soak).  
6. Local tag `tracer-wave2.3-windows-rc` exists after main fast-forward.

## Recommended Wave 2.4 themes (guidance only)

| Theme | Entry condition | Notes |
|---|---|---|
| Production Authenticode + CI secrets | Gate 2.3 PASS + cert policy | Elevate beyond UNSIGNED_DEVELOPMENT_RC |
| Prior-RC upgrade fixture (RC-03) | Explicit prior package available | Close FIXTURE_LIMITED honesty gap |
| Authorized live Grok GUI (LGJ-01...07) | Dual opt-in + grok + operator auth | Never default CI |
| Cross-platform packaging / GUI | Explicit product decision | |
| IDE / editor integration | Explicit product decision | Not started |
| ALMS / long-memory surfaces | Explicit product decision | |
| Plugins / marketplace | Explicit product decision | |

## Non-goals carried forward

- No auto live Grok in standard CI  
- No folding L2/L3/live into `pnpm -r test` without new isolation policy  
- No custom headless watchdog workers as product surface  
- No silent mock fallback / sequence fail-closed retained  
- No Wave 2.4 task creation from this document alone  

## Recommendation

**Wave 2.4 may be planned** after `main` fast-forwards to Gate 2.3 tip and local tag `tracer-wave2.3-windows-rc` exists.  
Until product explicitly authorizes a Wave 2.4 task: **do not create or claim** IDE/ALMS/plugins/marketplace/live-provider work from this document alone.
