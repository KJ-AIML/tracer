# VS1 Product Readiness (Gate 1.4)

**Date:** 2026-07-18  
**Decision:** **PASS** (product-local / fake ACP path)

## Scope of claim

Vertical Slice 1 is **product-locally ready** for:

1. Control-plane orchestration of project/session/prompt/cancel/approval/stop against **fake ACP**.
2. File-backed SQLite persistence, ordered event replay, restart recovery.
3. Bounded ingest bridge (256) with backpressure; sequence monotonicity under burst.
4. Desktop shell **typed snapshot journey** (H2) with mock command backend for deterministic UI tests.
5. Opt-in live harness present but **not required** for product-local acceptance.

## Proven paths

| Path | Status |
|---|---|
| Fake ACP + memory SQLite | PASS (VS-01…14) |
| Fake ACP + file SQLite | PASS (file-backed VS + soak) |
| Burst > bridge capacity | PASS (SOAK-01) |
| Desktop snapshot store / mock journey | PASS (vitest 18) |
| Heli absence non-fatal | PASS (VS-14) |

## Explicitly not claimed

- Production reliability of stock Grok under all versions / OSes.
- Full GUI E2E click-through with real Tauri runtime process.
- Wave 2 product features (multi-session polish, settings, collaboration, etc.).
- Presentation fan-out capacity under adversarial consumers (risk documented only).

## Residual product gaps (non-blocking for Gate 1.4 PASS)

1. Shell still uses mock backend in pure frontend tests; Tauri invoke path is thin glue (prior Gate 1.3 + H2).
2. Sticky within-session `persist_failed` UX messaging not polished.
3. Live provider path is separate readiness tier (see LIVE_PROVIDER_READINESS.md).

## Gate relationship

| Gate | Product-local |
|---|---|
| 1.3 vertical slice acceptance | accepted (fake) |
| 1.4 hardening | **PASS** — sequence fix + soak + desktop wiring integrated |
