# W2-B Completion Report — Real Tauri GUI Boundary E2E

**Task id:** `tracer-w2-tauri-gui-e2e`  
**Work item:** W2-B — Real Tauri GUI E2E  
**Branch:** `agent/tracer-w2-tauri-gui-e2e`  
**Base SHA:** `56715cc79047d22e4c66a2a8ba257ee7b68d1f3e`  
**Head SHA:** see commit table tip; base `56715cc`; deliverables `639db16`..`87f8629`  
**Session id:** `heli-ses-c1dbf8c3-34de-4440-a25b-5c84a56fda52`  
**Host:** grok-build  
**Target:** tracer  
**Date:** 2026-07-18  

## Decision

| Item | Result |
|---|---|
| Goal achieved | **Yes** — strongest practical automated Tauri journey |
| E2E strength classification | **`desktop-boundary-e2e`** (L0 + L1) |
| Full WebView GUI E2E (L3) | **Not claimed** — tauri-driver / WebView2 driver not wired |
| False full-GUI claim | **None** |
| CI class | standard — network no, credentials no, live Grok no, provider no |
| Fake ACP + temp file SQLite | **Yes** |
| Wave 2.1 / push / merge | **Not done** (integrator owns Gate 2.1 after W2-A) |

## Strength classification (honest)

```text
L3 Full GUI (WebDriver / Playwright + tauri-driver)  — NOT delivered (blocker documented)
L2 App process smoke (spawn packaged binary)         — NOT default CI
L1 Desktop boundary journey                          — DELIVERED (plane_* == Tauri handlers)
L0 Frontend invoke policy                            — DELIVERED (fail-closed Tauri)
```

**Primary claim:** executable **desktop-boundary E2E** through the same control-plane composition and `plane_*` command handlers the Tauri app registers, plus frontend policy that never silently mock-downgrades when Tauri is selected.

## Assertions 1–15

| # | Assertion | Status |
|---|---|---|
| 1 | App launches as Tauri | **Partial** — L1 composition path; L3 GUI launch blocked |
| 2 | Frontend detects Tauri; no silent mock | **Pass** (L0) |
| 3 | Tauri command registration valid | **Pass** (L1 + `tracer_e2e_env`) |
| 4 | Inspect snapshot | **Pass** |
| 5 | Start fake runtime | **Pass** |
| 6 | Create session | **Pass** |
| 7 | Submit prompt / running evidence | **Pass** |
| 8 | Approval path | **Pass** |
| 9 | Cancel path | **Pass** |
| 10 | Terminal state (stop) | **Pass** |
| 11 | History after restart | **Pass** |
| 12 | Heli unavailable without failure | **Pass** |
| 13 | No raw ACP to frontend surface | **Pass** (domain event types; adapter `runtimeMethod` provenance allowed) |
| 14 | Browser fallback deterministic without Tauri | **Pass** |
| 15 | Real Tauri invoke failure → error; no silent mock | **Pass** |

## Deliverables

### Code / harness

| Path | Role |
|---|---|
| `apps/desktop/src-tauri/src/commands/mod.rs` | `plane_*` testable handlers; `REGISTERED_COMMANDS`; `tracer_e2e_env` |
| `apps/desktop/src-tauri/src/control_plane/mod.rs` | E2E env hooks (`TRACER_DATABASE_PATH`, fake ACP, heli probe, node bin) |
| `apps/desktop/src-tauri/src/lib.rs` | Public modules; register `tracer_e2e_env`; resolve DB path on run |
| `apps/desktop/src-tauri/tests/desktop_boundary_journey.rs` | L1 boundary journeys (8 tests) |
| `apps/desktop/src/shared/commands/invoke.ts` | Fail-closed Tauri; export `isTauriAvailable` |
| `apps/desktop/src/shared/commands/invoke.policy.test.ts` | L0 policy (9 tests) |
| `apps/desktop/src/shared/store/snapshotStore.ts` | Optional runtime options on `createSession` for harness scenarios |
| `tools/tauri-e2e/` | Orchestrator + package (`run.mjs`, README) |
| `tests/e2e/tauri/README.md` | Entry pointer (no false GUI scripts) |

### Docs

