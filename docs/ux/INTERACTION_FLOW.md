# Tracer MVP Interaction Flows

**Status:** Wave 0 UX freeze candidate (Stage 0.2)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-product-ux` (W0-C)  
**Companion specs:** `INFORMATION_ARCHITECTURE.md`, `SESSION_SCREEN_SPEC.md`, `STATE_MATRIX.md`

## 1. Purpose

Define **end-to-end user journeys** for the vertical slice, including happy path, auth, approvals, changes review, cancel/stop, crash/disconnect, and capability-limited runtimes.

All flows consume **Tauri commands** and **normalized Tracer events** only. React does not speak ACP.

## 2. Conventions

| Notation | Meaning |
|---|---|
| `→` | User or system step |
| `invoke X` | Tauri command |
| `event T` | Normalized event type |
| **UI:** | Visible outcome |
| **Fail closed** | Deny/block by default when uncertain |

Session statuses are exactly the W0-A set.

---

## 3. Flow A — First-time happy path

**Goal:** Open repo → session → prompt → stream → complete → review.

```text
1. User opens Tracer
   invoke tracer_project_list
   UI: Projects home (empty or populated)

2. User registers a project
   optional invoke tracer_app_open_path_dialog
   invoke tracer_project_register { rootPath, name? }
   UI: Project appears (status ready)

3. User opens project → Create session
   invoke tracer_session_create { projectId, runtime… }
   UI: navigate Session workspace
   StatusChip: Creating session → Starting runtime

4. Control plane spawns runtime + initialize + caps
   events: session.created, runtime.process.started,
           runtime.process.ready, session.status.changed,
           session.ready
   UI: RuntimePill ready; StatusChip Ready; composer enabled

5. User submits prompt
   invoke tracer_session_submit_prompt { sessionId, text }
   UI: optimistic user bubble optional; composer disabled while running
   events: session.prompt.submitted, session.status.changed → running,
           agent.message.delta*, agent.message.completed,
           optional tool.*, agent.plan.updated, file.*
   events: session.completed and/or status → ready|completed

6. User reviews Changes / Plan tabs
   UI: file list and plan steps from events only

7. User stops or leaves
   invoke tracer_session_stop (if desired)
   events: runtime.process.exited expected=true
   UI: Stopped / Completed honest labels

8. User reopens session later
   invoke tracer_session_get + tracer_events_list
   UI: full timeline replay by sequence; no live process required for read-only history
```

**Acceptance feel:** User always knows whether the agent is starting, working, done, or dead.

---

## 4. Flow B — Authenticated vs unauthenticated runtime

### B1 — Unauthenticated stock runtime (Gate 0.1 critical)

```text
1. User creates session (same as A.3)
2. Process may start; initialize may succeed
   events: runtime.process.started, possibly runtime.process.ready
3. Session path requires authenticate before usable session/new
   UI: RuntimePill may show ready/starting; Auth banner "Sign in required"
       Composer disabled with reason "Sign in required"
       Status must NOT look like prompt-ready Ready without auth complete
4. User selects auth method → Continue
   Control plane performs authenticate (adapter-owned wire)
   UI: "Signing in…"
5a. Success → session ready events → Flow A from step 5
5b. Failure → Auth failed banner + retry; still no composer
6. User may Stop to tear down process
```

### B2 — Authenticated path

```text
Credentials already available to runtime (e.g. env/keychain via control plane)
→ session reaches ready without AuthSetupPanel
→ normal prompting
```

### B3 — Auth expired mid-run (if observed)

```text
UI: warning banner re-auth; composer disabled; cancel/stop available
Do not continue to show successful streaming as if authorized if control plane marks failure
```

---

## 5. Flow C — Runtime unavailable / missing executable

```text
1. User creates session
2. invoke tracer_session_create fails OR events runtime.process.failed
   errorClass: RuntimeExecutableNotFound | RuntimeSpawnFailed
3. UI:
   - StatusChip Failed
   - Banner with errorClass message
   - CTA: configure runtime installation / open docs
   - No fake “connected” state
4. User fixes config → Create new session (retry)
```

---

## 6. Flow D — Process started, session creation failed (non-auth)

```text
1. runtime.process.started (± ready)
2. createSession / session.ready never achieved
   session.failed or status failed + lastError
3. UI dual status:
   - RuntimePill: up/ready/failed as accurate
   - Session: Failed with message
   - Banner: "Runtime is running, but the session could not be created."
4. Actions: Stop (cleanup), Retry create, open Diagnostics
5. Forbidden: enable composer
```

---

## 7. Flow E — Capability-missing progressive UI

```text
1. runtime.process.ready { capabilities }
2. UI configures chrome:
   - planUpdates=false → Plan empty reason
   - fileChangeNotifications=false → Changes empty reason
   - cancellation=false → Cancel helper warns process may stop
   - promptStreaming=false → wait for agent.message.completed only
