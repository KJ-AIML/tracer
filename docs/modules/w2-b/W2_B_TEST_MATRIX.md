# W2-B Test Matrix — Tauri Desktop Boundary E2E

**Task:** `tracer-w2-tauri-gui-e2e`  
**CI class:** standard — network no, credentials no, live Grok no, provider no  
**Evidence:** fake-ACP + temp SQLite + frontend policy tests  
**Classification:** `desktop-boundary-e2e` (not full WebView GUI E2E)

## 1. Assertion coverage

| # | Assertion | Layer | Location | Status |
|---|---|---|---|---|
| 1 | App launches as Tauri | L2/L3 GUI | Documented follow-up; composition path proven via L1 | **Partial** — L1 composition; L3 blocked |
| 2 | Frontend detects Tauri; does not silently use mock | L0 | `invoke.policy.test.ts` | **Pass** |
| 3 | Tauri command registration valid | L1 | `a1_registered_commands_stable` + `tracer_e2e_env` | **Pass** |
| 4 | Inspect snapshot | L1 | `journey_happy_*` / `a2_*` | **Pass** |
| 5 | Start fake runtime | L1 | `session_create` + `scenarioId` | **Pass** |
| 6 | Create session | L1 | `plane_session_create` | **Pass** |
| 7 | Submit prompt / running evidence | L1 | submit + events poll | **Pass** |
| 8 | Approval path | L1 | `journey_approval_allow_then_terminal` | **Pass** |
| 9 | Cancel path | L1 | `journey_cancel_mid_stream` | **Pass** |
| 10 | Terminal state (stop) | L1 | `plane_session_stop` | **Pass** |
| 11 | History after restart | L1 | `journey_close_reopen_restores_history` | **Pass** |
| 12 | Heli unavailable without failure | L1 | `journey_heli_unavailable_non_fatal` | **Pass** |
| 13 | No raw ACP to frontend surface | L1 | `assert_no_raw_acp` (domain `type`; not ACP methods as event types; `runtimeMethod` provenance OK) | **Pass** |
| 14 | Browser fallback deterministic without Tauri | L0 | `invoke.policy.test.ts` A14 | **Pass** |
| 15 | Real Tauri invoke failure → error; no silent mock downgrade | L0 | `invoke.policy.test.ts` A15 | **Pass** |

## 2. Executable suites

| Suite | Command | Threads |
|---|---|---|
| Policy | `pnpm --filter @tracer/desktop exec vitest run src/shared/commands/invoke.policy.test.ts` | default |
| Boundary | `cargo test -p tracer-desktop --test desktop_boundary_journey -- --test-threads=1` | 1 (Windows fake-ACP) |
| Orchestrator | `node tools/tauri-e2e/run.mjs` | policy then boundary |

## 3. Fake ACP scenarios used

| Scenario | Journey |
|---|---|
| `happy_prompt_stream` | prompt/stream/terminal + reopen history |
| `permission_allow` | approval allow |
| `cancel_mid_stream` | cancel during stream |

## 4. Explicit non-goals (this task)

| Not claimed | Reason |
|---|---|
| Full Playwright/WebDriver GUI E2E | tauri-driver / WebView2 driver not wired in standard CI |
| Live Grok | Forbidden in standard class (W2-D owns live) |
| Control-plane redesign | W2-A owns presentation delivery |
| Multi-session isolation | W2-C |

## 5. Gate 2.1 re-validation note

After W2-A presentation contract changes:

- Re-run boundary journey; assert snapshot field names still match frontend `PresentationSnapshot`.
- If live `tracer://events` fan-out lands, add optional L1 subscription check (still no full GUI required).
- Do not weaken fail-closed invoke policy tests.

## 6. How to run (Windows PowerShell)

```powershell
cd repos/worktrees/tracer-w2-b
pnpm install
$env:TRACER_FAKE_ACP_JS = (Resolve-Path tools/fake-acp-runtime/bin/fake-acp-runtime.js).Path
node tools/tauri-e2e/run.mjs
pnpm --filter @tracer/desktop test
pnpm --filter @tracer/desktop typecheck
```
