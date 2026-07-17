# Vertical Slice Acceptance Criteria (Gate 1)

**Status:** Gate 0 acceptance freeze candidate  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-test-strategy` (W0-D)  
**Product goal:** Open a local repository → start an ACP-compatible agent runtime → create a session → submit a prompt → stream normalized agent/tool events → show changed files and runtime state → persist the session → stop or resume safely.

Normative references:

- `docs/architecture/TRACER_VERTICAL_SLICE.md`
- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`
- `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md`
- `docs/testing/TEST_STRATEGY.md`
- `docs/testing/FAILURE_MATRIX.md`
- `tests/specifications/scenarios/catalog.yaml`

## 1. How to read this document

Each acceptance case has:

| Field | Meaning |
|---|---|
| **ID** | Stable scenario id (matches specifications catalog where applicable) |
| **Tier** | Test tier from `TEST_STRATEGY.md` |
| **Evidence label** | `fake-runtime` / `synthetic` / `live-scrubbed` / `live-authenticated` |
| **Standard CI** | Whether pass is required on every PR |
| **Preconditions** | Setup |
| **Steps** | Ordered actions |
| **Must observe** | Required events, statuses, command results (W0-A names) |
| **Must not observe** | Forbidden outcomes |
| **Pass** | Binary criterion |

**Rule:** A case labeled `synthetic` or `fake-runtime` never counts as proof of live stock Grok multi-turn parity. A case labeled `live-authenticated` may consume provider usage and is **not** standard CI.

## 2. Gate 1 global checklist

Gate 1 **PASS** requires all of the following:

| # | Criterion | Primary evidence |
|---|---|---|
| G1-01 | Clean checkout setup is documented and followed | Docs + human/CI |
| G1-02 | Application starts from documented instructions | Manual or T7 |
| G1-03 | Fake runtime end-to-end flow passes **without network** | T2 VS-01 |
| G1-04 | UI does not parse raw ACP / vendor frames | T4 + code review |
| G1-05 | Normalized events persisted with monotonic `sequence` | T2/T3 VS-01, VS-10 |
| G1-06 | Session history reloads after app restart | T3 VS-10 |
| G1-07 | Runtime crash yields visible `failed`/`disconnected` (not `running`) | T2 VS-06 |
| G1-08 | Stop leaves **no orphaned** runtime process | T2/T5 VS-09 |
| G1-09 | Cancel works or falls back to process stop | T2 VS-04, VS-05 |
| G1-10 | Auth-gate behavior covered (fixture/fake) | T1/T2 VS-02 |
| G1-11 | Cancel-while-permission-pending does not deadlock | T2 VS-05 |
| G1-12 | Standard CI green (no paid APIs) | CI |
| G1-13 | Platform limitations explicitly documented | Docs |
| G1-14 | Stock runtime smoke **documented** (execution optional in CI) | Runbook ± T6 |

Optional stretch (not Gate 1 blockers):

| # | Criterion | Notes |
|---|---|---|
| G1-S1 | Live authenticated prompt stream on stock Grok | T6; may cost usage |
| G1-S2 | Multi-OS process matrix green | When runners exist |

## 3. Acceptance cases

### VS-01 — Happy path with fake ACP (Gate 1 core)

| Field | Value |
|---|---|
| **ID** | `VS-01` / scenario `happy_prompt_stream` |
| **Tier** | T2 (+ T3 persist, T4 optional UI) |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Preconditions**

- Fake ACP on PATH or configured `runtimeKind: acp-stdio` executable.
- Scenario `happy_prompt_stream`.
- Temp project directory (no machine-specific paths committed).

**Steps**

1. `tracer_project_register` with temp project root.
2. `tracer_session_create` for that project.
3. Wait until session status is `ready` (via events and/or `tracer_session_get`).
4. `tracer_session_submit_prompt` with sanitized text.
5. Wait until run completes (`session.completed` or status returns to `ready` after successful run).
6. `tracer_events_list` for the session.
7. `tracer_session_stop`.

**Must observe**

- Events including (order illustrative, sequences monotonic):
  - `session.created`
  - `runtime.process.started`
  - `runtime.process.ready` with `payload.capabilities` object
  - `session.ready` (and/or `session.status.changed` to `ready`)
  - `session.prompt.submitted`
  - one or more `agent.message.delta` **or** `agent.message.completed` if streaming synthesized
  - tool events if scenario emits tools: `tool.started` / `tool.updated` / `tool.completed`
  - terminal success: `session.completed` and/or status not failed