3. If CapabilityMismatch hard fail:
   - session failed banner; no ready composer
```

Vendor-only features from adapter metadata remain hidden unless later contracted.

---

## 8. Flow F — Prompt with tools (no approval)

```text
ready → submit prompt → running
event tool.started → timeline tool card Running
event tool.updated* → progress
event tool.completed → card Completed
agent messages interleave by sequence
→ completed / ready
```

UI never invents tool cards without events.

---

## 9. Flow G — Approval interrupt (fail closed)

**Maps only to** `approval.requested` / `approval.resolved` and approval commands.

```text
1. During running, event approval.requested
2. session.status.changed → awaiting_approval
3. UI:
   - StatusChip: Waiting for approval
   - Interrupt card above composer (focus container, not Allow)
   - Approvals tab badge++
   - Composer disabled
4. User decides:
   a. Allow → invoke tracer_approval_resolve { decision: "allow" }
   b. Deny  → decision "deny"
   c. Cancel request → decision "cancel"
   d. Cancel run → tracer_session_cancel (broader)
5. event approval.resolved
6. Session leaves awaiting_approval (typically running or terminal)
7. Tool continues or fails per backend events (tool.completed / tool.failed)
```

### G-rules

| Rule | Detail |
|---|---|
| Fail closed | Unknown risk → no auto-allow; timeout does not allow |
| No parallel names | Do not introduce `permission.*` UI event types |
| Policy deny | `PermissionDenied` on allow → show why; remains honest |
| Queue | Multiple pending → list in tab; interrupt shows oldest |
| Ignore | User can cancel run; never implicit allow |

---

## 10. Flow H — Changed files review

```text
1. Events file.changed / file.diff.available arrive (any time after tools/edits)
2. Changes tab lists paths (repo-relative) with kind chips
3. User opens entry:
   - small unifiedDiff inline, or
   - diffId referenced view
4. Empty states:
   - capability false → explain runtime does not report changes
   - capability true, no events → "No file changes reported yet"
5. Out of scope: git commit/push UI, multi-file editor
```

---

## 11. Flow I — Cancel active run

```text
1. User clicks Cancel while running or awaiting_approval
2. invoke tracer_session_cancel { scope: active_run }
3. UI optimistically may show Cancelling but reconciles to events
4. events: session.status.changed → cancelling
5a. Cooperative success:
    session.cancelled → stopped or ready (per control plane)
5b. CapabilityUnsupported / timeout:
    UI offers or proceeds to process stop path
    events: runtime.process.exited, session.cancelled / failed / disconnected
6. Tools show cancelled/failed honestly
7. Never silent success
```

---

## 12. Flow J — Stop session / teardown

```text
1. User clicks Stop
2. invoke tracer_session_stop { force? }
3. UI: Stopping… (text)
4. events: cancel path if needed → runtime.process.exited expected=true
5. StatusChip: Stopped
6. Composer disabled; history remains readable
```

---

## 13. Flow K — Runtime crash / disconnect mid-prompt

```text
1. While running: process dies
2. events: runtime.process.exited expected=false and/or runtime.process.failed
          session.status.changed → disconnected|failed
          open tools → failed/cancelled events if emitted
3. UI immediate:
   - Remove any Running presentation
   - Disconnect/crash banner
   - Composer disabled
   - Diagnostics show exit code/signal when present
4. Subsequent invokes return RuntimeDisconnected / RuntimeCrashed → toasts if user acts
5. CTA: Start new session (do not pretend reconnect to dead stdio process in MVP)
6. History: still available via tracer_events_list for what was persisted
```

**MVP reconnect:** Tracer does not silently respawn into the same live ACP session unless a future resume design lands. User honesty > magic reconnect.

---

## 14. Flow L — Protocol unknown / malformed (process alive)

```text
event adapter.protocol.unknown → generic timeline card
event adapter.protocol.error → error card
Session may continue (per contract)
UI does not crash; does not parse raw vendor payload for behavior
Optional adapter metadata expander for debug
```

---

## 15. Flow M — Storage failure

```text
event storage.error OR command StorageError
UI: persistent warning banner
Do not show "Saved" / do not claim reload will work
User may continue viewing in-memory stream if still connected, with caveat
```

---

## 16. Flow N — Multi-turn session (when status returns to ready)

```text
completed run → status ready (if control plane supports)
composer re-enabled
user submits another prompt
timeline continues with rising sequence
stop ends process when user done
```

If `completed` is terminal for the whole session, CTA is “New session” only — **bind to actual status**, not this document’s preference.

---

## 17. Flow O — App restart / resume history

```text
1. User restarts Tracer
2. Opens project → prior session
3. invoke tracer_session_get + tracer_events_list { afterSequence: 0 }
4. UI rebuilds timeline by sequence
5. If session terminal: read-only history
6. If product supports resume with sessionResume capability:
   only then show Resume runtime control (future); MVP may always require new runtime binding
