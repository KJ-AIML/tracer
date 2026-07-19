# Wave 2.4.2 Entry Criteria

**Source gate:** 2.4.1 PASS on `integration/tracer-w2-4-1-upgrade` → `main`  
**This document does not create or claim any Wave 2.4.2 tasks.**  
**Date:** 2026-07-19

## Prerequisites (from Gate 2.4.1)

1. Upgrade fixture **PASS** with real N→N+1 NSIS (RC-03 supersession recorded).  
2. Data preservation + uninstall/reinstall **PASS**.  
3. Release provenance generate/verify **PASS** with `buildSourceSha` ≠ report-only tip when reports follow.  
4. Signing class honest: `UNSIGNED_DEVELOPMENT_RC` acceptable until Authenticode policy.  
5. Deterministic workspace suites green; L2/L3-I remain outside `pnpm -r test`.  
6. Local tag `tracer-wave2.4.1-upgrade-verified` exists after main fast-forward.  
7. Rollback tag `tracer-wave2.3-windows-rc` still present and untouched.

## Recommended Wave 2.4.2 themes (guidance only)

| Theme | Entry condition | Notes |
|---|---|---|
| Production Authenticode + CI secrets | Gate 2.4.1 PASS + cert policy | Elevate beyond UNSIGNED_DEVELOPMENT_RC |
| Authorized live Grok GUI (LGJ) | Dual opt-in + grok + operator auth | Never default CI |
| Cross-platform packaging / GUI | Explicit product decision | |
| IDE / editor integration | Explicit product decision | Not started |
| ALMS / long-memory surfaces | Explicit product decision | |
| Plugins / marketplace | Explicit product decision | |

## Non-goals carried forward

- No auto live Grok in standard CI  
- No folding L2/L3/live into `pnpm -r test` without new isolation policy  
- No silent mock fallback  
- No Wave 2.4.2 task creation from this document alone  

## Recommendation

**Wave 2.4.2 may be planned** after `main` fast-forwards to Gate 2.4.1 tip and local tag `tracer-wave2.4.1-upgrade-verified` exists.  
Until product explicitly authorizes a Wave 2.4.2 task: **do not create or claim** IDE/ALMS/plugins/marketplace/live-provider/signing work from this document alone.