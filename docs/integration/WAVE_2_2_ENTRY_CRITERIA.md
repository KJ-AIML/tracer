# Wave 2.2 Entry Criteria

**Source gate:** 2.1 PASS on `integration/tracer-w2-1`  
**This document does not create or claim Wave 2.2 tasks.**

## Prerequisites (from Gate 2.1)

1. Presentation hub + multi-session focus contracts landed and tested (MS-01..17, INV suite).
2. Desktop boundary L0+L1 green; fail-closed invoke policy retained.
3. Live approval harness present as **opt-in**; live observations still optional.
4. Fake ACP path remains the default CI reliability path.

## Recommended Wave 2.2 themes (guidance only)

| Theme | Entry condition | Notes |
|---|---|---|
| IDE / editor integration | Gate 2.1 PASS | Out of scope for 2.1 |
| ALMS / long-memory surfaces | Gate 2.1 PASS | |
| Plugins / marketplace | Explicit product decision | Do not start from this gate |
| Collab multi-user | Explicit product decision | |
| Full WebView GUI E2E | Tooling: tauri-driver + WebView2 | Elevates desktop from L1→L3 |
| Live approval observation campaign | Credentials + intentional run | Can raise LVA from NOT_OBSERVED |

## Non-goals carried forward

- No auto live Grok in CI
- No headless watchdog scripts as product surface
- Sequence safety remains fail-closed

## Recommendation

**Wave 2.2 may begin** after `main` fast-forwards to Gate 2.1 tip and local tag `tracer-wave2.1-runtime-polish` exists. Prioritize full GUI E2E tooling and intentional live LVA evidence only when product asks.