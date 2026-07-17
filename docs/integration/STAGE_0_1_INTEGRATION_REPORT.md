# Stage 0.1 Integration Report — W0-A + W0-B

**Gate:** 0.1 (Architecture contracts + Grok runtime recon)  
**Task:** `tracer-w0-integration-ab`  
**Work item:** W0-I  
**Integrator host:** `grok-build`  
**Heli session:** `heli-ses-ae209d0d-566e-4296-bfce-60a193dc224b`  
**Lease:** `heli-lease-0becdd45-b7e7-400c-95fd-a9e04f0772dd`  
**Write target:** `tracer` (`repos/tracer` main checkout)  
**Integration branch:** `integration/tracer-w0-ab`  
**Date:** 2026-07-17

## 1. Gate 0.1 decision

| Field | Value |
|---|---|
| **Gate 0.1** | **PASS** |
| Stage 0.2 authorization | **Authorized** for W0-C Product UX and W0-D Test Strategy to proceed against integrated main (after this merge lands on `main`) |
| Material contradictions blocking merge | **None** |
| Reconciliation class | Documentation notes and normative-name authority only (no contract rewrite required) |

## 2. Source branches and tip SHAs

| Source | Branch | Tip SHA | Base |
|---|---|---|---|
| W0-A Architecture & Contracts | `agent/tracer-w0-architecture-contracts` | `aa7b778d7f1c571e225b5727dd2dd6bb80c2ebef` | `0301a7413f63558c3f943bd90fcc1c01d68fe152` |
| W0-B Grok Runtime Recon | `agent/tracer-w0-grok-runtime-recon` | `a141d00bc0c2a54ecb9a9c3045b2f04a91bfd524` | `0301a7413f63558c3f943bd90fcc1c01d68fe152` |
| Main at integration start | `main` | `0301a7413f63558c3f943bd90fcc1c01d68fe152` | — |

### Commits integrated (ordered)

**W0-A**

| SHA | Message |
|---|---|
| `7bf772559258de2aca54390dcca3949d316581bb` | `docs(w0-a): architecture contracts and ADRs for vertical slice` |
| `d76bcdb7164b7dd11cef66241d1f354c6e545d16` | `docs(w0-a): add W0-A completion report` |
| `aa7b778d7f1c571e225b5727dd2dd6bb80c2ebef` | `docs(w0-a): correct completion report commit SHAs` |

**W0-B**

| SHA | Message |
|---|---|
| `ff2b2dd56d583511ccd3b0169e77d9fd99027f4a` | `docs(w0-b): grok-build runtime recon, ACP mapping, fixtures` |
| `a141d00bc0c2a54ecb9a9c3045b2f04a91bfd524` | `docs(w0-b): completion report` |

### Integration merge commits

| SHA | Message |
|---|---|
| `05f29518a522e3707700b30b8ebecfb116fe2dce` | `merge(w0-a): architecture contracts into integration/tracer-w0-ab` |
| `da9ccacc81af605a2373028ea5d27b8eac6afa85` | `merge(w0-b): grok runtime recon into integration/tracer-w0-ab` |
| `c0b37a0ad04ccb6c7b5438d12cb4e43b2ecd7be9` | `docs(w0-i): stage 0.1 integration report and reconciliation notes` |

Final SHAs after merge-to-main are in §12. A follow-up commit may refresh §12 only.

## 3. Documents inspected

### Coordinator / harness

- `.heli-harness/HARNESS.md`
- `resources/TRACER_MASTER_BUILD_PLAN.md`
- `resources/TRACER_WAVE0_EXECUTION_AMENDMENT.md`
- workspace `AGENTS.md` (pointer)

### W0-A deliverables

- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`
- `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md`
- `docs/architecture/TRACER_VERTICAL_SLICE.md`
- `docs/decisions/ADR-001-runtime-sidecar.md`
- `docs/decisions/ADR-002-event-normalization.md`
- `docs/architecture/W0-A_COMPLETION_REPORT.md`

### W0-B deliverables

- `docs/research/grok-build/CAPABILITY_MATRIX.md`
- `docs/research/grok-build/PROCESS_LIFECYCLE.md`
- `docs/research/grok-build/ACP_EVENT_MAPPING.md`
- `docs/research/grok-build/FORK_RISK_REPORT.md`
- `docs/research/grok-build/W0-B_COMPLETION_REPORT.md`
- `tests/fixtures/acp/README.md`
- `tests/fixtures/acp/*.json` and `session-prompt-stream.jsonl`

### Diff scope vs main (pre-merge)

- **W0-A:** 7 files under `docs/{architecture,contracts,decisions}/` only (+1764 lines)
- **W0-B:** 12 files under `docs/research/grok-build/` and `tests/fixtures/acp/` only (+1450 lines)
- **Path overlap:** none (merge was mechanical conflict-free)

## 4. Semantic reconciliation findings

### 4.1 Runtime start and lifecycle — **aligned**

| Topic | W0-A | W0-B evidence | Decision |
|---|---|---|---|
| Sidecar model | Managed out-of-process stdio ACP (`ADR-001`) | Stock `grok agent stdio` process | Adopt stock sidecar; no fork |
| Preferred start | Placeholder installation descriptor (`runtimeKind: acp-stdio`, example args `["--acp"]`) | Exact: `grok agent --no-leader stdio` | **W0-B start path is authoritative for stock Grok.** W0-A example args are illustrative logical placeholders, not the stock CLI. Wave 1 configs for Grok must use `agent --no-leader stdio` (executable `grok` / PATH). |
| Framing | NDJSON **or** Content-Length TBD | NDJSON JSON-RPC 2.0 confirmed | Implement NDJSON for stock Grok + fake runtime |
| Readiness | `runtime.process.ready` after initialize + capability negotiation | No ready banner; process usable after successful `initialize` | **Aligned.** Tracer synthesizes ready; runtime does not emit a ready notification. |
| Session create | `createSession` after connect/ready | `session/new` after initialize (+ auth when required) | Aligned layering: process ready ≠ prompt-ready session |
| Prompt / stream | Adapter maps to Tracer events | `session/prompt` + `session/update` | Aligned |
| Cancel | Cooperative cancel if capability; else process stop | `session/cancel` notification supported | Aligned; stock Grok has cancellation |
| Shutdown / exit | Explicit `runtime.process.exited` / failed; no silent success | Close stdin → exit; kill tree / Job Object on Windows | Aligned |
| EOF / crash | `RuntimeDisconnected` / `RuntimeCrashed`; session failed/disconnected | Broken pipe / panic → client sees death; no durable crash protocol beyond resume | Aligned; Tracer process manager owns recovery |

Nothing in W0-A invents lifecycle steps beyond what W0-B evidence supports. Wire-level method names remain W0-B-owned.

### 4.2 Authentication boundary — **aligned with additive W1 notes**

| Topic | Finding | Decision |
|---|---|---|
| Process start vs auth | W0-B: `initialize` succeeds without credentials; `session/new` without `authenticate` returns live error `Authentication required` (`-32000`) | **Auth is not process startup.** Runtime process can be up and initialized while unauthenticated. |
| Session creation | Auth required for real `session/new` on stock Grok | Adapter must call `authenticate` (method id from `initialize.authMethods`) before `session/new` when required. |
| W0-A surface | `connect` = init + caps; `createSession` separate; no dedicated `authenticate` method named in the logical API | Not a contradiction: W0-A deferred wire auth to evidence. Wave 1 should treat authenticate as an adapter step between ready-after-init and session create (or fold into `createSession` preconditions). |
| Error classes | W0-A lacks a dedicated `AuthenticationRequired` / `AuthenticationFailed` class | **Additive recommendation for W1 (not a Gate 0.1 fail):** introduce `AuthenticationRequired` and `AuthenticationFailed` on adapter/Tauri surfaces when implementing auth. Map JSON-RPC auth errors accordingly. Until then, map to `PromptRejected` / `ProtocolViolation` / structured protocol error is insufficient — prefer new classes before Gate 1. |
| Live verification | Authenticated prompt stream **not** exercised live on Windows (no credentials in recon) | Do **not** claim full prompt/tool/permission live parity. Synthetic stream fixtures are structural only. |
| Auth as capability | Auth methods advertised on initialize; product setup state | Auth is runtime capability/setup state, not hidden UI-only behavior. UX (W0-C) must surface auth-required / login / API-key paths explicitly. |

### 4.3 ACP vs vendor extensions — **aligned**

| Topic | Finding | Decision |
|---|---|---|
| Separation | W0-B splits standard ACP vs `x.ai/*` | MVP depends on **standard ACP core** only |
| Unknown vendor | W0-A: `adapter.protocol.unknown` + optional adapter metadata | Preserve unknown vendor notifications as metadata; UI must not parse raw Grok frames (`ADR-002`) |
| Vendor surface size | Large unstable `x.ai/*` catalog | Optional later; do not block vertical slice |
| UI coupling | Both forbid raw ACP in React | Normative |

### 4.4 Event naming authority — **resolved (W0-A normative)**

W0-B `ACP_EVENT_MAPPING.md` uses **conceptual** Tracer names in mapping tables (examples: `runtime.initialized`, `message.agent.delta`, `permission.requested`, `turn.started`).

W0-A `TRACER_EVENT_PROTOCOL_V1.md` freezes the product catalog (examples: `runtime.process.ready`, `agent.message.delta`, `approval.requested`, `session.prompt.submitted`).

W0-B completion report already states: mapping tables use stable conceptual names; W0-A may refine Tracer event names.

| Decision | Detail |
|---|---|
| **Normative product event `type` strings** | **W0-A** `TRACER_EVENT_PROTOCOL_V1.md` catalog |
| **W0-B mapping tables** | Wire → **concept** guidance; implementers map concepts onto W0-A types |
| Example alignments | `initialize` success → capabilities stored + `runtime.process.ready` (not a separate required `runtime.initialized` product type unless later added); `agent_message_chunk` → `agent.message.delta`; `session/request_permission` → `approval.requested` / `approval.resolved`; prompt start/end → `session.prompt.submitted` + completion/stop via session/agent events (`session.completed` / status / stop metadata as designed in W0-A) |
| New types from W0-B concepts | Only via formal contract revision (e.g. auth lifecycle types if product needs them) |

A short normative-authority note was added to `ACP_EVENT_MAPPING.md` in the reconciliation commit.

### 4.5 Fixtures and evidence boundary — **aligned**

| Fixture | Provenance |
|---|---|
| `initialize-request.json` | Canonical recon request |
| `initialize-response.json` | **Live** capture, scrubbed |
| `session-new-auth-required.json` | **Live** unauthenticated error, scrubbed |
| `session-prompt-stream.jsonl` | **Synthetic** (not live model output) |
| `permission-request.json` | **Synthetic** |
| `cancel-notification.json` | **Synthetic** |

Policy in `tests/fixtures/acp/README.md`: no credentials, private prompts, or fixed machine absolute paths. Scan on integration tree found no machine-absolute paths or secret-like tokens. Placeholders such as `{{PROJECT_ROOT}}` are used.

**Synthetic must not be presented as live-captured parity evidence** for authenticated multi-turn tool/permission flows.

### 4.6 Capability negotiation, cancel, errors, platforms — **aligned**

- W0-A capability keys are Tracer-view booleans; unknown runtime keys stay in adapter metadata — matches W0-B vendor `_meta` treatment.
- Missing cancellation → process stop fallback is consistent with W0-B (though stock Grok **does** support cancel).
- Error classes cover protocol, process, capability, permission, storage; authentication classes recommended additively (§4.2).
- Windows-specific limits (no OS sandbox enforce, named pipes for leader, Job Object kill-on-close, best-effort source builds) are documented **as Windows**, not generalized to all platforms.

### 4.7 Fork recommendation — **aligned**

Both streams: **do not fork** Grok Build for the vertical slice. Stock sidecar + adapter; revisit after adoption gate (`FORK_RISK_REPORT.md`, `ADR-001`).

## 5. Decisions made during integration

1. Merge order: **W0-A first**, then **W0-B**, non-fast-forward merges on `integration/tracer-w0-ab`, then ff-only into `main`.
2. No mechanical conflicts; no contract text rewrites required for Gate 0.1.
3. **W0-A owns normative Tracer event/command/adapter semantics.**
4. **W0-B owns stock Grok wire evidence, start command, fixtures, and vendor inventory.**
5. Stock spawn: `grok agent --no-leader stdio` (NDJSON stdio).
6. Readiness = process alive + successful initialize (+ recorded capabilities) → emit `runtime.process.ready`.
7. Auth is a separate gate before usable `session/new` on stock Grok; not verified end-to-end live for prompt streams.
8. No new ADR required beyond existing ADR-001 / ADR-002.
9. Reconciliation edit: clarify normative event-name authority on the W0-B mapping doc only.

## 6. Unresolved assumptions

1. Authenticated live multi-turn stream (tools + permissions + cancel under load) remains **unverified** on this workspace; Wave 1 optional smoke needs credentials or mock inference.
2. Exact Tauri streaming transport (Tauri events vs channel API) remains an implementation choice under `tracer://events`.
3. Whether `tracer_session_create` remains a combined create+start command may split later without breaking envelopes.
4. Control plane assigns `eventId` / `sequence` / `timestamp` (W0-A Wave 1 decision).
5. Fake ACP runtime will implement enough of the standard surface for CI; stock Grok remains optional smoke.
6. Auth method ids are discovered from `initialize`, not hard-coded (except examples).
7. `AuthenticationRequired` / `AuthenticationFailed` error classes should be added when W1 implements auth (contract minor bump or implementation note accepted by Gate 1 reviewers).

## 7. Risks passed to W0-C (Product UX)

| Risk | Severity | Guidance for W0-C |
|---|---|---|
| Auth-required state before prompts | High | Model explicit UI for unauthenticated runtime, auth method choice, and failure — not a generic “session error” only |
| Session statuses vs UX labels | Medium | Bind screens to W0-A status set: `creating`, `starting_runtime`, `ready`, `running`, `awaiting_approval`, `cancelling`, `completed`, `failed`, `disconnected`, `stopped` |
| Approval interrupt | Medium | Map to `approval.requested` / `approval.resolved`; fail closed; do not invent parallel permission event names |
| Vendor-rich Grok UI features | Medium | Out of MVP unless optional panels; do not require `x.ai/*` for core flows |
| Crash / disconnect honesty | Medium | Never show “running” after `runtime.process.exited` |
| Synthetic fixtures | Low | UX copy/examples must not claim live Grok transcripts for synthetic streams |

## 8. Risks passed to W0-D (Test Strategy)

| Risk | Severity | Guidance for W0-D |
|---|---|---|
| CI must not need paid APIs | High | Fake ACP + sanitized fixtures only in default CI |
| Auth gate in stock runtime | High | Contract tests for auth-required error fixture; separate optional live suite |
| Synthetic vs live labeling | Medium | Acceptance criteria must distinguish structural fixture tests from live smoke |
| Process orphan / Windows Job Object | Medium | Explicit crash and stop scenarios on Windows |
| Cancel + permission deadlock | Medium | Ignoring `session/request_permission` deadlocks the turn — test fail-closed and cancel-while-pending |
| Event catalog vs mapping doc names | Medium | Assert **W0-A type strings** in contract tests; treat W0-B names as conceptual |
| Framing | Low | NDJSON framing tests against fixtures + fake runtime |

## 9. Checks run

| Check | Result |
|---|---|
| Heli claim `tracer-w0-integration-ab` write | Pass — session `heli-ses-ae209d0d-566e-4296-bfce-60a193dc224b` |
| `target set tracer` | Pass — writes under `repos/tracer` |
| Session status: task, lease, worktree = main checkout | Pass |
| Conflicts / path-claim overlaps | None material for integration |
| `main` clean before branch | Pass |
| `repos/grok-build` clean (no edits) | Pass |
| W0-A / W0-B tips verified | `aa7b778`, `a141d00` |
| Full path lists vs main; no file overlap | Pass |
| Semantic reconciliation checklist (§4) | Pass with notes |
| `git merge --no-ff` W0-A then W0-B | Pass, zero conflicts |
| Machine-absolute path / secret-like scan on docs+fixtures | Pass (no hits) |
| Fixture synthetic/live provenance labels | Present |
| Files only under approved docs/fixture paths | Pass |
| No push to remotes | Observed |

## 10. Files changed by this integration task

**Merged from workers (already on integration branch after merges):**

- All W0-A and W0-B paths listed in §3

**Added/edited by W0-I:**

- `docs/integration/STAGE_0_1_INTEGRATION_REPORT.md` (this file)
- `docs/research/grok-build/ACP_EVENT_MAPPING.md` (normative-name authority note only)

## 11. ADRs

No new ADR. Existing:

- `ADR-001-runtime-sidecar` — confirmed by W0-B stock process evidence
- `ADR-002-event-normalization` — confirmed by vendor surface size and mapping complexity

## 12. Final SHAs (filled at close)

| Ref | SHA |
|---|---|
| W0-A merge | `05f29518a522e3707700b30b8ebecfb116fe2dce` |
| W0-B merge | `da9ccacc81af605a2373028ea5d27b8eac6afa85` |
| Integration report + reconciliation (initial) | `c0b37a0ad04ccb6c7b5438d12cb4e43b2ecd7be9` |
| Final `main` (includes SHA table refresh) | recorded in commit message / completion report as HEAD at release |
| Gate 0.1 | **PASS** |

## 13. What was not done

- Did not claim or modify W0-C / W0-D
- Did not push remotes
- Did not modify `repos/grok-build` or parent `resources/`
- Did not implement application source
- Did not re-run live Grok probes

## 14. Stage 0.2 authorization statement

Because **Gate 0.1 is PASS**, Stage 0.2 work that depends on integrated architecture contracts and runtime recon evidence is **authorized** once this integration lands on `main`:

- W0-C Product UX may bind UX to integrated contracts + recon findings
- W0-D Test Strategy may author acceptance tests against integrated contracts + fixtures

Full Gate 0 (all four workers + human approval) remains a later coordinator step after W0-C and W0-D integrate.
