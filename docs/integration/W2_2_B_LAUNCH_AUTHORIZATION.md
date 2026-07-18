# W2.2-B Launch Authorization — Full WebView GUI Product Journey

**Source gate:** 2.2.2 WebView tooling (this document)  
**Prior entry criteria:** `docs/integration/W2_2_B_ENTRY_CRITERIA.md` (Gate 2.2.1)  
**Date:** 2026-07-18

## Status of this document

This is a **launch authorization record and gate checklist** only.

| Action | Allowed by this document? |
|---|---|
| Document W2.2-B entry after tooling green | **Yes** |
| Create task `tracer-w2-webview-gui-journey` | **No** (not from this worker) |
| Claim W2.2-B | **No** |
| Author L3-J product journeys on W2.2-T branch | **No** |
| Fast-forward main | **No** |

## Prerequisites satisfied by Gate 2.2.2

| Prerequisite | Gate 2.2.2 evidence |
|---|---|
| Doctor can reach READY with drivers | **READY** on Windows authoring host |
| Opt-in setup for tauri-driver + msedgedriver | `tools/tauri-driver/setup.mjs` plan/apply |
| Compatibility rule documented | major(msedgedriver) == major(Edge) |
| L2 PASS | executable evidence |
| L3-I PASS | driver session + root + Tauri IPC surface + cleanup |
| L3-J not falsely claimed | still **NOT_STARTED** |
| CI isolation for L3-I | not in `pnpm -r test` |

## Authorization posture

```text
Gate 2.2.2 PASS  →  tooling ready for product journey work
Product decision  →  required before creating W2.2-B task
W2.2-B task create/claim  →  NOT authorized by W2.2-T worker alone
```

**W2.2-B launch authorization (tooling side):** **YES — tooling prerequisites met.**  
**W2.2-B start authorization (task create/claim):** **NO — deferred to product / program.**

## Recommended W2.2-B scope (when product authorizes)

| Theme | Entry condition | Notes |
|---|---|---|
| L3-I green on journey host | doctor READY; `pnpm test:tauri-e2e:l3i` PASS | Re-run after Edge updates |
| Product DOM journeys | L3-I PASS | session create, prompt, approval, multi-session focus |
| Public Tauri invoke surface | if journeys need `__TAURI__.core.invoke` | product may enable `withGlobalTauri` or use module API — **product decision** |
| Cross-platform GUI | explicit decision | macOS external driver still unsupported |
| Live Grok through GUI | credentials + opt-in | never default CI |

## Recommended commands before first L3-J commit

```powershell
pnpm test:tauri-e2e:doctor          # READY
pnpm test:tauri-e2e:l2              # PASS
pnpm test:tauri-e2e:l3i             # PASS
# Only after product-authorized task exists:
# author L3-J journeys under that task's ownership
```

## Non-goals carried forward

- No auto live Grok in standard CI  
- No converting `BLOCKED_BY_TOOLING` into product PASS/FAIL  
- No IDE / editor / ALMS / plugins / collab / marketplace from this auth doc  
- Sequence safety and fail-closed invoke policy remain mandatory  

## Explicit non-action

**Do not create or claim** `tracer-w2-webview-gui-journey` from Gate 2.2.2 tooling work.  
Wait for product/program authorization after this gate is integrated.
