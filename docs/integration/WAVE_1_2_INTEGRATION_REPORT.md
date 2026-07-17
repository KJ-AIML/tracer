# Wave 1.2 Integration Report — ACP Client / Runtime Adapter (Gate 1.2)

**Gate:** 1.2 (Wave 1.2 ACP adapter: W1-D)  
**Task:** `tracer-w1-d-integration`  
**Work item:** W1.2-I / Gate 1.2  
**Integrator host:** `grok-build`  
**Heli session:** `heli-ses-e1a2babc-aa49-42d0-ba13-f0b2c96283cf`  
**Lease:** `heli-lease-8e60df74-da8e-407a-8776-78ed370cdf2c`  
**Write target:** `tracer` (`repos/tracer` main checkout)  
**Integration branch:** `integration/tracer-w1-d`  
**Date:** 2026-07-17  
**Platform:** Microsoft Windows NT 10.0 · rustc/cargo 1.96.0 · Node v24.16.0 · pnpm 9.15.0

## 1. Gate 1.2 decision

| Field | Value |
|---|---|
| **Gate 1.2** | **PASS** |
| **W1-F authorization** | **YES** — control plane may claim and integrate against the public adapter API documented in `W1_F_HANDOFF_CONTRACT.md` |
| Material unresolvable contradictions | **None** |
| Reconciliation class | Workspace registration **inherited and verified** from W1-D feature commits (no post-merge Cargo/pnpm edit required); semantic API implementation-backed; test-count inventory reconciled in matrix |
| Network / live Grok credentials | **Not used** — standard CI path is fake ACP only |
| Live smoke | **Not performed** (optional; credentials not intentionally exercised) |

**Explicit statement:** Because **Gate 1.2 is PASS**, W1-F (control plane integration) is **authorized** once this integration is on `main`. **This task does not create or start W1-F.**

## 2. Bootstrap evidence

| Check | Result |
|---|---|
| WORKSPACE_ROOT | `D:\KJ\repo\tracer-lab` |
| Main at start | `bfcd205832fd9befa9d78dd204cb6916c7ad6385` (Gate 1.1 / tag `tracer-wave1.1-foundation`), clean |
| W1-D tip not ancestor of main | **Confirmed** (`2ba88a8` not ancestor of `bfcd205`) |
| W1-D tip | `2ba88a860a3b814402f92c933dfcfd4b42cdd250` |
| W1-D commits | `128f887` feat; `2ba88a8` docs |
| `repos/grok-build` | Clean main `8adf9013a0929e5c7f1d4e849492d2387837a28d` — **not modified** |
| Task claim | `tracer-w1-d-integration` write · host `grok-build` |
| Target | `heli target set tracer` → `repos/tracer` |
| W1-F branch / work | **None** (not created, not started) |
| Push | **Never** |

## 3. Source branch and tip SHAs (pre-merge)

| Work item | Branch | Tip SHA |
|---|---|---|
| W1-D ACP client + runtime adapter | `agent/tracer-w1-acp-adapter` | `2ba88a860a3b814402f92c933dfcfd4b42cdd250` |
| Base (main) | `main` | `bfcd205832fd9befa9d78dd204cb6916c7ad6385` |

## 4. Integration merge commit (non-FF)

| Order | SHA | Message |
|---|---|---|
| 1 | `13f1bbb12ee65b293b28fe0f95653bb8c74712ff` | `merge(w1-d): ACP client and runtime adapter into integration/tracer-w1-d` |

**Merge strategy:** `git merge --no-ff agent/tracer-w1-acp-adapter`  
**Conflicts:** **None** (clean ort merge). Provenance of `128f887` / `2ba88a8` preserved.

## 5. Post-merge integration commits

| SHA | Message | Notes |
|---|---|---|
| *(this docs tip)* | `docs(w1.2): Gate 1.2 integration report, test matrix, W1-F handoff` | Artifacts only |

