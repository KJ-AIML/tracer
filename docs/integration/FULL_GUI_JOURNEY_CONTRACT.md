# Full GUI Journey Contract (L3-J / Gate 2.2.3)

**Status:** Active after Gate 2.2.3 PASS  
**Owner surface:** `tools/tauri-e2e/l3j-gui.mjs` + `apps/desktop` product GUI  
**Isolation:** Explicit `pnpm test:tauri-e2e:gui` only — **not** part of `pnpm -r test` or `cargo test --workspace`

## Purpose

Prove end-to-end **product** journeys through the real Tauri WebView + WebDriver stack using **fake ACP** and **temp SQLite**, without live provider credentials.

## Preconditions

1. Doctor **READY** (tauri-driver + matching msedgedriver + app binary + frontend dist).
2. L2 and L3-I independently executable (honest infra).
3. Host PATH includes cargo bin + project-local drivers (`tools/tauri-driver/bin`, `.cache/msedgedriver/current`).

## Isolation rules

| Rule | Requirement |
|---|---|
| Database | Temp file SQLite via allowlisted `TRACER_DATABASE_PATH` |
| Runtime | Fake ACP only (`TRACER_FAKE_ACP_JS`) |
| Heli | Empty probe dir for non-fatal path |
| Env injection | `tracer-desktop.exe --tracer-e2e-env=<absolute-file>` |
| Env schema | Allowlist only; absolute existing file; no arbitrary keys |
| Network | None for product path |
| Credentials | None |
| User DB | Never open developer/user profile DB |

## Selector contract

Priority (product + harness):

1. role + accessible name  
2. form label / `htmlFor`  
3. `data-testid="tracer-*"`  
4. state markers (`data-tracer-ready`, `data-session-status`, …)  
5. CSS last resort (avoid)

Product surfaces must keep visible labels and fail-closed invoke behavior. Testids are a stable product contract, not a substitute for removing a11y.

## Journey authenticity

Product steps for create session, prompt, approval allow/deny, cancel, multi-session open, and fail-closed register **must** go through real GUI controls.

Allowed non-GUI setup only:

- temp dirs, env file, fake ACP scenario selection via product scenario dropdown  
- driver/session lifecycle  
- diagnostic invoke (`tracer_e2e_env`, project list) for restart/env verification — **not** as prompt/approval shortcut  
- harness orphan reap after teardown

## Classification

| Result | Meaning |
|---|---|
| PASS | All selected journeys PASS |
| PARTIAL | Some partial outcomes (honest, non-fail product) |
| BLOCKED_BY_TOOLING | Drivers/binary missing |
| BLOCKED_BY_PRODUCT_GAP | Product cannot complete journey |
| FAIL | Journey/harness hard fail |

## Artifacts

On failure: `artifacts/tauri-e2e/<runId>/` with page dump (sanitized), probe, report. No secrets in dumps.

## Out of scope

- Live Grok / live-provider GUI  
- Cross-platform GUI guarantees  
- Wave 2.3 IDE/ALMS/plugins/marketplace  
- Folding L3-J into default CI  