- Command submit returns `accepted: true` with `promptId` / `agentRunId`.
- Listed events: `eventVersion: 1`, required envelope fields, ascending `sequence`.

**Must not observe**

- Raw ACP method names as the only UI timeline types.
- `session.completed` without any agent/tool evidence when scenario promised a stream.
- Network calls / API keys required.

**Pass:** All must-observe; stop succeeds; process not running afterward.

---

### VS-02 — Auth required before session (stock shape)

| Field | Value |
|---|---|
| **ID** | `VS-02` / `auth_required_session_new` |
| **Tier** | T1 (fixture) + T2 (fake scenario) |
| **Evidence** | `live-scrubbed` (fixture shape) + `fake-runtime` (integration) |
| **Standard CI** | **Yes** |

**Preconditions**

- Fixture `tests/fixtures/acp/session-new-auth-required.json` available.
- Fake scenario reproduces auth-required on `session/new` without authenticate.

**Steps**

1. Start runtime path through initialize only (process ready).
2. Attempt session create without completing auth (as scenario defines).
3. Observe error path.

**Must observe**

- Process may reach `runtime.process.ready` (initialize succeeded).
- Session is **not** prompt-ready.
- Structured failure: wire error code `-32000` / message containing Authentication required at ACP layer **or** product `errorClass` in `{ AuthenticationRequired, PromptRejected, ProtocolViolation, InvalidState }` until dedicated auth classes land.
- User-visible / command-level non-success.

**Must not observe**

- Silent `session.ready`.
- Successful `tracer_session_submit_prompt`.
- Treating process start alone as authenticated.

**Pass:** Auth gate blocks prompts; process-ready ≠ session-ready proven.

**Evidence note:** Fixture is live-scrubbed; scenario does **not** prove interactive login or API-key success (that is VS-L1).

---

### VS-03 — Approval allow / deny (fail closed)