| Path | Role |
|---|---|
| `docs/modules/w2-b/W2_B_E2E_ARCHITECTURE.md` | Architecture + layering + blockers |
| `docs/modules/w2-b/W2_B_TEST_MATRIX.md` | Assertion matrix + run commands |
| `docs/modules/w2-b/W2_B_COMPLETION_REPORT.md` | This report |

### Root manifest touch (minimal)

| Path | Change |
|---|---|
| `package.json` | `test:tauri-e2e` script only |
| `pnpm-lock.yaml` | workspace package `@tracer/tauri-e2e` |
| `Cargo.lock` | dev-dep graph for `tracer-desktop` tests |
| `apps/desktop/src-tauri/Cargo.toml` | dev-dependencies for boundary tests |

Integrator owns root manifests if conflicted at Gate 2.1.

## Validation evidence

```text
pnpm --filter @tracer/desktop exec vitest run src/shared/commands/invoke.policy.test.ts
  ✓ 9 tests passed

cargo test -p tracer-desktop --test desktop_boundary_journey -- --test-threads=1
  ✓ 8 tests passed
    a1_registered_commands_stable
    a2_app_info_and_snapshot_via_plane_handlers
    e2e_env_command_lists_registered
    journey_happy_prompt_stream_and_terminal
    journey_approval_allow_then_terminal
    journey_cancel_mid_stream
    journey_close_reopen_restores_history
    journey_heli_unavailable_non_fatal

node tools/tauri-e2e/run.mjs
  ✓ policy PASS
  ✓ boundary PASS
  ✓ gui-probe documented (fullGuiE2e: false)

pnpm --filter @tracer/desktop test
  ✓ 27 tests passed (policy + snapshot + mock store)

pnpm --filter @tracer/desktop typecheck
  ✓ pass
```

**Preflight note:** Tauri `generate_context!()` requires `apps/desktop/dist` at compile time. `tools/tauri-e2e/run.mjs` creates a gitignored stub when missing so cargo test does not need a full Vite build.

## L3 full-GUI blocker (explicit)

| Blocker | Detail |
|---|---|
| `tauri-driver` / WebView2 WebDriver | Not installed or wired in standard CI |
| Packaged app launch under test env | Not automated (SDK-heavy; optional L2) |
| Playwright/Selenium WebView scripts | Not authored — would claim L3 without driver |

Follow-up path is documented in architecture §3 / tools README. **Do not treat this task as full GUI E2E.**

## Owned path compliance

| Path | Action |
|---|---|
| `apps/desktop/` | Modified (glue, policy, journey tests) |
| `tests/e2e/tauri/` | Added README |
| `tools/tauri-e2e/` | Added harness |
| `docs/modules/w2-b/` | Architecture, matrix, completion |
| Control-plane redesign | **Not done** (read-only use) |
| Multi-session CP (W2-C) | **Not touched** |
| Live Grok (W2-D) | **Not touched** |
| IDE / ALMS / plugins | **Not touched** |

## Assumptions

1. `plane_*` handlers are byte-for-byte the same logic Tauri `#[tauri::command]` wrappers call (thin State unwrap).
2. Presentation snapshot remains v1 camelCase until W2-A lands; Gate 2.1 re-validates field names.
3. Adapter `runtimeMethod` strings (e.g. `session/update`) are provenance metadata, not raw ACP event types on the frontend surface.
4. Node is available on PATH for fake ACP spawn.
5. Boundary tests serialize via process-global mutex + `--test-threads=1` on Windows.

## Risks / integration notes

| Risk | Mitigation |
|---|---|
| W2-A changes snapshot / live events | Gate 2.1 re-run boundary + policy; do not weaken fail-closed invoke |
| Root manifest conflicts | Integrator merges `package.json` / lockfiles |
| L3 still missing for product demos | Document preferred path; optional later host with tauri-driver |
| `frontendDist` missing in clean CI | Orchestrator stub; or run Vite build before cargo |

## Commit SHAs

| Commit | Contents |
|---|---|
| `639db16` | desktop Tauri glue + plane handlers + E2E env + fail-closed invoke |
| `fe60017` | boundary journey + invoke policy tests |
| `9976f9b` | tools/tauri-e2e harness + e2e entry README |
| `87f8629` | docs/modules/w2-b architecture, matrix, completion |
| `087cb0c`+ | completion report SHA pin commits (branch tip) |

## Lease

- Released: yes (end of task)
- Push: **no**
