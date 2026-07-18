# Wave 2.1 Integration Report

**Gate:** 2.1 (Wave 2 runtime polish: W2-A presentation, W2-C multi-session, W2-B desktop boundary E2E, W2-D opt-in live approval harness)  
**Task:** `tracer-w2-1-integration`  
**Work item:** W2.1-I  
**Integrator host:** `grok-build`  
**Heli session:** `heli-ses-64cd15e3-f5b1-4163-b208-b92c79f83300`  
**Write target:** `tracer` (`repos/tracer` main worktree)  
**Integration branch:** `integration/tracer-w2-1`  
**Date:** 2026-07-18  
**Platform:** Microsoft Windows · rustc/cargo 1.96.0 · Node · pnpm

## 1. Gate 2.1 decision

| Field | Value |
|---|---|
| **Gate 2.1** | **PASS** |
| Local product reliability | **PASS** — fake ACP path green; sequence isolation + multi-session focus hold |
| Desktop boundary readiness | **PASS** (L0+L1) |
| Packaged Tauri smoke | **PARTIAL** — `node tools/tauri-e2e/run.mjs` desktop-boundary harness PASS; not a signed installer smoke |
| Full WebView GUI E2E | **NOT DONE** — honest classification: desktop-boundary only (no tauri-driver/WebView drive) |
| Live approval readiness | **PARTIAL** — LVA harness integrated opt-in only; LVA-01..04 **NOT_OBSERVED** (no live credentials run) |
| Wave 2.2 authorization | **YES (entry criteria document only)** — no Wave 2.2 tasks claimed by this gate |
| Network / live Grok credentials | **Not used** for gate validation — standard CI is fake ACP only |

## 2. Bootstrap evidence

| Check | Result |
|---|---|
| WORKSPACE | `D:\KJ\repo\tracer-lab` |
| Main at start | `56715cc79047d22e4c66a2a8ba257ee7b68d1f3e` (`tracer-vs1-hardened`) |
| `repos/grok-build` | **Not modified** |
| Task claim | write · host `grok-build` · session `heli-ses-64cd15e3-f5b1-4163-b208-b92c79f83300` |
| Target | `heli target set tracer` → `repos/tracer` |
| Worktree bind | `d:/kj/repo/tracer-lab/repos/tracer` (re-pointed from lease default) |
| Push | **Never** |

## 3. Source branches and tip SHAs (pre-merge)

| Order | Work item | Branch | Tip SHA |
|---|---|---|---|
| 1 | W2-A Presentation delivery | `agent/tracer-w2-presentation-delivery` | `ca53c8fe60de11510115a5cd3ce914985c8a3495` |
| 2 | W2-C Multi-session | `agent/tracer-w2-multi-session` | `c81295eb9c4df9bd6d3bd617ba2773d451f0323b` |
| 3 | W2-B Tauri GUI E2E | `agent/tracer-w2-tauri-gui-e2e` | `add65675ffd47d0aa163fd73846154c82161c37b` |
| 4 | W2-D Live approval validation | `agent/tracer-w2-live-approval-validation` | `5b9820e53bfefb1f6cdaa3afd679a8b255741eee` |

**Merge order:** A → C → B → D (`--no-ff`).

## 4. Integration merge commits (non-FF)

| Order | SHA | Message |
|---|---|---|
| 1 | `34654e64bdf5019950d6732c5fb88d92336e3e67` | `merge(w2-a): presentation delivery hub and coalescing notify` |
| 2 | `6309b36ea15d28f4e60a957547c6366438ec9556` | `merge(w2-c): multi-session focus, isolation, shutdown_all` |
| 3 | `6f774910764c3d13674e350c7ed859307c4c01b0` | `merge(w2-b): desktop-boundary Tauri E2E and fail-closed invoke` |
| 4 | `d93041d9b86c8459673355dd44e447360485d161` | `merge(w2-d): opt-in live approval LVA harness` |

## 5. Post-merge integration commits