### 5.1 Workspace registration reconciliation

**No separate Cargo/pnpm reconciliation commit required.**

W1-D already registered workspace members in the feature commit (`128f887`):

```text
members += crates/tracer-acp-client, crates/tracer-runtime-adapter
workspace.dependencies += tracer-acp-client, tracer-runtime-adapter
```

Integrator verification after merge:

| Check | Result |
|---|---|
| Root `Cargo.toml` members include both crates | **Yes** |
| Path deps under `[workspace.dependencies]` | **Yes** |
| `cargo check --workspace` | **PASS** |
| `cargo test -p tracer-acp-client` / `-p tracer-runtime-adapter` | **PASS** |
| pnpm workspace change for optional TS `packages/runtime-client` | **Not added** (Rust API sufficient for W1-F; documented in `SHARED_MANIFEST_REQUESTS.md`) |

### 5.2 Semantic API reconciliation

**None required.** Public surface matches W1-F handoff ops (see §8 and `W1_F_HANDOFF_CONTRACT.md`). All claimed ops are implementation-backed.

### 5.3 Test / fixture corrections

**None required.** Fake-runtime vertical slice and codec/SM suites pass as landed.

## 6. Ownership validation

| Concern | Owner | Evidence |
|---|---|---|
| Spawn / stdio / readiness / exit / cleanup | `tracer-process` | Adapter composes `ProcessManager` / `ManagedProcess`; no second process manager |
| Envelopes / IDs / sequences / states / errors | `tracer-domain` | Adapter depends on domain; emits `EventEnvelope` only |
| ACP framing + protocol SM | `tracer-acp-client` | Transport/codec/state; no process ownership |
| Normalize + public lifecycle API | `tracer-runtime-adapter` | W1-F-facing surface |
| SQLite / sole writer | **Not W1-D** | No `sqlx`/`sqlite` deps or code in W1-D crates |
| UI / React / Tauri commands | **Not W1-D** | No desktop/UI ownership |
| Control plane | **W1-F (not started)** | No control-plane crate |
| Fake runtime | credential/network free, synthetic labeled | `fake_scenarios` + W1-G `no-network` suite |
| Raw ACP / Grok events to React | **Forbidden and not done** | Normalized envelopes only |

## 7. State machine readiness boundaries

Proven distinctions (process alive ≠ protocol ready ≠ authenticated ≠ session ready ≠ prompt complete):

| Claim | Evidence |
|---|---|
| process-alive ≠ protocol-ready | `process_ready_not_session_ready_apis`; acp-client `process_ready_not_authenticated_or_session_ready`; W1-C process lifecycle |
| protocol-ready ≠ authenticated | `auth_required_no_session_ready`; SM `auth_required_blocks_session_ready` |
| process/protocol ready ≠ session-ready | `create_session` only sets session ready; APIs `is_session_ready()` |
| session-ready ≠ prompt-complete | SM `session_ready_not_prompt_complete`; `may_accept_prompt` while prompt active is false |
| Invalid transitions → typed failures | acp-client `invalid_transition_errors`; adapter `AdapterError::invalid_state` / `not_ready` |

## 8. Public interface (W1-F ops)

| Op | Method | Status |
|---|---|---|
| start | `RuntimeAdapter::start(spec, project_id, session_id)` | **Implemented** |
| initialize | `initialize() -> Result<Capabilities, AdapterError>` | **Implemented** |
| inspect auth | `inspect_auth_requirement() -> AuthenticationState` | **Implemented** |
| authenticate | `authenticate(method_id: Option<&str>)` | **Implemented** |
| create session | `create_session(SessionCreateParams)` | **Implemented** |
| submit prompt | `submit_prompt(PromptRequest)` (blocking) | **Implemented** |
| resolve approval | `resolve_approval(ApprovalDecisionRequest)` | **Implemented** (never auto) |
| cancel | `cancel_prompt()` | **Implemented** |
| subscribe events | `take_event_receiver` / `try_recv_event` / `drain_events` / `wait_event` | **Implemented** |
| inspect state | `inspect()` / readiness helpers | **Implemented** |
| shutdown | `shutdown(ShutdownOptions)` / `force_kill()` | **Implemented** |

