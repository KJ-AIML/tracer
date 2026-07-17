# Tracer Test Strategy (Wave 0 / Gate 1)

**Status:** Gate 0 test-architecture freeze candidate  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-test-strategy` (W0-D)  
**Applies to:** vertical-slice implementation (Wave 1) and Gate 1 evidence  
**Normative product names:** W0-A contracts (`docs/contracts/*`)  
**Wire evidence:** W0-B recon (`docs/research/grok-build/*`, `tests/fixtures/acp/*`)

## 1. Purpose

Define how Tracer proves the vertical slice is correct, reliable, and CI-safe without requiring paid APIs or network access for standard continuous integration.

This document freezes:

1. test **tiers** and what each may depend on;
2. the **deterministic fake ACP runtime** as the default CI path;
3. how **synthetic fixtures** differ from **live** evidence;
4. process-startup vs **authenticated session creation**;
5. **contract** tests vs **vendor-extension** compatibility tests;
6. crash, EOF, cancel, orphan, recovery, and unsupported-capability coverage;
7. the **minimal CI matrix**;
8. evidence required for **Gate 1**.

This document does **not** implement full test suites. Wave 1 agents implement tests against these specifications and `tests/specifications/`.

### Related documents

| Document | Role |
|---|---|
| `docs/testing/VERTICAL_SLICE_ACCEPTANCE.md` | Gate 1 acceptance scenarios and pass criteria |
| `docs/testing/FAILURE_MATRIX.md` | Failure class → expected outcome → owning test tier |
| `tests/specifications/` | Machine-readable scenario catalogs and expected event sequences |
| `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md` | Normative event `type` strings and envelope |
| `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md` | Adapter lifecycle, capabilities, error classes |
| `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` | Command surface and stream channel |
| `docs/architecture/TRACER_VERTICAL_SLICE.md` | Slice scope and Gate 1 exit |
| `docs/integration/STAGE_0_1_INTEGRATION_REPORT.md` | Stage 0.1 recon/contract reconciliation |
| `tests/fixtures/acp/` | Sanitized ACP wire fixtures (W0-B) |

## 2. Design principles

1. **Determinism first.** Default CI proves product logic with a fake ACP runtime and recorded fixtures. No network, no credentials, no model spend.
2. **Honest evidence labels.** Synthetic fixtures prove structural mapping only. Live smoke proves stock runtime behavior. Never present synthetic streams as live multi-turn parity.
3. **Normative names only.** Contract and acceptance tests assert **W0-A** event `type` strings (for example `runtime.process.ready`, `agent.message.delta`, `approval.requested`). W0-B mapping names are conceptual guidance only.
4. **Fail closed.** Unknown approvals, unknown destructive actions, and missing required readiness states must not look successful.
5. **Process honesty.** Process startup ≠ authenticated session ≠ prompt-ready session. Tests must distinguish each gate.
6. **No orphans.** Stop, cancel, crash, and app-exit paths leave no unmanaged runtime or shell children (platform rules apply; Windows Job Object coverage is mandatory).
7. **UI never parses ACP.** Frontend tests consume normalized Tracer events / Tauri command results only.
8. **Single DB writer.** Storage tests enforce control-plane-only writes.

## 3. Test tier taxonomy

| Tier | Code | Purpose | Network | Paid API / credentials | Default CI |
|---|---|---|---|---|---|
| **T0 Unit** | `unit` | Pure functions: envelope validation, status transitions, redaction, error mapping | No | No | Yes |
| **T1 Contract** | `contract` | Schema/fixtures: event envelopes, command error shapes, ACP fixture parse + normalize mapping | No | No | Yes |
| **T2 Fake runtime integration** | `fake` | Adapter + process + control plane against **deterministic fake ACP** | No | No | Yes |
| **T3 Storage / recovery** | `storage` | SQLite migrations, ordered replay, interrupt recovery | No | No | Yes |
| **T4 UI contract** | `ui` | React components against mock stores / recorded normalized events | No | No | Yes |
| **T5 Process platform** | `platform` | Spawn/kill/orphan matrix; may be OS-selective | No | No | Yes (host OS) or scheduled multi-OS |
| **T6 Live authenticated smoke** | `live` | Stock runtime with real auth + optional model call | Often yes | **May consume provider usage** | **No** (opt-in job only) |
| **T7 Desktop E2E** | `e2e` | Full app shell after slice stable | Prefer no (fake) | No in default | After Gate 1 baseline; not required for first CI land |

### 3.1 What each tier is allowed to claim

| Claim | Allowed evidence |
|---|---|
| “Adapter maps `agent_message_chunk` → `agent.message.delta`” | T1 fixture + T2 fake |
| “Stock Grok initialize shape matches fixture” | T1 against live-scrubbed fixture; optional T6 re-probe |
| “Authenticated multi-turn tools work on stock Grok” | **T6 only** (never T1 synthetic stream) |
| “No orphan after force stop on Windows” | T5 on Windows runner |
| “Gate 1 vertical slice works offline” | T2 + T3 + T4 (and documented T5 on at least one OS) |

### 3.2 Evidence provenance labels (mandatory)

Every fixture, scenario, and acceptance result must carry one of:

| Label | Meaning |
|---|---|
| `synthetic` | Constructed from contracts/source; not a live capture |
| `live-scrubbed` | Captured from a real runtime, credentials/paths scrubbed |
| `live-authenticated` | Captured or executed with real auth (opt-in; may use provider) |
| `fake-runtime` | Produced by Tracer’s deterministic fake ACP process |
| `unit-generated` | Generated by pure unit test helpers |

W0-B fixture provenance (authoritative until replaced):

| Fixture | Label |
|---|---|
| `tests/fixtures/acp/initialize-request.json` | synthetic / canonical request |
| `tests/fixtures/acp/initialize-response.json` | **live-scrubbed** |
| `tests/fixtures/acp/session-new-auth-required.json` | **live-scrubbed** |
| `tests/fixtures/acp/session-prompt-stream.jsonl` | **synthetic** |
| `tests/fixtures/acp/permission-request.json` | **synthetic** |
| `tests/fixtures/acp/cancel-notification.json` | **synthetic** |

## 4. Critical distinctions (must appear in test design)

### 4.1 Process startup vs authenticated session creation

These are **three separate gates**:

```text
Gate P — Process alive
  spawn child; pipes open; pid known
  event: runtime.process.started

Gate I — Protocol initialized / process ready
  successful initialize + capability negotiation recorded
  event: runtime.process.ready
  NOT the same as “user can prompt”

Gate A — Authenticated + session created
  authenticate (when required) → session/new success
  events: session.ready (and status ready)
  ONLY then submit prompt may succeed
```

**Test implications:**

| Scenario | Expected |
|---|---|
| Spawn + initialize without authenticate | `runtime.process.ready` allowed; session create may fail with auth error class |
| `session/new` without auth against stock-like behavior | Error mapped from fixture `session-new-auth-required.json` (live-scrubbed shape) |
| Prompt before `runtime.process.ready` | `RuntimeNotReady` / command `InvalidState` |
| Prompt after ready but before session ready | `InvalidState` / session not ready |
| Fake runtime can skip real credentials | Fake implements auth as no-op or scripted success for CI |

Control plane must never claim `session.ready` solely because the OS process started.

### 4.2 Deterministic fake runtime vs synthetic ACP fixtures vs live smoke

| Mechanism | Role | CI |
|---|---|---|
| **Fake ACP runtime** | Child (or in-process double) that speaks NDJSON JSON-RPC, drives scripted scenarios | Default |
| **Synthetic ACP fixtures** | Static files for parser/normalizer unit/contract tests | Default |
| **Live stock smoke** | Real `grok agent --no-leader stdio` (or configured stock binary) | Opt-in only |

Rules:

- Fake runtime is the **system under test partner** for integration acceptance.
- Synthetic fixtures are **not** substitutes for live authenticated multi-turn evidence.
- Live smoke may consume provider usage; isolate behind env flags (for example `TRACER_LIVE_SMOKE=1`) and never enable in standard CI.

### 4.3 Contract tests vs vendor-extension compatibility tests

| Kind | Asserts | Must not |
|---|---|---|
| **Contract (standard ACP + Tracer)** | Envelope fields; W0-A `type` catalog; adapter capability keys; Tauri command names/error classes; standard methods (`initialize`, `session/*`, `session/request_permission`) | Depend on `x.ai/*` methods |
| **Vendor-extension compatibility** | Unknown vendor notifications → `adapter.protocol.unknown`; optional mapping of selected `x.ai/*` when implemented | Block MVP/Gate 1 if vendor surface changes |

MVP Gate 1 **requires** contract + fake path. Vendor tests are **non-blocking** unless product ships a vendor-dependent feature.

### 4.4 Tests that may consume provider usage

Only **T6 live authenticated smoke** (and any manual exploratory runs) may call models or authenticated stock agent endpoints.

All T0–T5 tests MUST:

- set hermetic homes (fake `GROK_HOME`-like dirs only if needed);
- refuse to read live API keys for success paths;
- use fake runtime or fixtures exclusively.

## 5. Fake ACP runtime requirements

Wave 1 implements a **deterministic fake ACP-compatible process** (preferred: real OS child over stdio, same as production path).

### 5.1 Transport

- JSON-RPC 2.0 **NDJSON** on stdin/stdout (aligned with Stage 0.1: stock Grok uses NDJSON).
- Stderr is logs only (may emit scripted stderr lines for process tests).
- No ready banner; readiness = successful `initialize` response (Tracer synthesizes `runtime.process.ready`).

### 5.2 Scenario driver

Fake selects behavior via one of (implementation choice; document in Wave 1):

1. CLI arg: `--scenario <id>` matching `tests/specifications/scenarios/*.yaml` ids;
2. env: `TRACER_FAKE_ACP_SCENARIO=<id>`;
3. first control message / fixture pack path.

Scenarios must be **deterministic**: fixed chunking, fixed tool ids, fixed ordering, no sleeps beyond optional injectable delays for timeout tests.

### 5.3 Minimum scenario catalog (IDs)

See `tests/specifications/scenarios/catalog.yaml`. Required IDs:

| ID | Intent |
|---|---|
| `happy_prompt_stream` | init → (fake auth) → session → prompt → deltas → tools → complete |
| `auth_required_session_new` | init ok; session/new returns Authentication required shape |
| `permission_allow` | tool + `session/request_permission` → allow → complete |
| `permission_deny` | permission deny → tool failed; no silent success |
| `cancel_mid_stream` | cancel during message/tool stream |
| `cancel_while_permission_pending` | cancel while permission reverse-request open; no deadlock |
| `malformed_frame` | emit bad JSON line mid-stream |
| `unknown_vendor_notification` | emit unmapped method/notification |
| `eof_mid_prompt` | close stdout / exit mid-prompt |
| `crash_nonzero_exit` | exit non-zero while “running” |
| `cancel_unsupported` | advertise `cancellation: false` (Tracer capability view) |
| `slow_cancel_ack` | delay cancel handling past timeout (force kill path) |
| `duplicate_response_id` | protocol violation path |
| `capability_minimal` | only minimal caps; missing plan/file/terminal |
| `clean_shutdown_stdin_close` | exit after stdin EOF |

### 5.4 Identity and path rules

- Use fixed UUIDs from specifications where asserted.
- Paths in fake payloads use placeholders / repo-relative forms (`{{PROJECT_ROOT}}`, `src/main.rs`) — never machine drive letters in fixtures.

## 6. What to assert (normative)

### 6.1 Event types (W0-A strings)

Contract and fake integration tests MUST assert these product types where scenarios apply:

| Lifecycle | Types |
|---|---|
| Process | `runtime.process.started`, `runtime.process.ready`, `runtime.process.stderr`, `runtime.process.exited`, `runtime.process.failed` |
| Session | `session.created`, `session.ready`, `session.prompt.submitted`, `session.status.changed`, `session.completed`, `session.failed`, `session.cancelled` |
| Agent | `agent.message.delta`, `agent.message.completed`, `agent.progress.delta`, `agent.plan.updated` |
| Tools | `tool.started`, `tool.updated`, `tool.completed`, `tool.failed` |
| Approvals | `approval.requested`, `approval.resolved` |
| Adapter | `adapter.protocol.error`, `adapter.protocol.unknown` |
| Storage | `storage.error` (when persistence fails) |

**Do not** assert W0-B conceptual names (`message.agent.delta`, `permission.requested`, `turn.started`, `runtime.initialized`) as product types.

### 6.2 Session status values

Assert control-plane statuses only from:

```text
creating, starting_runtime, ready, running, awaiting_approval,
cancelling, completed, failed, disconnected, stopped
```

### 6.3 Error classes

Assert stable `errorClass` strings from adapter/Tauri contracts, including (non-exhaustive):

```text
RuntimeExecutableNotFound, RuntimeSpawnFailed, RuntimeNotReady,
RuntimeDisconnected, RuntimeCrashed, ProtocolInitializeFailed,
CapabilityMismatch, CapabilityUnsupported, ProtocolParseError,
ProtocolViolation, SessionNotFound, PromptRejected, CancellationFailed,
ApprovalUnknown, PermissionDenied, Timeout, InvalidArgument,
InvalidState, NotFound, StorageError, InternalError / InternalAdapterError
```

**Additive recommendation (Stage 0.1):** when Wave 1 implements auth, prefer introducing `AuthenticationRequired` and `AuthenticationFailed`. Until added, tests must still prove auth-gate behavior maps to a **non-success**, user-visible error — not silent ready. Specification scenarios use expected class aliases under `errorClass` / `errorClassAnyOf` for forward compatibility.

### 6.4 Envelope rules

For every persisted/streamed event in T1–T3:

- `eventVersion === 1`
- required fields present (`eventId`, `sequence`, `timestamp`, `projectId`, `sessionId`, `agentRunId`, `type`, `payload`)
- `sequence` monotonic +1 per session
- unknown `type` tolerated by consumers
- no secrets / no machine-absolute paths in committed fixtures

## 7. Layered test ownership (Wave 1 mapping)

| Area | Suggested location (informative) | Tiers |
|---|---|---|
| Domain envelope / Zod or serde types | domain crates / packages | T0, T1 |
| ACP framing + normalizer | adapter crate | T1, T2 |
| Process manager | process crate | T2, T5 |
| Storage | storage crate | T3 |
| Control plane + Tauri commands | control-plane / src-tauri | T2, T3 |
| Fake ACP binary | tools or crates test support | T2 |
| UI components | apps/desktop | T4 |
| Live smoke scripts | tests/live or docs-runbooks | T6 |

Wave 0 does not create application source; paths above are planning guidance consistent with the master plan.

## 8. Required scenario families

### 8.1 Happy path (Gate 1 core)

1. Register project (path validation; missing path fails).
2. Create session → spawn fake runtime → `runtime.process.started` → initialize → `runtime.process.ready`.
3. Create/auth session as scripted → `session.ready`.
4. Submit prompt → `session.prompt.submitted` → stream `agent.message.delta` → tools → `session.completed` / return to `ready`.
5. Persist events; `tracer_events_list` returns ascending `sequence`.
6. Stop session → `runtime.process.exited` with `expected: true`; no orphans.

### 8.2 Auth-gate (stock shape)

1. Drive fixture/scenario `auth_required_session_new`.
2. Assert session is **not** ready for prompts.
3. Assert structured error (live-scrubbed code `-32000` / message Authentication required at wire; product error class per Wave 1 auth design).
4. Label evidence: structural contract from **live-scrubbed** fixture; full login UX is separate.

### 8.3 Approvals

1. `approval.requested` emitted; session status `awaiting_approval`.
2. Resolve allow → `approval.resolved` → tools complete.
3. Resolve deny → tool failed / cancelled path; fail closed.
4. Unknown approval kind never auto-approved.

### 8.4 Cancel + deadlock avoidance

1. Cancel mid-stream → `cancelling` → `session.cancelled` / stopped appropriately.
2. **Cancel while permission pending:** control plane must answer/cancel the reverse-request path and/or stop the process so the turn cannot hang forever. Acceptance: finishes within `T_cancel` + `T_term` bound; no deadlock; user-visible terminal status.
3. Cancel unsupported → `CapabilityUnsupported` then process stop fallback; still no orphans.

### 8.5 Crash / EOF / recovery

1. Unexpected exit mid-run → `runtime.process.exited` (`expected: false`) and/or `runtime.process.failed`; session `failed` or `disconnected`; **never** still `running`.
2. EOF mid-prompt → `RuntimeDisconnected` / exit events; no silent `session.completed`.
3. App restart → `tracer_events_list` replays history; runtime not auto-claimed running without process.
4. Storage interrupt → no corrupt sequence; `storage.error` if write fails visibly.

### 8.6 Unsupported capabilities

| Missing cap | Expected product behavior |
|---|---|
| `cancellation` | Cooperative cancel API fails closed with `CapabilityUnsupported`; process stop used |
| `approvals` | No auto-approve; tools requiring approval fail closed or never appear |
| `planUpdates` | No synthetic plans; plan UI empty/hidden |
| `promptStreaming` | Single `agent.message.completed` acceptable |
| `fileChangeNotifications` / `terminalOutput` | Features disabled; not hard-fail MVP |

### 8.7 Vendor unknown

Unmapped vendor notification → `adapter.protocol.unknown`; session continues; UI does not crash; raw method only under adapter metadata.

### 8.8 Windows process / orphan cases

Mandatory platform scenarios (T5), especially Windows:

| Case | Expectation |
|---|---|
| Graceful stop (stdin close / cooperative) | Process exits; no leftover `grok`/fake children |
| Force kill after cancel timeout | Job Object / kill-tree; no orphan shells/PTYs |
| Crash of runtime mid-tool | Tracer observes exit; does not leave unmanaged grandchildren when process manager owns the job |
| App killed while runtime running | Document limitation + best-effort job kill-on-close |
| Named-pipe leader mode | **Out of MVP**; Tracer uses `--no-leader` for stock; tests must not require leader |

macOS/Linux: process-group / setsid kill coverage in multi-OS CI when available; Windows Job Object notes remain Windows-specific (do not generalize “sandbox enforce” to Windows).

## 9. Event ordering tests

| Rule | Test |
|---|---|
| Monotonic `sequence` | After happy path, list events; `sequence == index` (1-based) |
| Ready before prompt success | No `session.prompt.submitted` before `runtime.process.ready` in successful runs |
| Approval bracketing | `approval.requested` before `approval.resolved` for same `approvalId` |
| Exit ends running | After `runtime.process.exited`, no further successful prompt accepts on same handle |
| Batch reorder forbid | If batching used, persisted order still ascending |
| Replay stability | Restart app/process; replayed sequences match stored |

## 10. Storage recovery tests (T3)

1. Fresh DB + migrations apply.
2. Migration re-run is idempotent.
3. Persist full envelopes including unknown types/fields.
4. Ordered read by `sequence`.
5. Interrupted write does not yield partial success claims.
6. DB path via platform app-data APIs (no hardcoded user homes in code).
7. Runtime must not open SQLite for writes.

## 11. UI / command contract tests (T4)

1. Components render from **normalized** events only (fixture packs under `tests/specifications/expected-events/`).
2. Status banners for `failed`, `disconnected`, `awaiting_approval`, `cancelling`.
3. Command errorClass mapping to user-visible messages (no raw stack traces as sole UX).
4. Accessibility: non-color-only status (aligned with W0-C when present; backend statuses are source of truth).
5. Explicitly assert UI tests do **not** import ACP wire parsers.

## 12. Minimal CI matrix

### 12.1 Standard CI (required, every PR)

| Job | OS | Commands (illustrative) | Includes |
|---|---|---|---|
| `ci-rust` | linux (primary) | `cargo test --workspace` (or scoped crates) | T0–T3, fake runtime |
| `ci-frontend` | linux | `pnpm`/`npm` test + typecheck | T0/T4 |
| `ci-fixtures` | linux | fixture schema validation / contract tests | T1 |

**Forbidden in standard CI:**

- network calls to model providers;
- reading `XAI_API_KEY` / user auth stores for success;
- requiring installed stock `grok` binary;
- paid API quotas.

### 12.2 Scheduled / optional jobs

| Job | When | Includes |
|---|---|---|
| `ci-windows-process` | PR if runners available; else nightly | T5 orphan/kill matrix |
| `ci-macos-process` | nightly or when available | T5 |
| `live-smoke` | manual / nightly with secrets | T6; may consume provider usage |
| `desktop-e2e` | post–Gate 1 stabilization | T7 with fake runtime |

### 12.3 Feature flags / env gates

```text
TRACER_LIVE_SMOKE=1          # enable T6
TRACER_STOCK_RUNTIME=grok    # binary name on PATH
TRACER_FAKE_ACP_SCENARIO=…   # fake scenario id
# Secrets only via CI secret store for live jobs — never committed
```

### 12.4 CI time budgets (guidance)

| Suite | Soft budget |
|---|---|
| Unit + contract | < 2 min |
| Fake integration | < 5 min |
| Full standard CI | < 15 min |
| Live smoke | isolated; not on critical PR path |

## 13. Gate 1 evidence package

To claim Gate 1, maintainers need:

1. **CI green** on standard matrix (fake path).
2. **Acceptance scenario results** mapped in `VERTICAL_SLICE_ACCEPTANCE.md` (automated where possible).
3. **Failure matrix coverage** for crash, EOF, cancel, orphan, auth-gate, cancel-while-permission-pending.
4. **Provenance honesty:** synthetic vs live-scrubbed vs live-authenticated labeled in reports.
5. **Platform limitations** documented (especially Windows sandbox absence, optional live auth gaps).
6. **No orphan affidavit:** process tests or manual signed checklist on at least Windows or Linux with clear OS label.
7. **Stock smoke (documented):** at least runbook + optional result; not required green in standard CI. Authenticated prompt stream remains optional until credentials available.

## 14. What Wave 0 does not implement

- Full `cargo test` / Vitest suites
- Fake ACP binary source
- CI YAML workflows (may be added in Wave 1 infra tasks)
- Live provider calls

Wave 0 **does** provide:

- strategy and acceptance definitions;
- failure matrix;
- specification catalogs and expected normalized event sequences under `tests/specifications/`.

## 15. Change control

After Gate 0 freeze:

- additive scenarios: minor update + tests;
- changing expected W0-A types or success criteria: contract-change process (proposal, impact, fixtures, notify tasks);
- new error classes (auth): document in contracts then update this strategy’s aliases.

## 16. Assumptions

1. Fake ACP will implement enough standard ACP for all default CI scenarios.
2. Stock start command for smoke remains `grok agent --no-leader stdio` (Stage 0.1).
3. Control plane assigns `eventId` / `sequence` / `timestamp`.
4. Framing for production adapter is NDJSON for stock + fake.
5. W0-C UX binds to the same status set; tests use backend statuses as truth.

## 17. Risks

| Risk | Mitigation |
|---|---|
| Implementers assert W0-B conceptual event names | Explicit ban + fixture expected-events use W0-A only |
| CI accidentally needs `grok` + API key | Standard CI job forbid list + fake default |
| Synthetic treated as live parity | Provenance labels + acceptance wording |
| Permission cancel deadlock | Dedicated scenario + time-bound acceptance |
| Windows orphans | Job Object / kill-tree T5 scenarios |
| Auth error class not yet in v1 contract | `errorClassAnyOf` until additive contract bump |

---

**Document control:** W0-D deliverable for `tracer-w0-test-strategy`. Amend with contract-change process after Gate 0.