| Field | Value |
|---|---|
| **ID** | `VS-03` / `permission_allow`, `permission_deny` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` (wire shape may also use synthetic fixture `permission-request.json`) |
| **Standard CI** | **Yes** |

**Steps (allow)**

1. Happy setup to `ready`.
2. Prompt scenario that emits permission reverse-request.
3. Observe `approval.requested`; status `awaiting_approval`.
4. `tracer_approval_resolve` with `allow`.
5. Observe `approval.resolved` and tool completion / session success path.

**Steps (deny)**

1. Same through `approval.requested`.
2. Resolve `deny`.
3. Tool ends failed/cancelled; no claimed success for denied action.

**Must not observe**

- Auto-approve of unknown risk/kind.
- Hang forever without status change.
- UI requiring raw `session/request_permission` parsing.

**Pass:** Allow and deny paths both terminal and honest.

---

### VS-04 — Cancel mid-stream

| Field | Value |
|---|---|
| **ID** | `VS-04` / `cancel_mid_stream` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Steps**

1. Start streaming prompt.
2. `tracer_session_cancel` while deltas/tools active.
3. Wait ≤ `T_cancel` (+ process fallback if needed).

**Must observe**

- `session.status.changed` to `cancelling` (if not already terminal).
- `session.cancelled` and/or terminal status `stopped` / cancelled semantics.
- Cooperative mode when `capabilities.cancellation` true; else process stop.

**Must not observe**

- Status stuck in `running` after timeout budget.
- Orphan process.

**Pass:** Terminal cancel within policy timeouts.

---

### VS-05 — Cancel while permission pending (deadlock avoidance)

| Field | Value |
|---|---|
| **ID** | `VS-05` / `cancel_while_permission_pending` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Why this exists**

Ignoring an open `session/request_permission` while cancelling can deadlock the runtime turn. Stage 0.1 risks list this explicitly.

**Steps**

1. Drive scenario that opens permission reverse-request and parks.
2. Do **not** resolve approval; invoke `tracer_session_cancel` (or stop).
3. Bound wait: `T_cancel + T_term` (implementation defaults from adapter contract guidance, e.g. 5–15s cancel + force kill budget).

**Must observe**

- Session leaves `awaiting_approval` / `running` for a terminal state (`cancelling` → `stopped`/`cancelled`/`failed`/`disconnected` as designed).
- Either: runtime receives cancel and releases the park, **or** process is stopped.
- `approval.resolved` with `decision: cancel` **or** equivalent system cancellation recording is acceptable.
- Process not alive after stop path.

**Must not observe**

- Deadlock past timeout budget.
- Permanent `awaiting_approval` with live process ignoring cancel.
- Silent success.

**Pass:** Completes within bound; no orphan; user-visible terminal state.

---

### VS-06 — Unexpected runtime crash mid-prompt

| Field | Value |
|---|---|
| **ID** | `VS-06` / `crash_nonzero_exit` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Steps**

1. Start prompt.
2. Fake exits non-zero (or kills itself) mid-run.

**Must observe**

- `runtime.process.exited` with `expected: false` and/or `runtime.process.failed`.
- Session status `failed` or `disconnected`.
- Active tools fail or cancel; no new prompt accepted on dead handle (`RuntimeDisconnected`).

**Must not observe**

- Status remaining `running`.
- `session.completed` as success.

**Pass:** Honest failure; recoverable only via new runtime start.

---

### VS-07 — EOF / disconnect mid-prompt

| Field | Value |
|---|---|
| **ID** | `VS-07` / `eof_mid_prompt` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Must observe**

- `RuntimeDisconnected` and/or exit/fail events.
- No silent completion.

**Pass:** Same honesty bar as VS-06.

---

### VS-08 — Malformed frame and unknown vendor notification

| Field | Value |
|---|---|
| **ID** | `VS-08` / `malformed_frame`, `unknown_vendor_notification` |
| **Tier** | T1 + T2 |
| **Evidence** | `fake-runtime` / `synthetic` |
| **Standard CI** | **Yes** |

**Must observe**

- Malformed: `adapter.protocol.error` (and/or `ProtocolParseError`); session continues if process alive **or** fails transport cleanly if unrecoverable — never UI crash.
- Unknown vendor: `adapter.protocol.unknown`; continue.

**Must not observe**

- Process kill as the only reaction to a single unknown notification.
- UI exception from unknown `type`.

**Pass:** Resilience criteria met for both subcases.

---

### VS-09 — Stop without orphans (incl. Windows)

| Field | Value |
|---|---|
| **ID** | `VS-09` / `clean_shutdown_stdin_close` + force-kill variant |
| **Tier** | T2 + **T5** |
| **Evidence** | `fake-runtime` (+ OS process inspection) |
| **Standard CI** | T2 yes; T5 on available OS runners |

**Steps**

1. Running or ready session with live child.
2. `tracer_session_stop` (graceful).
3. Assert child PID gone.
4. Repeat path with cancel timeout / force kill scenario (`slow_cancel_ack`).

**Windows-specific musts**

- Force path uses Job Object or equivalent kill-tree.
- Document if grandchildren edge cases remain.

**Must not observe**

- Leftover fake/stock agent processes from the session.
- Zombie tool shells owned by the runtime job (best-effort where OS allows).

**Pass:** No orphans for managed session process tree on tested OS.

---

### VS-10 — Persistence and reload

| Field | Value |
|---|---|
| **ID** | `VS-10` |
| **Tier** | T3 |
| **Evidence** | `fake-runtime` + storage |
| **Standard CI** | **Yes** |

**Steps**

1. Complete VS-01 (or subset) so ≥ N events stored.
2. Shut down control plane / app.
3. Restart; `tracer_session_get` + `tracer_events_list`.

**Must observe**

- Same `sessionId`; events identical by `sequence` / `eventId`.
- Status is durable truth (not “running” without process).
- Unknown types preserved if present.

**Pass:** Replay matches storage order.

---

### VS-11 — Unsupported cancellation capability

| Field | Value |
|---|---|
| **ID** | `VS-11` / `cancel_unsupported` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Must observe**

- Negotiated capabilities show cancellation false (Tracer view).
- Cooperative cancel returns `CapabilityUnsupported` **or** control plane immediately selects process stop.
- User cancel/stop still ends work; `runtime.process.exited` as needed.
- No orphans.

**Pass:** Fallback works; product does not pretend cooperative cancel succeeded.

---

### VS-12 — Capability minimal surface

| Field | Value |
|---|---|
| **ID** | `VS-12` / `capability_minimal` |
| **Tier** | T2 |
| **Evidence** | `fake-runtime` |
| **Standard CI** | **Yes** |

**Must observe**

- Missing `planUpdates` → no synthetic `agent.plan.updated`.
- Missing streaming → still get `agent.message.completed`.
- Ready still emitted if hard requirements met.

**Pass:** Product degrades; does not hard-crash.

---

### VS-13 — UI consumes normalized events only

| Field | Value |
|---|---|
| **ID** | `VS-13` |
| **Tier** | T4 (+ review) |
| **Evidence** | `unit-generated` / recorded normalized packs |
| **Standard CI** | **Yes** (unit/component) |

**Must observe**

- Timeline fixtures use W0-A `type` strings only.
- Approval UX bound to `approval.requested` / `approval.resolved`.
- Disconnect/crash UX bound to process exit / session failed|disconnected.

**Must not observe**

- Imports of ACP JSON-RPC parsers in UI feature modules.
- Dependence on `x.ai/*` for core path.

**Pass:** Component tests green; architecture review clean.

---

### VS-14 — Project register validation

| Field | Value |
|---|---|
| **ID** | `VS-14` |
| **Tier** | T2/T0 |
| **Evidence** | unit / integration |
| **Standard CI** | **Yes** |

**Must observe**

- Missing path → `NotFound` / `InvalidArgument`.
- Valid directory registers; list returns project.
- Committed tests use temp dirs, not fixed `D:\` / `/Users/...` paths.

**Pass:** Validation + list round-trip.

---

## 4. Live optional cases (not standard CI)

### VS-L1 — Stock Grok initialize + auth + one prompt

| Field | Value |
|---|---|
| **ID** | `VS-L1` |
| **Tier** | T6 |
| **Evidence** | `live-authenticated` |
| **Standard CI** | **No** |
| **May consume provider usage** | **Yes** |

**Steps (manual or gated job)**

1. Spawn `grok agent --no-leader stdio` with hermetic `GROK_HOME` if possible.
2. `initialize` (compare shape to live-scrubbed fixture; tolerate additive fields).
3. `authenticate` with allowed secret source.
4. `session/new` with project cwd.
5. One short `session/prompt`; observe stream.
6. Cancel or complete; shutdown; check orphans.

**Pass:** Documented result with provenance `live-authenticated`. Failures due to missing credentials are **skips**, not Gate 1 failures.

**Must not:** Commit secrets, private prompts, or machine-absolute paths from captures.

### VS-L2 — Stock auth-required re-probe

| Field | Value |
|---|---|
| **ID** | `VS-L2` |
| **Tier** | T6 without credentials |
| **Evidence** | `live-scrubbed` refresh |
| **Standard CI** | **No** (optional nightly if `grok` installed) |

Confirms unauthenticated `session/new` still returns Authentication required. Does not consume model tokens if stopped before prompt.

## 5. Mapping: acceptance → specification artifacts

| Acceptance | Specification scenario id | Expected events pack |
|---|---|---|
| VS-01 | `happy_prompt_stream` | `tests/specifications/expected-events/happy_prompt_stream.json` |
| VS-02 | `auth_required_session_new` | `auth_required_session_new.json` |
| VS-03 | `permission_allow` / `permission_deny` | matching JSON packs |
| VS-04 | `cancel_mid_stream` | `cancel_mid_stream.json` |
| VS-05 | `cancel_while_permission_pending` | `cancel_while_permission_pending.json` |
| VS-06 | `crash_nonzero_exit` | `crash_nonzero_exit.json` |
| VS-07 | `eof_mid_prompt` | `eof_mid_prompt.json` |
| VS-08 | `malformed_frame`, `unknown_vendor_notification` | matching packs |
| VS-11 | `cancel_unsupported` | `cancel_unsupported.json` |

Expected-event packs list **W0-A** `type` strings and ordering constraints; they are not full live transcripts.

## 6. Non-goals for Gate 1 acceptance

- Full IDE behavior
- Vendor `x.ai/*` feature parity
- Leader mode / multi-window shared agent
- ALMS orchestration
- Cloud multi-tenant
- Guaranteed live model quality

## 7. Sign-off template (for Gate 1 report)

```text
Gate 1 acceptance run
Date:
Commit SHA:
OS:
Runtime under test: fake | stock (version)
CI standard: green | red
VS-01 … VS-14: pass/fail/skip
VS-L1/L2: pass/fail/skip/not-run
Orphan check OS:
Known limitations:
Evidence labels used:
Approver:
```

---

**Document control:** W0-D deliverable. Implementation fills automation; criteria stay stable unless contract-change process revises them.
