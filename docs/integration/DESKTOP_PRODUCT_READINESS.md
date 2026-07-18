# Desktop Product Readiness — Gate 2.2.3

**Date:** 2026-07-18  
**Gate:** 2.2.3 Full WebView GUI product journey

## Decision summary

| Dimension | Decision |
|---|---|
| **Desktop product readiness** | **PASS** |
| Live-provider GUI | **UNPROVEN** |
| Cross-platform GUI | **UNPROVEN** |
| Fail-closed invoke policy | **PASS** (held under L3-J GJ-11) |
| Fake ACP product journeys | **PASS** (GJ-01..12 ×2) |

## Product surface review

Integrated desktop changes were classified item-by-item (see integration report §6.3).

**No weakened product behavior for automation:**

- Approvals remain explicit Allow/Deny (never auto-allow).
- Tauri mode does not fall back to mock on invoke failure.
- Leave-session confirm remains for users; skipped only when harness sets `globalThis.__TRACER_E2E__`.
- E2E env loader is no-op without absolute allowlisted env file / CLI flag.

## Product capabilities proven via real GUI

1. Startup in Tauri backend mode  
2. Project register by path  
3. Session create with fake ACP scenario  
4. Prompt submit + streaming timeline  
5. Approval accept / reject  
6. Cancel while approval pending (deadlock-free)  
7. Two-session presentation focus switch  
8. Runtime crash/EOF reflection  
9. Restart + history restore from file DB  
10. Heli unavailable non-fatal  
11. Invoke failure fail-closed  
12. Clean shutdown / no orphans  

## Explicit non-claims

| Claim | Status |
|---|---|
| Live Grok / provider GUI journeys | **UNPROVEN** — no credentials, not run |
| macOS / Linux WebView journeys | **UNPROVEN** — Windows evidence only |
| Native OS folder picker as sole register path | Optional; path form is product-supported |
| Production packaging / signed installers | Not this gate |
| IDE / ALMS / plugins / marketplace | Out of scope |

## Residual product notes

1. Fake ACP scenario selector is intentional in the current fake-default product world.  
2. `withGlobalTauri: true` exposes public `__TAURI__` — intentional for invoke.  
3. Timing-sensitive GUI hosts may need artifact-assisted re-runs.  

## Readiness statement

**Desktop product readiness: PASS** for Windows fake-ACP WebView product journeys under Gate 2.2.3 criteria.  
**Live-provider GUI: UNPROVEN.**  
**Cross-platform GUI: UNPROVEN.**
