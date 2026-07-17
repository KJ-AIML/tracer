# Final Gate 0 Report â€” Tracer Wave 0 Complete Integration

**Gate:** 0 (full Wave 0: W0-A + W0-B + W0-C + W0-D)  
**Task:** `tracer-w0-final-integration`  
**Work item:** W0-FINAL  
**Integrator host:** `grok-build`  
**Heli session:** `heli-ses-7c2f27d1-a702-4c12-b07f-9962714c8c09`  
**Lease:** `heli-lease-a1cb1862-7eef-4730-b41a-aee98270bec6`  
**Write target:** `tracer` (`repos/tracer` main checkout)  
**Integration branch:** `integration/tracer-w0-final`  
**Date:** 2026-07-17

## 1. Gate 0 decision

| Field | Value |
|---|---|
| **Gate 0** | **PASS** |
| **Wave 1 authorization** | **YES** â€” foundation modules W1-Aâ€¦W1-H may be claimed after this lands on `main` |
| Material unresolvable contradictions | **None** |
| Reconciliation class | Documentation integration only (no contract rewrites; no application source) |
| Docs-only integration validity | Valid â€” all deliverables under `docs/` and `tests/{fixtures,specifications}/` |

**Explicit Wave 1 authorization statement**

Because **Gate 0 is PASS**, Wave 1 foundation implementation tasks are **authorized** once this integration is fast-forwarded onto `main`:

- W1-A Desktop Shell  
- W1-B Domain and Event Protocol  
- W1-C Runtime Process Manager  
- W1-D ACP Client and Runtime Adapter  
- W1-E Storage and Session Persistence  
- W1-F Control Plane Integration  
- W1-G Fake Runtime and Contract Harness  
- W1-H HeliHarness Workspace Integration  

**This task does not begin Wave 1 implementation.**

## 2. All Wave 0 tasks

| Work item | Task ID | Branch | Tip SHA | Status |
|---|---|---|---|---|
| W0-A Architecture & Contracts | `tracer-w0-architecture-contracts` | `agent/tracer-w0-architecture-contracts` | `aa7b778d7f1c571e225b5727dd2dd6bb80c2ebef` | Integrated at Gate 0.1 |
| W0-B Grok Runtime Recon | `tracer-w0-grok-runtime-recon` | `agent/tracer-w0-grok-runtime-recon` | `a141d00bc0c2a54ecb9a9c3045b2f04a91bfd524` | Integrated at Gate 0.1 |
| W0-I Stage 0.1 Integration | `tracer-w0-integration-ab` | `integration/tracer-w0-ab` â†’ `main` | Stage report on main | Gate 0.1 **PASS** |
| W0-C Product UX | `tracer-w0-product-ux` | `agent/tracer-w0-product-ux` | `23a66eaf3a91c63dffe4e462d30b759004e0b871` | Merged this gate |
| W0-D Test Strategy | `tracer-w0-test-strategy` | `agent/tracer-w0-test-strategy` | `977ddee928adcaa39770436007e5af5c0bf7dbc8` | Merged this gate |
| W0-FINAL | `tracer-w0-final-integration` | `integration/tracer-w0-final` â†’ `main` | See Â§12 | This report |

### Prior Gate 0.1 context

Main tip at Final Wave 0 start:

```text
5b936412b982cc4310f1196caef023a968ea070a
```

That tip already contained W0-A, W0-B, Stage 0.1 integration report, and reconciliation note on `ACP_EVENT_MAPPING.md`.

## 3. Source branches, tip SHAs, merge commits

### 3.1 Sources for this final integration

| Source | Branch | Tip SHA | Base at worker close |
|---|---|---|---|
| W0-C | `agent/tracer-w0-product-ux` | `23a66eaf3a91c63dffe4e462d30b759004e0b871` | `5b936412b982cc4310f1196caef023a968ea070a` |
| W0-D | `agent/tracer-w0-test-strategy` | `977ddee928adcaa39770436007e5af5c0bf7dbc8` | `5b936412b982cc4310f1196caef023a968ea070a` |
| Main at start | `main` | `5b936412b982cc4310f1196caef023a968ea070a` | â€” |