| SHA | Message |
|---|---|
| `232620b95efd6e04d13d787bec98e5f8967a1ddc` | `fix(w2.1): reconcile presentation hub with multi-session focus` |
| `c607117b98bb0172a0baf4afc5ec12026de6e0b3` | `fix(w2.1): desktop presentation_focus contract for multi-session` |
| `6e4654a23935398f351e4bbba47e0bbea1bba534` | `style(w2.1): rustfmt after A+C+B+D integration` |
| `1941efcfaebd2d916c1d8f6c33a550a3498fcffc` | `test(w2.1): stabilize soak07 metrics under post-return drain races` |
| `57c534750d22f3abb061a2c14df5b94364fa683f` | `docs(w2.1): Gate 2.1 integration reports and contracts` |

## 6. Conflicts and reconciliations

### 6.1 Mechanical conflicts

| Path | Sources | Resolution |
|---|---|---|
| `crates/tracer-control-plane/src/plane.rs` | W2-A hub vs W2-C `presentation_tx`/`snapshot` Mutex | **Semantic reconciliation:** sole field is `PresentationHub`; multi-session focus APIs retained |

W2-B and W2-D merged cleanly (ort).

### 6.2 Semantic reconciliations (focus + projection model)

| Rule | Implementation |
|---|---|
| Canonical projection | `PresentationHub` (snapshot-authoritative, revisioned) |
| Coalescing notify | `DEFAULT_NOTIFY_CAPACITY = 1` (deliberate) |
| Multi-session registry | `HashMap<String, Arc<LiveSession>>`; per-session sequences/adapters |
| Focus switch | `presentation_focus` forces focus via `publish_snapshot` |
| Create focus | `session_create` forces focus onto new session |
| Background work | `submit`/`cancel`/`approval` + post-persist path use `publish_session_update` (no focus steal) |
| Shutdown | `shutdown_all` stops registry + clears focus via hub |
| Tauri Send | No `MutexGuard<SessionRuntimeState>` held across `.await` |

### 6.3 Desktop / Tauri contract

- Registered `tracer_presentation_focus`
- TS invoke + mock backends updated
- Optional `PresentationSnapshot.revision`
- Journey: `journey_multi_session_presentation_focus_switch`
- Invoke policy remains fail-closed (no silent mock downgrade)

## 7. Validation aggregate (standard CI path)

| Check | Result |
|---|---|
| `cargo fmt --all --check` | PASS (after rustfmt commit) |
| `cargo check --workspace` | PASS |
| `cargo test --workspace -- --test-threads=1` | PASS |
| `cargo clippy --workspace --all-targets` | PASS (pre-existing allows; no -D failures blocking finish) |
| `pnpm install --frozen-lockfile` | PASS |
| `pnpm -r test` | PASS |
| `pnpm -r build` | PASS |
| `vs_scenarios` (`--test-threads=1`) | PASS (23) |
| `presentation_delivery` | PASS (19) |
| `multi_session` (`--test-threads=1`) | PASS (17 incl. MS-17) |
| `tracer-vs1-soak` (`--test-threads=1`) | PASS (8) |
| `tracer-vs1-stress` multi-session | PASS (3) |
| `live-grok-smoke` unit/dry | PASS (24) — no live network |
| `desktop_boundary_journey` | PASS (9) |
| `node tools/tauri-e2e/run.mjs` | PASS (desktop-boundary) |

### SOAK-01 style burst (Gate 1.4 continuity)

From soak01 run evidence: bridge capacity 256 under burst; `persist_errors=0`; event sequences monotonic; no sequence loss/dup claims on success path.

## 8. Residual risks

1. Full WebView GUI E2E still blocked on tauri-driver + WebView2 drive wiring.
2. Live approval LVA-01..04 remain **NOT_OBSERVED** until intentional live run with credentials.
3. Soak metrics can race post-return on drain; isolation proven by prompt success + own events (soak07 tightened).
4. Packaged desktop installer smoke not in this gate.
5. Clippy still emits pre-existing domain/process style warnings (not introduced by W2.1).

## 9. Wave 2.2

See `docs/integration/WAVE_2_2_ENTRY_CRITERIA.md`. **No Wave 2.2 tasks created or claimed.**

## 10. Final tip

| Item | Value |
|---|---|
| Integration / main tip | `7be64d47dd8e4d1a16b3d0b7b2f315c25266e699` |
| Local tag (on PASS finalize) | `tracer-wave2.1-runtime-polish` |
| Remote push | **none** |