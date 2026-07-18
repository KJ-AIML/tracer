# Live GUI Readiness (Gate 2.3)

**Gate:** 2.3  
**Decision:** **READY for opt-in manual runs; live journeys NOT_RUN**  
**Date:** 2026-07-18  
**Source tip (W2.3-B):** `61a222b1728f7b6913166a2f19be67032940d96c`  
**Resume session (W2.3-B):** `heli-ses-da4d6507-4948-4776-90de-2cb7f1e4cbeb`  
**Integration session:** `heli-ses-9ccdc8b9-7065-43ff-b243-85efe0759187`

## Honesty

LGJ-01...07 were **not executed**. Stock `grok` was not on PATH; dual opt-in unset; no operator authorization for provider usage. **No PASS fabricated.**

## Dual opt-in gate

Live requires **all** of:

1. `TRACER_LIVE_GROK=1` (or `TRACER_LIVE_SMOKE=1`)  
2. `TRACER_LIVE_GUI=1`  
3. Explicit `run` / `--live`  
4. Grok binary discoverable + local auth  

Dry-run / unit never spawn live Grok.

## Classifications

| Suite | Result |
|---|---|
| Unit | PASS |
| Dry-run | constructionPass; journeys NOT_RUN |
| LGJ-01...07 | **NOT_RUN** |
| Provider usage | none |
| Standard CI | Live **forbidden** (`pnpm -r test` does not run LGJ) |

## Operation class

`manual_local_live_authenticated_gui`