### 3.2 Worker commit chains integrated

**W0-C**

| SHA | Message |
|---|---|
| `0cda243831401d0ec6907044e2ea9a35264c3a49` | `docs(w0-c): product UX IA, session screen, state matrix, flows` |
| `04269ba1ff587c23ffb7a5192a22844c162dac8b` | `docs(w0-c): completion report` |
| `23a66eaf3a91c63dffe4e462d30b759004e0b871` | `docs(w0-c): record completion report commit SHA` |

**W0-D**

| SHA | Message |
|---|---|
| `a28d634084be43359e20e354f8f66f3c8619dcc0` | `docs(w0-d): test strategy, acceptance criteria, failure matrix, specifications` |
| `c71f19727017c3e441aed8b2cc90ac34a2edcfa5` | `docs(w0-d): completion report` |
| `5f8c9d6cbc40a4376f4df9a79df36eec86704efa` | `docs(w0-d): correct completion report commit SHA` |
| `977ddee928adcaa39770436007e5af5c0bf7dbc8` | `docs(w0-d): finalize completion report SHA table` |

### 3.3 Integration merge commits (non-FF, exact order)

| Order | SHA | Message |
|---|---|---|
| 1 | `eb9c7b14df3949b684c049e21444cbd102236e15` | `merge(w0-c): product UX into integration/tracer-w0-final` |
| 2 | `1cdf3383febbe6e289f664b4d0d4c33bc961dbea` | `merge(w0-d): test strategy into integration/tracer-w0-final` |

### 3.4 Reconciliation / Gate 0 artifact commits

| SHA | Message | Notes |
|---|---|---|
| *(filled at close)* | `docs(w0-final): Final Gate 0 report, readiness and traceability matrices` | This report + readiness + RTM |
| *(none required)* | semantic contract rewrites | No material contradictions required edits to W0-A/B/C/D bodies |

### 3.5 Prior Stage 0.1 merge commits (already on main)

| SHA | Message |
|---|---|
| `05f29518a522e3707700b30b8ebecfb116fe2dce` | `merge(w0-a): architecture contracts into integration/tracer-w0-ab` |
| `da9ccacc81af605a2373028ea5d27b8eac6afa85` | `merge(w0-b): grok runtime recon into integration/tracer-w0-ab` |
| `c0b37a0ad04ccb6c7b5438d12cb4e43b2ecd7be9` | `docs(w0-i): stage 0.1 integration report and reconciliation notes` |

## 4. Documents inspected

### Coordinator / harness

- `.heli-harness/HARNESS.md`
- `resources/TRACER_MASTER_BUILD_PLAN.md` (Wave 0 + Wave 1 module map)
- `resources/TRACER_WAVE0_EXECUTION_AMENDMENT.md`
- workspace `AGENTS.md` (pointer)

### W0-A (on main)

- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`
- `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md`
- `docs/architecture/TRACER_VERTICAL_SLICE.md`
- `docs/architecture/W0-A_COMPLETION_REPORT.md`
- `docs/decisions/ADR-001-runtime-sidecar.md`
- `docs/decisions/ADR-002-event-normalization.md`

### W0-B (on main)

- `docs/research/grok-build/CAPABILITY_MATRIX.md`
- `docs/research/grok-build/PROCESS_LIFECYCLE.md`
- `docs/research/grok-build/ACP_EVENT_MAPPING.md`
- `docs/research/grok-build/FORK_RISK_REPORT.md`
- `docs/research/grok-build/W0-B_COMPLETION_REPORT.md`
- `tests/fixtures/acp/*` (README + JSON/JSONL)

### Stage 0.1

- `docs/integration/STAGE_0_1_INTEGRATION_REPORT.md`

### W0-C (merged this gate)

- `docs/ux/INFORMATION_ARCHITECTURE.md`
- `docs/ux/SESSION_SCREEN_SPEC.md`
- `docs/ux/STATE_MATRIX.md`
- `docs/ux/INTERACTION_FLOW.md`
- `docs/ux/W0-C_COMPLETION_REPORT.md`

### W0-D (merged this gate)

- `docs/testing/TEST_STRATEGY.md`
- `docs/testing/VERTICAL_SLICE_ACCEPTANCE.md`
- `docs/testing/FAILURE_MATRIX.md`
- `docs/testing/W0-D_COMPLETION_REPORT.md`
- `tests/specifications/README.md`
- `tests/specifications/scenarios/catalog.yaml`
- `tests/specifications/ci/matrix.yaml`
- `tests/specifications/expected-events/*.json` (15 packs)

### Diff scope vs main (pre-merge)

- **W0-C:** 5 files under `docs/ux/` only (+1580 lines)  
- **W0-D:** 22 files under `docs/testing/` and `tests/specifications/` only (+2140 lines)  
- **Path overlap between C and D:** none  
- **Mechanical merge conflicts:** none  

## 5. Semantic reconciliation

### 5.1 Domain vocabulary â€” **aligned**

| Term | Authority | W0-C | W0-D |
|---|---|---|---|
| Project | W0-A vertical slice | Projects home / register | VS-14 path missing |
| Runtime installation / process | W0-A + W0-B | RuntimePill orthogonal to session | F-P* process matrix |
| Session | W0-A status catalog | StatusChip binds catalog | All VS cases |
| Agent run | W0-A | Timeline activity matrix | prompt/tool scenarios |
| Event | W0-A protocol types | Timeline binds `type` strings | Expected-event packs |
| Tool call | W0-A `tool.*` | Tool cards | permission/tool scenarios |
| Approval | W0-A `approval.*` only | No parallel `permission.*` product events | VS-03 / F-C* |
| Capability | Adapter contract | Progressive disclosure matrix | VS-11/12 / F-R08 |
| Authentication state | Stage 0.1 + W0-B | First-class auth matrix | VS-02 / F-A* |

No parallel incompatible UX/test vocabulary found. Product event names are exclusively W0-A strings; W0-B conceptual names appear only as **forbidden aliases** in expected-event packs (e.g. `permission.requested`, `message.agent.delta`).

### 5.2 Session lifecycle â€” **aligned**

Shared W0-A status set used by UX and tests:

```text
creating Â· starting_runtime Â· ready Â· running Â· awaiting_approval Â·
cancelling Â· completed Â· failed Â· disconnected Â· stopped
```

| Lifecycle concern | Contract / recon | UX | Tests |
|---|---|---|---|
| Runtime not installed | spawn errors | Flow C; Runtime unavailable | F-P01 |
| Runtime starting | process started | StatusChip + RuntimePill starting | process scenarios |
| Process ready (init+caps) | `runtime.process.ready` | Pill ready; not alone prompt-ready | Gate P/I vs A |
| Auth required / failed | live-scrubbed fixture; additive error classes | AuthSetupPanel + banners | VS-02; F-A01/A02 |
| ACP initialize + caps | W0-B NDJSON | Capabilities footer/banner | capability scenarios |
| Session creation | `session.ready` | Composer only when ready | VS-01/02 |
| Prompt / streaming | `session.prompt.submitted`, deltas | Timeline live | VS-01 |
| Waiting for approval | `awaiting_approval` | Interrupt fail-closed | VS-03/05 |
| Cancelling / cancelled | `cancelling` â†’ `stopped` | Cancel honesty | VS-04/05/11 |
| Completed | `completed` or back to `ready` | Terminal success; multi-turn policy open | VS-01 |
| Failed / disconnected / crash | exit/fail events | Never â€œRunningâ€ after exit | VS-06/07 |
| Recovery / resume after restart | storage + reconcile | History reload | VS-10; F-S04 |

Every visible first-slice UX state has a backend status/event/error source and a failure-matrix or VS ownership path.

### 5.3 Evidence boundary â€” **aligned**

| Label | Meaning | Where enforced |
|---|---|---|
| `synthetic` | Constructed; not live capture | fixtures README; catalog; UX honesty rules |
| `live-scrubbed` | Real runtime, secrets/paths scrubbed | `initialize-response`, `session-new-auth-required` |
| `fake-runtime` | Deterministic Tracer fake ACP | default CI; most VS cases |
| `live-authenticated` | Real auth / optional model usage | T6 only; not standard CI |
| `unit-generated` | Pure unit helpers | T0 |

**Non-claim (normative):** Synthetic `session-prompt-stream.jsonl` and permission/cancel fixtures do **not** prove live authenticated multi-turn tool/permission parity. Stage 0.1 did **not** exercise authenticated live prompt streams on Windows with credentials.

### 5.4 Runtime invocation â€” **aligned**

| Layer | Decision |
|---|---|
| Stock Grok discovery/spawn | **Authoritative:** `grok agent --no-leader stdio` (PATH executable `grok`) per Stage 0.1 / W0-B |
| W0-A adapter example args | Illustrative logical placeholders only (`runtimeKind: acp-stdio`); not stock CLI argv |
| Process startup | OS spawn + NDJSON pipes; no ready banner from runtime |
| Authenticated session creation | Separate gate after initialize; stock requires `authenticate` before usable `session/new` |
| Live Windows authenticated session | **Not claimed** â€” recon blocked by missing credentials; CI uses fake |
| Platform | Windows Job Object kill-on-close documented; not generalized as identical on all OS |

### 5.5 ACP compatibility â€” **aligned**

| Topic | Decision |
|---|---|
| MVP surface | Standard ACP core only |
| Vendor `x.ai/*` | Optional later; unknown â†’ `adapter.protocol.unknown` |
| UI coupling | React never parses raw ACP / vendor frames (`ADR-002`) |
| Framing | NDJSON JSON-RPC 2.0 for stock + fake |
| Tests | Contract packs assert W0-A types; vendor scenario is non-blocking for MVP |

### 5.6 UX-to-test critical path map â€” **aligned**

| UX / product condition | Backend condition | Fake / fixture scenario | Acceptance / failure | Recovery |
|---|---|---|---|---|
| Runtime missing | Spawn fail | fake spawn error | F-P01; Flow C | Fix PATH/config; new session |
| Auth required | `session/new` âˆ’32000 | `auth_required_session_new` + live-scrubbed fixture | VS-02; F-A01 | Authenticate then session |
| Auth failed | auth error | fake scripted / optional live | F-A02 | Retry method / stop |
| Capability unsupported | soft/hard mismatch | `cancel_unsupported`, `capability_minimal` | VS-11/12; F-R08 | Fallback or different runtime |
| Process up, session create failed | dual status | auth or protocol fail paths | Flow D; F-A01 | Stop process; retry |
| Malformed protocol | bad frame | `malformed_frame` | VS-08; F-R01 | Continue or stop if wedged |
| Runtime EOF | pipe death mid-prompt | `eof_mid_prompt` | VS-07; F-P07 | New process |
| Crash | non-zero exit | `crash_nonzero_exit` | VS-06; F-P06 | New session; no â€œrunningâ€ |
| Approval requested | reverse-request | `permission_allow` / `deny` | VS-03; F-C06/07 | Allow/deny |
| Approval rejected / fail closed | deny | `permission_deny` | VS-03; F-C09 | Continue honestly |
| Cancellation | cancel mid-stream | `cancel_mid_stream` | VS-04; F-C01 | Stop or new prompt |
| Cancel while permission pending | park + cancel | `cancel_while_permission_pending` | VS-05; F-C04 | Must not deadlock |
| Successful completion | stream complete | `happy_prompt_stream` | VS-01 | Review / stop |
| Session restore after app restart | storage reconcile | VS-10 (+ F-S04) | T3 | History only; no fake running |

### 5.7 MVP scope fence â€” **aligned**

All Wave 0 streams exclude from Gate 1 / MVP:

- full IDE / multi-file primary editor  
- multi-tenant SaaS / remote collab  
- distributed scheduler / full ALMS  
- mandatory Grok Build fork  
- production plugin marketplace  

Future-only sections are labeled as such in IA, vertical slice, and test strategy.

### 5.8 Contradictions and resolutions

| Topic | Finding | Resolution |
|---|---|---|
| Event naming (W0-B conceptual vs W0-A) | Already resolved at Gate 0.1 | **W0-A normative**; W0-D forbids conceptual aliases |
| Stock spawn argv vs W0-A examples | Already resolved at Gate 0.1 | Stock Grok argv is W0-B; W0-A examples illustrative |
| Auth error classes missing from frozen contract | Additive W1 recommendation | UX + tests use structured `lastError` / `errorClassAnyOf` bridge |
| Multi-turn: `completed` vs return to `ready` | Policy open in W0-A/C | UI binds live status; tests accept either terminal success shape |
| Live authenticated parity | Unverified | Explicit non-claim; optional T6 only |
| Parallel permission product events | Risk if reintroduced | W0-C and W0-D both forbid; approvals = `approval.*` only |

**No material unresolvable contradiction.** No contract rewrite required for Gate 0.

## 6. Decisions made during final integration

1. Merge order on `integration/tracer-w0-final`: **W0-C first**, then **W0-D**, each `git merge --no-ff`.  
2. Fast-forward `main` only after Gate 0 PASS (this document).  
3. Retain Stage 0.1 authority split: **W0-A** normative product contracts; **W0-B** stock wire evidence.  
4. **W0-C** owns UX binding; **W0-D** owns acceptance/failure strategy; neither rewrites contracts.  
5. No new ADR required (ADR-001, ADR-002 remain sufficient).  
6. Wave 1 may add `AuthenticationRequired` / `AuthenticationFailed` as additive contract classes (Gate 1 reviewers).  
7. No Wave 1 application scaffolding in this gate.  
8. No remote push.  
9. `repos/grok-build` remains clean / unmodified.

## 7. Live vs synthetic evidence boundary (summary)

| Evidence class | Gate 0 claim allowed? |
|---|---|
| Live-scrubbed initialize + auth-required error shapes | Yes (structural mapping) |
| Synthetic stream / permission / cancel fixtures | Yes (structural only) |
| Fake-runtime scenario design (not yet implemented) | Strategy complete; **implementation is Wave 1** |
| Live authenticated multi-turn on stock Grok (Windows) | **No** â€” not performed; optional T6 later |

## 8. Unresolved assumptions

1. Exact Tauri stream transport (`tracer://events` channel vs event API) remains implementer choice under contract.  
2. Whether `tracer_session_create` stays combined create+start may split later without envelope breakage.  
3. Control plane owns assignment of `eventId` / `sequence` / `timestamp`.  
4. Fake ACP scenario selection mechanism (CLI vs env vs control message) is Wave 1 choice (catalog ids frozen).  
5. Auth method ids discovered from `initialize`, not hard-coded (examples only).  
6. Multi-turn status after a successful run (`completed` vs `ready`) remains control-plane policy.  
7. Authenticated live smoke requires credentials outside this workspace and is opt-in.  
8. W1 additive auth error classes preferred before Gate 1 freeze of adapter error catalog.

## 9. Platform limitations

| Platform | Limitation | Product implication |
|---|---|---|
| Windows | Prefer Job Object kill-on-close for session runtimes | F-W01 mandatory for process manager |
| Windows | No OS sandbox enforce equivalent to some Unix modes | Approvals still mandatory; never imply OS sandbox |
| Windows | Named-pipe leader path exists in Grok; MVP uses `--no-leader` | Tests must not require leader mode |
| Windows recon host | Authenticated live session creation not exercised | Do not claim live prompt parity |
| All | Standard CI has no network / paid APIs | Fake ACP only by default |

## 10. Deferred scope (not Wave 1 blockers if documented)

- Full IDE, multi-window editor chrome  
- Vendor-rich Grok panels driven by `x.ai/*` as core product  
- Live multi-OS CI matrix (platform tests on available OS first)  
- Desktop E2E (T7) as Gate 1 stretch after baseline  
- Production plugin marketplace / ALMS / multi-tenant  

## 11. Risk register

| ID | Risk | Severity | Mitigation |
|---|---|---|---|
| R1 | CI accidentally requires stock `grok` + credentials | High | `tests/specifications/ci/matrix.yaml` forbids live on standard CI |
| R2 | Implementers assert W0-B conceptual event names | Medium | Forbidden alias lists in expected-events; W0-A normative |
| R3 | Permission-cancel deadlock in real adapter | High | VS-05 / F-C04 mandatory Gate 1 |
| R4 | Windows orphans without Job Object | High | W1-C + T5; F-W01 |
| R5 | Auth UX depends on immature error classes | Medium | Bridge via `errorClassAnyOf` + structured messages |
| R6 | Synthetic stream treated as live parity | Medium | Labels + Gate 0/1 non-claims |
| R7 | W1-A expands into full IDE | Medium | IA/session scope fences; coordinator rejects IDE creep |
| R8 | Process ready confused with prompt ready | High | VS-02 / F-A05 / dual RuntimePill + StatusChip |
| R9 | Control plane / module integration lag | Medium | Recommended first launch group in readiness matrix |
| R10 | Live stock behavior drifts from fixtures | Low | Optional T6 re-probe; fixtures remain structural |

## 12. Final main candidate SHA

| Ref | SHA |
|---|---|
| Main at integration start | `5b936412b982cc4310f1196caef023a968ea070a` |
| W0-C merge | `eb9c7b14df3949b684c049e21444cbd102236e15` |
| W0-D merge | `1cdf3383febbe6e289f664b4d0d4c33bc961dbea` |
| Gate 0 artifacts commit | `58db0891cfc966c02e7a6f581fee2aa097e46513` |
| **Final `main` tip after FF** | `58db0891cfc966c02e7a6f581fee2aa097e46513` (pre close-table commit; see Â§16 for tip after SHA table refresh) |

## 13. Validation checklist

| # | Check | Result |
|---|---|---|
| 1 | History contains W0-A tip `aa7b778` | Pass (ancestor of HEAD) |
| 2 | History contains W0-B tip `a141d00` | Pass |
| 3 | History contains W0-C tip `23a66ea` | Pass |
| 4 | History contains W0-D tip `977ddee` | Pass |
| 5 | Merge order C then D preserved | Pass |
| 6 | Required W0-A deliverables exist | Pass |
| 7 | Required W0-B deliverables + fixtures exist | Pass |
| 8 | Required W0-C four UX docs + completion report | Pass |
| 9 | Required W0-D three testing docs + specifications tree | Pass |
| 10 | Completion reports for A/B/C/D present | Pass |
| 11 | Markdown links practical (relative docs paths) | Pass |
| 12 | No fixed machine paths in docs/fixtures (except â€œdo not useâ€ examples) | Pass â€” see scan commands |
| 13 | No secrets/credentials/private prompts/tokens | Pass â€” see scan commands |
| 14 | Synthetic fixtures labeled | Pass (`tests/fixtures/acp/README.md`, catalog, strategy) |
| 15 | `repos/grok-build` clean | Pass |
| 16 | No Wave 1 application source scaffolding | Pass (docs/specs only) |
| 17 | Heli path-claim conflicts | None material |
| 18 | Working tree clean after commits | Pass (at close) |
| 19 | No remote push | Observed / required |

### 13.1 Deterministic scan commands (PowerShell, from `repos/tracer`)

```powershell
# Absolute path patterns (expect only documentation "do not use" examples)
rg -n "D:\\\\|C:\\\\|/Users/|/home/[^/\s]+" docs tests

# Secret-like patterns (expect policy mentions only; no live credentials)
rg -n -i "(api[_-]?key|secret|password|token|bearer\s+[a-z0-9]|sk-[a-zA-Z0-9]{10,}|xai-[a-zA-Z0-9]{10,})" docs tests

# Private prompt / sensitive content wording
rg -n -i "(private.?prompt|my password|ssn|credit.?card)" docs tests

# Ancestry
git merge-base --is-ancestor aa7b778 HEAD
git merge-base --is-ancestor a141d00 HEAD
git merge-base --is-ancestor 23a66ea HEAD
git merge-base --is-ancestor 977ddee HEAD

# Grok-build cleanliness
git -C ../grok-build status -sb
```

**Scan interpretation (2026-07-17):** Absolute path hits only in docs explaining not to use fixed `D:\` / `/Users/` paths. Secret-like hits are policy, env var **names**, fixture usage metadata, or scrubbed examples â€” no committed credentials.

## 14. Files changed by this final integration task

**Merged from workers:**

- `docs/ux/**` (W0-C)
- `docs/testing/**` (W0-D)
- `tests/specifications/**` (W0-D)

**Added by W0-FINAL:**

- `docs/integration/FINAL_GATE_0_REPORT.md` (this file)
- `docs/integration/WAVE_1_READINESS_MATRIX.md`
- `docs/integration/REQUIREMENT_TRACEABILITY_MATRIX.md`

## 15. What was not done

- Did not start Wave 1 source implementation  
- Did not push remotes  
- Did not modify `repos/grok-build` or parent `resources/` plans  
- Did not re-run live Grok authenticated probes  
- Did not rewrite frozen W0-A contracts  
- Did not use headless/watchdog worker scripts  

## 16. Close table (filled at finalize)

| Field | Value |
|---|---|
| W0-C merge | `eb9c7b14df3949b684c049e21444cbd102236e15` |
| W0-D merge | `1cdf3383febbe6e289f664b4d0d4c33bc961dbea` |
| Gate 0 artifacts commit | `58db0891cfc966c02e7a6f581fee2aa097e46513` |
| Final main tip after FF (artifacts) | `58db0891cfc966c02e7a6f581fee2aa097e46513` |
| Close-table refresh commit | `cb09ca0270e5853dc303005403a29053836f45a1` |
| Ahead of `origin/main` | 21 commits at artifacts FF; +1 after close-table refresh |
| Lease released | Yes â€” session `heli-ses-7c2f27d1-a702-4c12-b07f-9962714c8c09` |
| Session closed | Yes |
| Pushed | **No** |

## 17. Recommended first Wave 1 launch group

See `docs/integration/WAVE_1_READINESS_MATRIX.md`. Summary:

**Parallel start (after Gate 0 on main):**

1. **W1-B** Domain and Event Protocol  
2. **W1-C** Runtime Process Manager  
3. **W1-E** Storage and Session Persistence  
4. **W1-G** Fake Runtime and Contract Harness  
5. **W1-A** Desktop Shell (placeholders only; mock stores)  
6. **W1-H** HeliHarness Workspace Integration (docs/templates)

**Then integrate:**

7. **W1-D** ACP Client and Runtime Adapter (depends on B + G fixtures; process API from C)  
8. **W1-F** Control Plane Integration (depends on Bâ€“E + D)

---

**Document control:** Final Gate 0 artifact. Update Â§16 SHAs only when main is fast-forwarded and lease released.