```

---

## 18. Flow P — Leave session while active

```text
1. User hits Back while running or awaiting_approval
2. UI modal:
   "This session is still active."
   [ Stay ] [ Cancel run and leave ] [ Leave without stopping ]
3. Default recommendation: Stay (or Cancel run and leave) — avoid orphan confusion
4. If leave without stopping: project list shows session still running (status chip)
```

Exact process policy is control-plane owned; UX must warn.

---

## 19. Sequence diagrams (logical)

### 19.1 Happy path

```text
User          UI              Control plane         Runtime
 |             |                    |                  |
 |--open proj->|                    |                  |
 |--create ses->|--session_create-->|                  |
 |             |                    |--spawn---------->|
 |             |                    |--initialize----->|
 |             |<--events ready-----|<--init result----|
 |--prompt---->|--submit_prompt---->|--session/prompt->|
 |             |<--deltas-----------|<--session/update-|
 |             |<--completed--------|<--prompt result--|
```

### 19.2 Approval

```text
Runtime --request_permission--> Adapter --> event approval.requested --> UI interrupt
User Allow --> tracer_approval_resolve --> Adapter decision --> Runtime continues
             --> event approval.resolved
```

### 19.3 Crash

```text
Runtime dies --> process manager --> runtime.process.exited
             --> session disconnected/failed --> UI banner, not Running
```

---

## 20. Copy catalog (selected)

| Situation | Title | Body / helper |
|---|---|---|
| Empty projects | No projects yet | Open a local repository to begin. |
| Empty sessions | No sessions | Create a session to start an agent on this project. |
| Ready empty timeline | Session ready | Send a prompt to begin. |
| Starting runtime | Starting runtime | Waiting for the agent process to initialize… |
| Auth required | Sign in required | This runtime needs authentication before a session can accept prompts. |
| Auth failed | Sign-in failed | {message} Retry or choose another method. |
| Awaiting approval | Approval needed | The agent requested permission. Review details before allowing. |
| Unknown risk | Unknown risk | Review carefully. Tracer will not allow this automatically. |
| Cancelling | Cancelling | Stopping the current agent run… |
| Disconnected | Runtime disconnected | The agent process exited. Prompting is disabled. |
| Crashed | Runtime crashed | The agent stopped unexpectedly. See diagnostics for exit details. |
| Process up, session fail | Session could not be created | The runtime process is running, but session setup failed. |
| Capability plan | Plan unavailable | This runtime did not advertise plan updates. |
| Capability files | File changes unavailable | This runtime does not report file changes to Tracer. |
| Cancel no coop | Cancel may stop runtime | Cooperative cancel is unavailable; Tracer may stop the process. |
| Storage error | Could not save | Session data may not reload after restart. |
| Synthetic demo | Demo runtime | Not live model output. |
| Generic unknown event | Unrecognized event | {type} — open details for a safe summary. |

---

## 21. Accessibility flow requirements

| Moment | a11y behavior |
|---|---|
| Status change to blocking | Announce with live region (polite or assertive for disconnect/approval) |
| Approval appears | Move focus to interrupt container; Escape focuses Cancel run only if documented — prefer explicit buttons |
| Approval resolves to ready | Return focus to composer |
| Crash | Assertive announcement of disconnected/crashed label |
| Color | All status chips include text |

---

## 22. Synthetic evidence limitation (product + demos)

| Allowed | Not allowed |
|---|---|
| Fake runtime demos clearly labeled | Claiming synthetic fixtures are live Grok captures |
| Scrubbed live initialize fixtures in tests | Shipping private prompts/tokens in UI samples |
| UI storybook with mock envelopes | Teaching implementers to parse `x.ai/*` in React |

Gate 0.1: authenticated multi-turn tool/permission live parity was **not** fully verified; UX must not over-claim runtime intelligence or vendor feature completeness.

---

## 23. Out-of-scope flows (explicit non-goals)

- Full IDE edit-debug-commit loops
- Multi-agent ALMS orchestration UI
- Leader-mode multi-window Grok sharing
- Auto-approve / yolo as default product mode
- Cloud workspace onboarding
- Vendor subagent graph exploration as MVP navigation

---

## 24. Implementer checklist

- [ ] All primary flows use only contracted command names
- [ ] Status labels match STATE_MATRIX
- [ ] Approvals only via `approval.*` + approval commands
- [ ] Auth gate is not a generic error
- [ ] Crash never leaves Running
- [ ] Capability absence degrades UI, does not invent data
- [ ] No raw ACP in UI
- [ ] No full-IDE IA expansion

---

**Document control:** W0-C deliverable. Update with formal contract changes only.
