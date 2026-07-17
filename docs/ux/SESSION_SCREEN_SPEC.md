# Session Screen Specification (MVP)

**Status:** Wave 0 UX freeze candidate (Stage 0.2)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-product-ux` (W0-C)  
**Screen:** Session workspace (`projects/:projectId/sessions/:sessionId`)  
**Normative statuses:** `creating`, `starting_runtime`, `ready`, `running`, `awaiting_approval`, `cancelling`, `completed`, `failed`, `disconnected`, `stopped`

## 1. Purpose

Specify the **primary vertical-slice UI**: layout regions, components, density, and binding rules so Wave 1 desktop shell and Wave 2 feature modules implement one consistent session workspace.

This is a **control-plane session view**, not an IDE.

## 2. Layout overview

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ SessionHeader: title · StatusChip · RuntimePill · [Cancel] [Stop]        │
├──────────────────────────────────────────────────────────────────────────┤
│ SessionBannerRegion (auth / disconnect / capability / storage)           │
├──────────────────────────────────┬───────────────────────────────────────┤
│                                  │ SidePane tabs: Plan | Approvals |     │
│ TimelinePane                     │ Changes | Runtime                     │
│  (messages, tools, plan notes,   │                                       │
│   protocol cards, system)        │  tab body                             │
│                                  │                                       │
│                                  │                                       │
├──────────────────────────────────┴───────────────────────────────────────┤
│ PromptComposer  [attachments stub]                    [Send]             │
├──────────────────────────────────────────────────────────────────────────┤
│ SessionFooter: capabilities · lastError · sequence · runtime kind        │
└──────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Responsive / minimum sizes (guidance)

| Region | Guidance |
|---|---|
| Window min | ~1024×640 logical px for comfortable split |
| Timeline min width | ~360px |
| Side pane default | ~320px; collapsible to icon rail |
| Header | single row preferred; wrap controls on narrow |
| Composer | fixed bottom of session workspace, not global app |

Mobile-first layouts are out of MVP scope (desktop Tauri).

### 2.2 Density

- Timeline cards: comfortable (8–12px vertical gap).
- Tool cards: collapsed by default with one-line summary; expand for I/O.
- No multi-column editor chrome.

## 3. Session header

### 3.1 Contents

| Element | Spec |
|---|---|
| Back control | Returns to project session list; warn if `running` or `awaiting_approval` |
| Title | Session title or truncated first prompt; editable later (optional MVP) |
| **StatusChip** | Text + icon for W0-A status (see §3.2) |
| **RuntimePill** | Process health: starting / ready / exited / unavailable |
| Cancel | Visible when status ∈ {`running`, `awaiting_approval`}; maps to `tracer_session_cancel` |
| Stop | Always available for non-terminal; maps to `tracer_session_stop` (stronger than cancel) |
| Overflow | Optional: copy session id, open diagnostics |

### 3.2 StatusChip (normative labels)

Status is **never color-only**. Pattern: `[Icon] [Label]`.

| Backend status | Label (en) | Icon hint | Color role (hint only) |
|---|---|---|---|
| `creating` | Creating session | spinner / plus | neutral |
| `starting_runtime` | Starting runtime | spinner / plug | neutral |
| `ready` | Ready | check | success |
| `running` | Running | activity | info |
| `awaiting_approval` | Waiting for approval | shield / pause | warning |
| `cancelling` | Cancelling | spinner | warning |
| `completed` | Completed | check-circle | success |
| `failed` | Failed | error | danger |
| `disconnected` | Disconnected | unlink | danger |
| `stopped` | Stopped | stop | neutral |

Optional sublabel from `session.status.changed.reason` or `lastError.message` (one line, truncated).

### 3.3 RuntimePill

Distinct from session status (process can be up while session is not prompt-ready):

| Runtime observation | Pill label | Notes |
|---|---|---|
| No process yet | Runtime: not started | Early `creating` |
| Spawned, init in flight | Runtime: starting | After `runtime.process.started` |
| Init + caps done | Runtime: ready | `runtime.process.ready` |
| Process up, auth pending | Runtime: sign-in required | Gate 0.1 auth boundary |
| Expected exit | Runtime: stopped | `exited` expected true |
| Unexpected exit / crash | Runtime: crashed | `exited` expected false / `failed` |
| Executable missing | Runtime: unavailable | `RuntimeExecutableNotFound` |

## 4. Session banner region

Single stack of banners (max one critical + one warning recommended to avoid noise).

### 4.1 Auth required (high priority)

**When:** Control plane cannot complete session readiness because authentication is required (W0-B: initialize may succeed; session create fails without auth). Map command/`lastError` classes such as future `AuthenticationRequired` or interim protocol/prompt-rejected messaging that clearly indicates auth.

**UI:**

```text
┌─────────────────────────────────────────────────────────────┐
│ [shield] Sign in required to use this agent runtime         │
│ Choose an authentication method, then continue.             │
│ [ Method dropdown ]  [ Continue ]  [ View details ]         │
└─────────────────────────────────────────────────────────────┘
```

Rules:

- Prompt composer **disabled** with helper text “Sign in required”.
- Do not show as only “Session failed” without auth framing when auth is the cause.
- Method list comes from control plane / runtime setup surface (discovered methods), not hard-coded product marketing lists. UI shows display names; ids stay opaque.
- Auth UI does not parse raw Grok `x.ai/auth/*` frames in React; control plane owns the wire steps.

### 4.2 Runtime unavailable / capability mismatch

| Cause | Banner |
|---|---|
| `RuntimeExecutableNotFound` | “Agent runtime executable was not found. Configure an installation or install the runtime.” CTA: open runtime settings / docs link (in-app stub ok) |
| `RuntimeSpawnFailed` | “Could not start the agent runtime.” + message |
| `ProtocolInitializeFailed` | “Runtime started but failed protocol initialize.” |
| `CapabilityMismatch` | “Runtime is missing required capabilities for Tracer.” List missing keys from payload when present |
| Process started, session create failed (non-auth) | “Runtime is running, but the session could not be created.” Show `errorClass` + message; offer Stop + Retry create |

### 4.3 Disconnected / crashed

```text
[error icon] Runtime disconnected
The agent process exited while this session was active. Prompting is disabled.
[ View exit details ]  [ Start new session ]
```

Never show Running after exit events. If user had an in-flight tool, timeline marks tools failed/cancelled via events.

### 4.4 Storage error

From `storage.error` or command `StorageError`:

```text
[warning] Could not persist session data. On-screen history may not reload after restart.
```

### 4.5 Synthetic / fake runtime (dev & demos)

If app/runtime mode is fake or fixture-driven, show a **subtle** badge in header or footer:

```text
Demo runtime · not live model output
```

Do not present synthetic stream fixtures as authenticated live Grok transcripts (Gate 0.1 evidence boundary).

## 5. Timeline pane

### 5.1 Role

Authoritative chronological view of normalized events for the session. Virtualize when long (Wave 2 optimization allowed; MVP may simple-scroll).

### 5.2 Item types (renderers)

| Event type(s) | Card kind | Default presentation |
|---|---|---|
| `session.prompt.submitted` | User message | Right/secondary bubble; full text |
| `agent.message.delta` / `completed` | Assistant message | Stream into one bubble per `messageId` |
| `agent.progress.delta` | Progress | Subtle subline under running status or compact card |
| `agent.plan.updated` | Plan notice | One-line “Plan updated (n steps)” → focus Plan tab |
| `tool.started` / `updated` / `completed` / `failed` | Tool card | Name, status badge, expandable I/O |
| `approval.requested` | Approval interrupt | **Sticky or high-emphasis** card (see §7) |
| `approval.resolved` | Approval result | Compact “Allowed/Denied by user/policy” |
| `file.changed` / `file.diff.available` | Change notice | Path + kind; open Changes tab |
| `runtime.process.*` | System / runtime | Muted system style |
| `session.status.changed` | System | Optional collapsed; header already shows status |
| `session.completed` / `failed` / `cancelled` | Terminal markers | Clear section dividers |
| `adapter.protocol.unknown` | Generic | Type label + expandable JSON-safe payload |
| `adapter.protocol.error` | Error | severity error; message |
| `terminal.output` / `exited` | Terminal chunk | Monospace collapsible; only if capability/events exist |
| `storage.error` | Error | Persistent until dismissed or resolved |

### 5.3 Streaming rules

1. Coalesce `agent.message.delta` by `messageId` into one live bubble.
2. Tolerate batch envelopes from `tracer://events`.
3. On reconnect/open: `tracer_events_list` ordered by `sequence`; live stream continues after `latestSequence`.
4. Unknown fields ignored; unknown types still render.

### 5.4 Empty timeline

| Session status | Empty copy |
|---|---|
| `creating` / `starting_runtime` | Skeleton + “Starting session…” |
| Auth gate | “Sign in to start chatting with the agent.” |
| `ready` | “Session ready. Send a prompt to begin.” |
| Terminal (`completed` / `stopped` / `failed` / `disconnected`) with no events | “No events recorded for this session.” + error if any |

### 5.5 Tool card anatomy

```text
┌─────────────────────────────────────────────┐
│ [tool icon] tool.name          [status]     │
│ summary line (risk if present)              │
│ ▸ Inputs   ▸ Output                         │
└─────────────────────────────────────────────┘
```

Tool status labels: Pending · Running · Completed · Failed · Cancelled (text + icon).

Risk display is informational; it does **not** auto-approve.

## 6. Side pane tabs

### 6.1 Plan

- Source: latest `agent.plan.updated` snapshot (by `revision` if present).
- Steps: title + status (`pending` / `running` / `completed` / failed if mapped).
- Empty: “No plan yet. Plans appear when the agent shares one.”
- Hidden/disabled when `planUpdates` capability is false — show reason in empty state if tab still visible.

### 6.2 Approvals

- Pending list from events + `tracer_approval_list_pending`.
- Each row: action, description, risk, created time, Allow / Deny / Cancel.
- Decisions call `tracer_approval_resolve` with `allow` \| `deny` \| `cancel`.
- Fail closed: no “Allow always” in MVP unless control plane exposes a policy-backed option later; if runtime offers always-allow wire options, product MVP maps to explicit user action still gated by policy.
- Badge count on tab = pending approvals.

### 6.3 Changes

- List from `file.changed` / `file.diff.available`.
- Paths repo-relative; kind chips: created / modified / deleted / renamed.
- Diff: inline when small; otherwise “Open diff” using `diffId` reference when provided.
- Empty with capability false: “This runtime does not report file changes to Tracer.”
- Empty with capability true: “No file changes reported yet.”
- **Not** a full VCS client (no stage/commit UI in MVP).

### 6.4 Runtime / Diagnostics

- Process state, pid if available, runtime kind, negotiated capabilities.
- Recent `runtime.process.stderr` chunks (truncated flag honored).
- Last protocol errors.
- Adapter metadata expander (advanced): show that content is optional debug data; **UI logic must not branch on vendor-only keys** for core flows.
- Actions: refresh via `tracer_runtime_status`, copy diagnostics (sanitized).

## 7. Approval interrupt (in-timeline + modal-or-inline)

When `approval.requested` arrives and session → `awaiting_approval`:

1. StatusChip → Waiting for approval.
2. Approvals tab badge increments.
3. **Interrupt surface** appears above composer (or sticky within timeline viewport):

```text
┌──────────────────────────────────────────────────────────┐
│ Approval needed                                          │
│ {action} — {description}                                 │
│ Risk: {risk or “Unknown — review carefully”}             │
│ [ Allow ]  [ Deny ]  [ Cancel request ]                  │
└──────────────────────────────────────────────────────────┘
```

4. Focus moves to the interrupt’s primary action (Allow is **not** auto-focused if risk unknown — prefer Deny or a neutral container focus for fail-closed safety; recommended: focus the **card container**, not Allow).
5. Composer disabled until resolved or session leaves `awaiting_approval`.
6. On resolve: emit path completes via `approval.resolved`; interrupt dismisses; if still `running`, composer stays disabled until back to `ready`/`running` rules in state matrix.

**Queueing:** MVP assumes one blocking approval at a time for the active run (matches typical ACP permission park). If multiple pending appear, show list in Approvals tab; interrupt focuses **oldest unresolved**. Never auto-resolve unknown kinds.

## 8. Prompt composer

### 8.1 Fields

| Control | Behavior |
|---|---|
| Text area | Multiline; Submit on Ctrl/Cmd+Enter; Enter may insert newline |
| Send | Invokes `tracer_session_submit_prompt` |
| Attachments | Stub disabled or hidden in MVP unless contract attachments supported end-to-end |
| Helper text | Explains why disabled (see matrix) |

### 8.2 Enabled when

Session status is `ready` (Tauri contract precondition). Optionally allow queue-later UX only if control plane supports it — **MVP does not** invent client-side prompt queues.

Disabled when:

- `creating`, `starting_runtime`, `running` (until ready for next prompt per product rule: if status returns to `ready` after a run, enable again), `awaiting_approval`, `cancelling`, `completed` (if terminal), `failed`, `disconnected`, `stopped`
- Auth gate incomplete
- Runtime pill indicates unavailable / crashed
- Command would return `RuntimeNotReady` / `RuntimeDisconnected`

**Note:** Vertical slice allows multiple prompts over a session when status returns to `ready` after `completed` of a run or equivalent — bind strictly to control-plane status, not local timers.

### 8.3 Submit UX

1. Disable send; optimistic user bubble optional.
2. Wait for `{ accepted: true, promptId, agentRunId }`.
3. On error: toast/banner with `errorClass` + message; remove optimistic bubble or mark failed.
4. Stream content only from events, never from command result body.

## 9. Cancel and stop

| Control | Command | User expectation |
|---|---|---|
| Cancel | `tracer_session_cancel` | Stop active run cooperatively if capable; status → `cancelling` → `stopped` or back to `ready` per backend |
| Stop | `tracer_session_stop` | Tear down runtime for session; no orphans; terminal `stopped` |

Show mode from result when useful: `cooperative` vs `process_stop`. If `CapabilityUnsupported` for cooperative cancel, UI may immediately emphasize Stop or show “Force stopping runtime…”.

Never claim success without terminal events / session get confirmation.

## 10. Footer

Compact metadata row:

```text
acp-stdio · streaming · tools · approvals · cancel  |  seq 42  |  last error: —
```

- Capability chips only for **true** negotiated flags (short names).
- Sequence = `latestSequence` from events list/stream (debug confidence).
- Clicking footer may open Runtime tab.

## 11. Accessibility requirements (screen-level)

| Requirement | Implementation note |
|---|---|
| Status not color-only | StatusChip + RuntimePill always include text |
| Live updates | `aria-live="polite"` for status changes; `assertive` for approval interrupt and disconnect |
| Focus | Approval card container on request; restore focus to composer when returning to `ready` |
| Keyboard | Tab order: Back → Cancel/Stop → Timeline → Side tabs → Composer → Send |
| Contrast | Danger/warning text meets WCAG AA against surface |
| Reduced motion | Spinners have text equivalent (“Starting…”) |
| Screen readers | Tool status and approval decisions announced as text, not icon-only |

## 12. What this screen must not do

1. Parse or require Grok-specific event shapes or `x.ai/*` methods.
2. Auto-approve tools or unknown permission kinds.
3. Show “Running” after process exit.
4. Embed a full code editor or project file tree as the primary surface.
5. Spawn runtimes or shells from the frontend.
6. Present synthetic demos as live authenticated model evidence without labeling.

## 13. Data wiring checklist (implementers)

| Concern | API |
|---|---|
| Initial session | `tracer_session_get` |
| Subscribe | `tracer_session_subscribe` → `tracer://events` |
| History | `tracer_events_list` |
| Prompt | `tracer_session_submit_prompt` |
| Cancel / stop | `tracer_session_cancel` / `tracer_session_stop` |
| Approvals | `tracer_approval_list_pending` / `tracer_approval_resolve` |
| Runtime | `tracer_runtime_status` |

All UI state machines in `STATE_MATRIX.md` and flows in `INTERACTION_FLOW.md` bind to this screen.

---

**Document control:** W0-C deliverable for session workspace implementation.
