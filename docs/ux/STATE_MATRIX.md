# Tracer UX State Matrix (MVP)

**Status:** Wave 0 UX freeze candidate (Stage 0.2)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-product-ux` (W0-C)  
**Normative session statuses (W0-A):**  
`creating` · `starting_runtime` · `ready` · `running` · `awaiting_approval` · `cancelling` · `completed` · `failed` · `disconnected` · `stopped`

## 1. Purpose

Map **every first-slice backend condition** to a visible, accessible UI state. Exit criteria for W0-C:

- Every first-slice backend state has a visible UI state.
- No critical state depends on color alone.
- Runtime disconnect and approval flows are unambiguous.
- Auth, capability gaps, process-up/session-fail, and synthetic limits are explicit.

## 2. Accessibility baseline (applies to all rows)

| Rule | Requirement |
|---|---|
| A1 | Status always has **text label** (+ icon preferred) |
| A2 | Color is redundant reinforcement only |
| A3 | Blocking conditions use banner or interrupt with `aria-live` |
| A4 | Disabled controls include **visible reason** (helper text or tooltip) |
| A5 | Icons have accessible names when not adjacent to visible text |

## 3. Session status matrix (authoritative)

| Status | Header label | Composer | Cancel | Stop | Timeline treatment | Side pane | User goal |
|---|---|---|---|---|---|---|---|
| `creating` | Creating session | Disabled: “Creating session…” | Hidden/disabled | Enabled (abort) | Skeleton / session.created when arrives | Idle | Wait or abort |
| `starting_runtime` | Starting runtime | Disabled: “Starting runtime…” | Hidden/disabled | Enabled | Runtime started/ready events | Runtime tab useful | Wait for ready or fail |
| `ready` | Ready | **Enabled** | Hidden/disabled | Enabled | Empty prompt CTA or history | Normal | Submit prompt |
| `running` | Running | Disabled: “Agent is working…” | **Enabled** | Enabled | Live deltas, tools, progress | Plan/tools update | Monitor / cancel |
| `awaiting_approval` | Waiting for approval | Disabled: “Approval required” | **Enabled** | Enabled | Interrupt card + tool park | Approvals focused | Allow / deny / cancel |
| `cancelling` | Cancelling | Disabled: “Cancelling…” | Disabled (in progress) | Enabled (escalate) | Cancel-related events | Mostly read-only | Wait for terminal |
| `completed` | Completed | Disabled unless product returns to `ready` for next prompt* | Hidden | Enabled if process still up | Terminal success marker | Review plan/changes | Review / new session |
| `failed` | Failed | Disabled | Hidden | Enabled if process still up | Error + failed marker | Diagnostics | Read error / retry new session |
| `disconnected` | Disconnected | Disabled | Hidden | Enabled (cleanup) | Exit/crash events | Diagnostics | Understand crash / new session |
| `stopped` | Stopped | Disabled | Hidden | Disabled or no-op | Cancelled/stopped markers | Review history | Review / new session |

\*If control plane uses `completed` only for a single run and returns to `ready` for multi-turn, UI follows **current** status from `tracer_session_get` / `session.status.changed`. Do not hardcode single-turn.

## 4. Runtime process matrix

Orthogonal to session status. Header **RuntimePill** + banners.

| Runtime condition | Detection (normative signals) | UI | Session typically |
|---|---|---|---|
| Not started | No process events; early create | Pill: not started | `creating` |
| Starting | `runtime.process.started` | Pill: starting; spinner text | `starting_runtime` |
| Ready (initialized) | `runtime.process.ready` | Pill: ready; capabilities in footer | often → `ready` after session.ready |
| **Unauthenticated** | Process ready or init ok; session create / auth probe requires sign-in | Banner: Sign in required; AuthSetupPanel | not prompt-ready (`failed`/`creating` recovery or dedicated holding — see §5) |
| **Authenticated** | Session reaches `ready` after auth+session/new | No auth banner | `ready` |
| Stderr noise | `runtime.process.stderr` | Diagnostics; optional collapse | any |
| Expected exit | `runtime.process.exited` `expected: true` | Pill: stopped | `stopped` / `completed` |
| Unexpected exit / crash | `exited` `expected: false` and/or `runtime.process.failed` | Banner disconnect/crash; **never Running** | `disconnected` or `failed` |
| Spawn failure | Command error `RuntimeSpawnFailed` / `RuntimeExecutableNotFound` | Global or session banner | `failed` |
| Protocol init failure | `ProtocolInitializeFailed` / failed events | Banner: started but handshake failed | `failed` |

### 4.1 Process started but session creation failed

| Field | Spec |
|---|---|
| Symptoms | `runtime.process.started` and maybe `ready`, but `createSession` / session path errors; session status `failed` or stuck non-ready |
| Causes | Auth required; protocol violation; invalid cwd; internal adapter error |
| UI | Dual honesty: RuntimePill may show **ready/up**; Session StatusChip **Failed** (or auth banner if auth). Banner: “Runtime is running, but the session could not be created.” + `errorClass` + message |
| Actions | Retry session create (if safe), complete auth, **Stop** runtime to avoid orphans |
| Forbidden | Implying the user can prompt |

## 5. Authentication matrix

| Product state | User-visible | Composer | Primary actions |
|---|---|---|---|
| Auth not required (e.g. fake runtime) | No auth UI | Per session status | Normal |
| **Unauthenticated — auth required** | Banner + method chooser | Disabled: “Sign in required” | Continue auth, cancel/stop session |
| Auth in progress | Banner: “Signing in…” | Disabled | Wait / cancel |
| **Auth failed** | Banner error: “Sign-in failed” + message | Disabled | Retry auth, change method, stop |
| Authenticated, session ready | No auth banner | Per status | Prompt |
| Auth expired mid-session (if observed) | Warning banner: “Re-authentication required” | Disabled until resolved | Re-auth or stop |

**Copy rules:**

- Prefer specific language (“Authentication required”, “Sign-in failed”) over generic “Something went wrong”.
- Map future `AuthenticationRequired` / `AuthenticationFailed` error classes when W1 adds them (Gate 0.1 recommendation); until then, use structured message detection from control plane `lastError` without parsing raw ACP in UI.

## 6. Capability-missing matrix

Based on Tracer capability keys from `RUNTIME_ADAPTER_CONTRACT_V1.md` / `runtime.process.ready`.

| Capability | If false / missing | Visible UI |
|---|---|---|
| `promptStreaming` | Final message only | No live typing indicator required; still show `agent.message.completed` |
| `cancellation` | Cooperative cancel unsupported | Cancel may trigger process-stop path; helper: “Cancel will stop the runtime process” |
| `planUpdates` | No plan events expected | Plan tab empty: “Plan updates not available for this runtime” |
| `toolCalls` | No tools expected | Timeline simply lacks tool cards; do not mock tools |
| `approvals` | No approval events | Approvals empty; **if** a tool still requires policy approval, fail closed via control plane — show error, never auto-allow |
| `fileChangeNotifications` | No file events | Changes empty with capability explanation |
| `terminalOutput` | No terminal events | Hide terminal styling; no fake console |
| `sessionResume` | No runtime resume | “Resume” control hidden; history still reloadable from Tracer DB |
| Hard `CapabilityMismatch` | Session not ready | Error banner listing mismatch; status `failed` |

Optional vendor capabilities under adapter metadata **must not** enable core MVP features without contract promotion.

## 7. Approval state matrix

Events: `approval.requested` / `approval.resolved` only (no parallel permission event names).

| State | Session status | UI |
|---|---|---|
| No pending | not `awaiting_approval` | Approvals empty |
| Pending | `awaiting_approval` | Interrupt + tab badge + composer disabled |
| Resolving | still awaiting until event | Buttons disabled, “Submitting decision…” |
| Resolved allow | → `running` or per backend | Timeline shows allowed; interrupt closes |
| Resolved deny | tool fails / run continues or ends per backend | Timeline shows denied; clear tool failed if emitted |
| Resolved cancel | per backend | Same family as deny/cancel request |
| Unknown risk | pending | Risk label “Unknown — review carefully”; **do not** auto-allow; prefer neutral focus |
| Duplicate resolve | command `InvalidState` | Toast: already resolved; refresh pending list |
| User ignores forever | stays awaiting | Cancel run / Stop still available; fail closed (no implicit allow on timeout in MVP) |

## 8. Agent run / timeline activity matrix

| Condition | Visible |
|---|---|
| Idle ready | Empty CTA or prior history |
| Prompt submitted | User bubble; status often `running` |
| Streaming text | Assistant bubble grows |
| Progress only | Progress line |
| Tool running | Tool card Running |
| Tool failed | Tool card Failed + message |
| Plan update | Plan tab + optional timeline notice |
| Unknown event | Generic card with `type` string |
| Protocol error (process alive) | Error card; session may continue |
| Storage error | Banner + optional card; do not claim saved |

## 9. Terminal session outcomes

| Outcome | Status | Banner / marker | Next step CTA |
|---|---|---|---|
| Success complete | `completed` | Success marker | New session / return to list (or prompt if back to `ready`) |
| User cancelled | `stopped` (after `cancelling`) | “Cancelled” marker; partial tools honest | New prompt only if status allows; else new session |
| User stopped | `stopped` | “Stopped” | New session |
| Failed (logical) | `failed` | Error message / `session.failed` | Fix config / new session |
| Crash / disconnect | `disconnected` or `failed` | Disconnect banner; exit details | New session; do not “Continue” as if process alive |
| Exit before ready | `failed` | “Runtime failed during startup” | Fix runtime / retry |

## 10. Command error class → UI mapping

| errorClass | UI treatment |
|---|---|
| `InvalidArgument` | Inline form validation |
| `NotFound` | Toast + navigate back if session missing |
| `AlreadyExists` | Inline on register project |
| `InvalidState` | Toast with current status explanation |
| `PermissionDenied` | Toast / approval deny explanation |
| `RuntimeExecutableNotFound` | Error banner + configure CTA |
| `RuntimeSpawnFailed` | Error banner |
| `RuntimeNotReady` | Disable composer; helper text |
| `RuntimeDisconnected` | Disconnect banner; disable mutations |
| `RuntimeCrashed` | Crash banner |
| `ProtocolInitializeFailed` | Startup failure banner |
| `CapabilityMismatch` | Capability error banner |
| `CapabilityUnsupported` | Explain fallback (e.g. force stop) |
| `CancellationFailed` | Error + offer Stop force |
| `Timeout` | Timeout message + retry guidance |
| `StorageError` | Storage banner |
| `InternalError` | Generic error with support details expander |
| `Unsupported` | Feature unavailable copy |
| `UserCancelled` | Silent or mild toast on dialog cancel |
| `PromptRejected` | Toast/banner on submit failure |
| Auth-related (when added) | Auth banners (§5) |

## 11. Projects / non-session states

| State | UI |
|---|---|
| No projects | Empty projects: illustration + “Open a local repository” |
| Project ready | List row normal |
| Project missing path | Status text “Folder missing” + warning icon (not color-only) |
| Project invalid | “Invalid project” + remove/re-register |
| No sessions | Empty: “Create a session to run an agent” |
| Loading lists | Skeleton rows + “Loading…” text |
| List fetch error | Error panel with retry |

## 12. Empty / loading / running / failed / disconnected / completed (product shorthand)

Master plan exit language mapped to concrete statuses:

| Product shorthand | Includes backend states | Key UI |
|---|---|---|
| **Empty** | No projects/sessions/events | CTA empties (§11, timeline empty) |
| **Loading** | `creating`, `starting_runtime`, list fetches, auth in progress | Spinners **with text** |
| **Running** | `running` | Live timeline; cancel available |
| **Failed** | `failed` + command hard failures | Error banners; diagnostics |
| **Disconnected** | `disconnected` + unexpected process death | Disconnect honesty |
| **Completed** | `completed` | Success terminal; review changes |
| **Cancelled / stopped** | `cancelling` → `stopped` | Explicit cancelled/stopped labels |
| **Awaiting approval** | `awaiting_approval` | Interrupt fail-closed |

## 13. Honesty rules (non-negotiable)

1. **Never** show session status Running after `runtime.process.exited` / `runtime.process.failed` for that binding.
2. **Never** auto-approve unknown permission requests.
3. **Never** require UI to parse raw ACP or Grok vendor frames for core behavior.
4. **Never** present synthetic fixture streams as live authenticated model evidence without labeling.
5. **Never** claim persistence succeeded after `storage.error` / `StorageError`.
6. **Never** enable prompt while unauthenticated when auth is required for session readiness.
7. Prefer event-authoritative status over optimistic UI when they conflict.

## 14. State transition diagram (UX view)

```text
                 ┌─────────────┐
                 │  creating   │
                 └──────┬──────┘
                        ▼
              ┌─────────────────────┐
              │ starting_runtime    │──────► failed / disconnected
              └──────────┬──────────┘
                         ▼
              ┌─────────────────────┐
         ┌────│ auth gate (UI)      │──── failed (auth)
         │    └──────────┬──────────┘
         │               ▼
         │         ┌─────────┐
         │         │  ready  │◄──────────────────┐
         │         └────┬────┘                   │
         │              ▼                        │
         │         ┌─────────┐    approval     ┌─┴────────────────┐
         │         │ running │◄───────────────►│ awaiting_approval│
         │         └────┬────┘                 └────────┬─────────┘
         │              │ cancel                        │ cancel
         │              ▼                               ▼
         │         ┌───────────┐                 (same cancelling path)
         │         │cancelling │
         │         └─────┬─────┘
         │               ▼
         │    stopped / disconnected / failed
         │
         └──► completed / failed / stopped / disconnected
```

Auth gate is a **product UI phase** that may span control-plane statuses depending on W1 implementation (e.g. hold before `ready`, or `failed` with auth error + retry). UX must remain specific either way.

## 15. Testability hooks (for W0-D / W1)

Each matrix row should be exercisable with:

- Fake ACP runtime scenarios (preferred CI)
- Sanitized fixtures labeled live vs synthetic
- Command error injection for `errorClass` mapping

UI tests assert **text labels** and disabled reasons, not color tokens alone.

---

**Document control:** W0-C deliverable. If W0-A status set changes via formal contract revision, update this matrix in the same change set.
