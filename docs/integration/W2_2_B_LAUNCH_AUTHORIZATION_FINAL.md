# W2.2-B Launch Authorization — FINAL (Gate 2.2.2 integrated)

**Source gate:** 2.2.2 WebView tooling **integrated on main**  
**Prior worker auth (non-final):** `docs/integration/W2_2_B_LAUNCH_AUTHORIZATION.md`  
**Prior entry criteria:** `docs/integration/W2_2_B_ENTRY_CRITERIA.md` (Gate 2.2.1)  
**Integrator:** `tracer-w2-webview-tooling-integration` · host `grok-build` · 2026-07-18

## Status of this document

This is the **integration-final** launch authorization record for W2.2-B after Gate 2.2.2 tooling lands on main.

| Action | Allowed by this document? |
|---|---|
| Record tooling prerequisites met on integrated main | **Yes** |
| Treat doctor READY + L2 PASS + L3-I PASS as tooling green | **Yes** |
| Create task `tracer-w2-webview-gui-journey` / W2.2-B | **No** |
| Claim W2.2-B | **No** |
| Author L3-J product journeys from this gate | **No** |
| Auto-start W2.2-B from integration finalize | **No** |

## Prerequisites (integration evidence)

| Prerequisite | Integrated evidence |
|---|---|
| Tooling merged to main via non-FF | `merge(w2.2.2): WebView tooling…` |
| Doctor READY with drivers | `pnpm test:tauri-e2e:doctor` → **READY** |
| Opt-in setup plan/apply | `tools/tauri-driver/setup.mjs` |
| Compatibility rule | `major(msedgedriver)==major(Edge)` |
| L2 PASS | `pnpm test:tauri-e2e:l2` |
| L3-I PASS | `pnpm test:tauri-e2e:l3i` (infra only) |
| L3-J not falsely claimed | still **NOT_STARTED** |
| CI isolation | L2/L3-I not in `pnpm -r test` |
| No binaries tracked | gitignore + audit zero |
| No W2.2-B task created by this gate | confirmed |

## Authorization posture (FINAL)

```text
Gate 2.2.2 PASS on main  →  tooling ready for product journey work
Product / program decision →  required before creating W2.2-B task
W2.2-B task create/claim   →  NOT authorized by tooling or integration alone
```

| Decision | Result |
|---|---|
| **W2.2-B launch authorization (tooling side)** | **YES** — prerequisites met on integrated tree |
| **W2.2-B start authorization (task create/claim)** | **NO** — deferred to product/program |
| **Do not create or start W2.2-B from Gate 2.2.2-I** | **Confirmed** |

## Recommended W2.2-B scope (when product authorizes)

| Theme | Entry condition | Notes |
|---|---|---|
| L3-I green on journey host | doctor READY; `pnpm test:tauri-e2e:l3i` PASS | Re-run after Edge updates |
| Product DOM journeys | L3-I PASS | session create, prompt, approval, multi-session focus |
| Public Tauri invoke surface | if journeys need `__TAURI__.core.invoke` | product may enable `withGlobalTauri` or module API — **product decision** |
| Cross-platform GUI | explicit decision | macOS external driver still unsupported |
| Live Grok through GUI | credentials + opt-in | never default CI |

## Recommended pre-flight before first L3-J commit

```powershell
pnpm test:tauri-e2e:doctor          # READY
pnpm test:tauri-e2e:l2              # PASS
pnpm test:tauri-e2e:l3i             # PASS
# Only after product-authorized task exists:
# author L3-J journeys under that task's ownership
```

## Non-goals carried forward

- No auto live Grok in standard CI  
- No converting tooling `BLOCKED_BY_TOOLING` into product PASS/FAIL  
- No IDE / editor / ALMS / plugins / collab / marketplace from this auth doc  
- Sequence safety and fail-closed invoke policy remain mandatory  
- L3-J remains **NOT_STARTED** until a separate authorized task owns it  

## Explicit non-action

**Do not create or claim** `tracer-w2-webview-gui-journey` / W2.2-B from Gate 2.2.2 tooling or integration work.  
Wait for product/program authorization after this gate is on main.