# Requirement Traceability Matrix (Gate 0 → Wave 1)

**Status:** Gate 0 output  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-final-integration`  
**Purpose:** Map each vertical-slice product requirement to architecture decision, contract, UX state, test scenario, and Wave 1 owner. No critical requirement lacks a Wave 1 owner.

## 1. Legend

| Column | Meaning |
|---|---|
| **Req ID** | Stable requirement id for Gate 0/1 tracking |
| **Product requirement** | User/system need for first slice |
| **Architecture / ADR** | Design decision source |
| **Contract** | Normative freeze doc section |
| **UX state / surface** | Visible product binding |
| **Test scenario** | VS / F / catalog id |
| **Wave 1 owner** | Primary module; secondary in parentheses |

## 2. Core vertical-slice requirements

| Req ID | Product requirement | Architecture / ADR | Contract | UX state / surface | Test scenario | Wave 1 owner |
|---|---|---|---|---|---|---|
| R-01 | Register a local project (folder path) | Vertical slice §2; control plane owns projects | `TAURI_COMMAND_CONTRACT_V1` `tracer_project_*` | Projects home; project list statuses | VS-14; F-U05 | **W1-F** (W1-E persist; W1-A shell) |
| R-02 | Open project and list sessions | Vertical slice | `tracer_session_list` / `get` | Project workspace session list | VS-01 preconditions; T0 list | **W1-F** (W1-E; W1-A) |
| R-03 | Create session bound to project | Vertical slice happy path | `tracer_session_create`; session statuses | Creating → Starting runtime | VS-01 | **W1-F** (W1-D session create; W1-C spawn) |
| R-04 | Spawn managed sidecar runtime (stdio ACP) | **ADR-001** runtime sidecar | Adapter connect/start; process events | RuntimePill starting/ready | F-P01–P04; process tests | **W1-C** (W1-D init; W1-F compose) |
| R-05 | Stock Grok spawn path when using Grok | Stage 0.1 / PROCESS_LIFECYCLE | Installation descriptor; not illustrative-only for Grok | Runtime unavailable vs ready | Optional T6; docs smoke | **W1-C** config + **W1-D** (W1-F) |
| R-06 | Initialize + capability negotiation | Adapter lifecycle; readiness synthesis | RUNTIME_ADAPTER caps; `runtime.process.ready` | Capabilities footer; progressive disclosure | VS-01; VS-12 | **W1-D** |
| R-07 | Distinguish process-ready vs session-ready | Stage 0.1 auth boundary | Adapter gates; command `InvalidState` / auth | Dual RuntimePill + StatusChip; auth panel | VS-02; F-A05 | **W1-D** + **W1-F** (W1-A display) |
| R-08 | Authenticate when required before usable session | W0-B live-scrubbed auth error | Additive auth error classes recommended | Auth required / failed banners | VS-02; F-A01/A02; VS-L1 optional | **W1-D** + **W1-F** (W1-A) |
| R-09 | Submit prompt | Vertical slice | `tracer_session_submit_prompt` | Composer enabled only when `ready` | VS-01; F-A03/A04 | **W1-F** (W1-D prompt) |
| R-10 | Stream normalized agent/tool/plan events | **ADR-002** normalization | EVENT_PROTOCOL catalog | Timeline cards by `type` | VS-01; expected `happy_prompt_stream` | **W1-D** normalize; **W1-B** types; **W1-F** stream; **W1-A**/features display |
| R-11 | UI consumes only normalized events (no raw ACP) | ADR-002; vertical slice rules | Event protocol; Tauri stream | Timeline never needs ACP methods | VS-13; F-U03 | **W1-A** + feature modules; **W1-F** gate |
| R-12 | Show runtime state honestly | Process lifecycle | `runtime.process.*` events | RuntimePill; disconnect banners | VS-06/07; F-U04 | **W1-C** emit; **W1-F**; **W1-A** |
| R-13 | Approval interrupt fail-closed | Vertical slice permissions | `approval.requested` / `resolved`; `tracer_approval_*` | Awaiting approval; Approvals tab | VS-03; F-C06–C09 | **W1-F** permissions; **W1-D** map; **W1-A**/approvals UI |
| R-14 | Cancel active run | Adapter cancel + process fallback | `tracer_session_cancel`; cancellation capability | Cancelling → stopped | VS-04; VS-11; F-C01–C03 | **W1-D** + **W1-C** + **W1-F** |
| R-15 | Cancel while permission pending without deadlock | Stage 0.1 risk handoff | cancel + approval resolve paths | Leave awaiting; terminal within budget | VS-05; F-C04/C05 | **W1-F** + **W1-D** |
| R-16 | Stop runtime without orphans | ADR-001; Windows Job Object | `tracer_session_stop`; process exit expected | Stopped; no fake running | VS-09; F-P10/P11; F-W01 | **W1-C** + **W1-F** |
| R-17 | Persist session + events (control plane writer only) | Vertical slice persistence | Storage responsibilities; envelope fields | History after restart | VS-10; F-S01–S05 | **W1-E** + **W1-F** |
| R-18 | Reload session history after app restart | Vertical slice | `tracer_events_list`; sequence monotonic | Timeline replay read-only | VS-10; F-S04 | **W1-E** + **W1-F** (W1-A) |
| R-19 | Crash / EOF mid-prompt not silent success | Failure design | `RuntimeCrashed` / `RuntimeDisconnected` | Disconnected/failed; never Running | VS-06; VS-07; F-P06/P07 | **W1-C** + **W1-D** + **W1-F** |
| R-20 | Malformed / unknown protocol frames tolerated | Event unknown rules | `adapter.protocol.error` / `unknown` | Generic error / unknown cards | VS-08; F-R01–R05 | **W1-D** (W1-B unknown types) |
| R-21 | Unsupported capability degrades safely | Capability matrix product view | `CapabilityUnsupported` / mismatch | Capability banners; cancel force-stop helper | VS-11; VS-12; F-R07/R08 | **W1-D** + **W1-F** (W1-A) |
| R-22 | File change / plan display when events exist | Progressive disclosure | `file.*` / `agent.plan.*` events | Changes + Plan tabs empty reasons | VS-01 optional tools/files; VS-12 | **W1-D** map; UI features (post shell); **W1-F** |
| R-23 | Standard CI without network/paid APIs | Test strategy | N/A product; harness policy | N/A | All required VS via fake; matrix.yaml | **W1-G** + module tests |
| R-24 | Deterministic fake ACP scenario catalog | Test strategy §5 | Aligns adapter contract | UX testability hooks | catalog.yaml all standardCi scenarios | **W1-G** |
| R-25 | Normative W0-A event type strings in tests | Gate 0.1 naming authority | EVENT_PROTOCOL | UI binds same strings | expected-events `forbiddenProductTypeAliases` | **W1-B** + **W1-D** + **W1-G** |
| R-26 | Vendor extensions non-blocking for MVP | CAPABILITY_MATRIX; FORK_RISK | unknown preservation only | Runtime tab debug only | `unknown_vendor_notification` | **W1-D** |
| R-27 | No Grok Build fork for vertical slice | ADR-001; FORK_RISK | stock sidecar | N/A | Architecture review | All W1; coordinator |
| R-28 | Accessibility: status not color-only | UX IA / STATE_MATRIX | N/A (UX freeze) | StatusChip text+icon | T4 UI contract; STATE_MATRIX A1–A5 | **W1-A** (+ feature UIs) |
| R-29 | No full IDE in MVP shell | UX IA scope fence; vertical slice OOS | N/A | Session-centric layout | Review / Gate 1 | **W1-A** |
| R-30 | HeliHarness concurrent task discipline for W1 | Master plan; harness | N/A | N/A | Template dry-run | **W1-H** |

## 3. Lifecycle state coverage (cross-check)

| Lifecycle / UX state | Contract / backend source | UX doc | Test | Wave 1 owner |
|---|---|---|---|---|
| Runtime not installed | `RuntimeExecutableNotFound` | Flow C; STATE_MATRIX runtime | F-P01 | W1-C, W1-F, W1-A |
| Runtime starting | `runtime.process.started` | StatusChip starting_runtime | process integration | W1-C, W1-F |
| Runtime process ready | `runtime.process.ready` | RuntimePill ready | VS-01 | W1-D, W1-F |
| Authentication required | auth-required fixture / error | AuthSetupPanel | VS-02; F-A01 | W1-D, W1-F, W1-A |
| Authentication failed | auth error mapping | Auth failed banner | F-A02 | W1-D, W1-F, W1-A |
| ACP initialize | initialize exchange | Diagnostics optional | T1 fixtures | W1-D |
| Capability negotiation | caps on ready payload | Capability banners | VS-12 | W1-D, W1-A |
| Session creation | `session.ready` / status | StatusChip ready | VS-01 | W1-D, W1-F |
| Prompt submitted | `session.prompt.submitted` | User bubble; running | VS-01 | W1-F, W1-D |
| Streaming | `agent.message.delta` | Timeline growth | VS-01 | W1-D, W1-B, UI |
| Waiting for approval | `awaiting_approval` | Interrupt | VS-03 | W1-F, W1-D, UI |
| Cancelling / cancelled | `cancelling` → `stopped` | Cancelling labels | VS-04/05/11 | W1-F, W1-D, W1-C |
| Completed | `completed` / ready multi-turn | Completed marker | VS-01 | W1-F |
| Failed | `failed` + errors | Failed banner | multiple F-* | W1-F |
| Disconnected / crashed | exit unexpected | Disconnect honesty | VS-06/07 | W1-C, W1-F, W1-A |
| Runtime crashed | `RuntimeCrashed` | Crash banner | VS-06 | W1-C, W1-D, W1-F |
| Recovery / resume history | storage + list events | Flow A step 8 | VS-10; F-S04 | W1-E, W1-F, W1-A |

## 4. Ownership completeness check

| Critical vertical-slice area | Wave 1 owner assigned? |
|---|---|
| Desktop shell / placeholders | Yes — W1-A |
| Domain/event types | Yes — W1-B |
| Process lifecycle / orphans | Yes — W1-C |
| ACP + normalization | Yes — W1-D |
| SQLite persistence | Yes — W1-E |
| Tauri commands + composition | Yes — W1-F |
| Fake runtime CI | Yes — W1-G |
| Harness workflows | Yes — W1-H |
| Auth gate product path | Yes — W1-D + W1-F (+ W1-A) |
| Approvals fail-closed | Yes — W1-F + W1-D |
| Gate 1 acceptance evidence | Yes — W1-G harness + W1-F E2E + per-module tests |

**Result:** No critical vertical-slice requirement without a Wave 1 owner.

## 5. Evidence provenance requirements (tests)

| Evidence label | May claim | Must not claim | Owner of enforcement |
|---|---|---|---|
| synthetic | Structural mapping | Live model parity | W1-G, W1-D tests |
| live-scrubbed | Wire shape fidelity | Interactive login success | W1-D T1 |
| fake-runtime | Product logic Gate 1 | Stock Grok auth UX | W1-G + W1-F |
| live-authenticated | Optional stock smoke | Default CI green | Optional T6 operator |

## 6. Related Gate 0 artifacts

- `docs/integration/FINAL_GATE_0_REPORT.md` — decision, SHAs, risks  
- `docs/integration/WAVE_1_READINESS_MATRIX.md` — launch sequencing  
- `docs/integration/STAGE_0_1_INTEGRATION_REPORT.md` — A+B reconciliation  

---

**Document control:** Update when requirements split or Wave 1 ownership changes; keep Req IDs stable.