Full signatures: `docs/integration/W1_F_HANDOFF_CONTRACT.md`.

## 9. Aggregated validation summary

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all --check` | **PASS** | Clean after merge |
| `cargo check --workspace` | **PASS** | Includes new members |
| `cargo test --workspace` | **PASS** | 0 failures; see test matrix |
| `cargo clippy --workspace --all-targets` | **PASS** | Completes exit 0. **No new W1-D clippy warnings.** Inherited Gate 1.1 nits in domain/process/storage only (documented) |
| `pnpm install --frozen-lockfile` | **PASS** | No lock change required |
| `pnpm -r test` | **PASS** | event-types 11, ui 3, desktop 4, fake-runtime 30 |
| `pnpm -r build` | **PASS** | event-types, ui, desktop |

**Credentials / network:** No Grok/XAI API keys required or used. `TRACER_LIVE_SMOKE` unset. Fake runtime `no-network` tests pass.

**Platform-gated:** Windows Job Object orphan test (`force_kill_reaps_grandchild_no_orphan`) **ran and passed** on this Windows host (W1-C regression).

## 10. Test-count reconciliation

| Source | Claim | Actual on integration host |
|---|---|---|
| W1-D completion report | `tracer-acp-client` **15**; `tracer-runtime-adapter` **23** (via `--lib` 3 + `fake_scenarios` 20) | **15** + **3** + **20** = **38** W1-D tests, all PASS |
| “Coordinator focused rerun 3+20” | runtime-adapter lib + integration only | Matches **3** lib + **20** `fake_scenarios` |
| “Subagent 15+23” | acp-client all + runtime-adapter all | Matches **15** + **(3+20=23)** |

**Difference explanation:** Not a real discrepancy. **15+23** aggregates per-crate totals; **3+20** is the runtime-adapter suite split (unit vs fake vertical slice). Gate evidence is **named scenarios**, not a bare integer.

Misleading if read as “38 vs 23 failures” — there were none. Completion report wording is consistent once both notations are shown.

## 11. Live smoke classification

| Classification | Value |
|---|---|
| Performed? | **No** |
| Reason | Optional path; not required for Gate 1.2; credentials not intentionally used |
| Gate impact | **None** — standard acceptance is fake-ACP only |

## 12. What was not done (by design)

- W1-F control plane **not created / not started**
- No push to origin
- No edits under `repos/grok-build`
- No headless/watchdog launchers
- No live provider smoke as gate requirement
- No optional TS `packages/runtime-client`

## 13. Residual risks / notes for W1-F

1. Unbounded event `mpsc` — control plane must drain promptly.
2. Blocking `submit_prompt` requires concurrent threads for cancel/approval.
3. Adapter assigns live-stream `sequence`/`eventId`; W1-F remains sole SQLite writer and may re-key.
4. Stock Grok path helper exists (`grok_stdio_spawn_config`) but is **not** CI-proven authenticated on this gate.
5. Inherited clippy style nits on domain IDs / sequence naming remain foundation debt (not W1-D regressions).

## 14. Finalization (when Gate PASS)

| Step | Status |
|---|---|
| Integration commits clean | Yes |
| `git checkout main && git merge --ff-only integration/tracer-w1-d` | Performed after docs commit |
| Annotated tag `tracer-wave1.2-acp-adapter` | Applied on final main tip |
| Push | **Never** |
| Release lease with session ID | After main finalization |

## 15. Ahead/behind (local, after finalize)

Recorded at finalize time against `origin/main` (local only; nothing pushed). See final return block / post-merge `git status -sb`.
