# Tracer MVP Information Architecture

**Status:** Wave 0 UX freeze candidate (Stage 0.2)  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-product-ux` (W0-C)  
**Write boundary:** `docs/ux/` only  
**Normative backends:** `docs/contracts/*`, `docs/architecture/TRACER_VERTICAL_SLICE.md`, Gate 0.1 integration report

## 1. Purpose

This document freezes the **MVP screen hierarchy**, **navigation model**, and **information ownership** for the Tracer vertical slice.

Goals:

1. Map every first-slice backend concern to a visible product surface.
2. Keep the product a **desktop control plane for AI coding agents**, not a full IDE.
3. Bind UI labels and routes to **W0-A session statuses** and **normalized Tracer events**.
4. Keep React free of raw ACP / vendor-specific frames (`ADR-002`).

Non-goals for MVP / Gate 1:

- Multi-file code editor, file tree as primary workspace, Monaco/full IDE chrome
- Cloud multi-tenant, collaboration, remote workspaces
- Vendor-rich Grok panels driven by `x.ai/*` as core product paths
- Brand redesign beyond functional layout and status language

## 2. Product metaphor

Tracer is a **session workspace** over a **local project**, backed by a **managed runtime sidecar**.

| Layer | User-facing meaning |
|---|---|
| **App shell** | Trusted local control plane chrome (nav, global banners, settings stub) |
| **Project** | Registered local folder Tracer manages sessions against |
| **Session** | Tracer-owned work unit: prompts, timeline, tools, approvals, status |
| **Runtime** | Sidecar process + negotiated capabilities (not a user-editable “server” UI) |
| **Timeline** | Ordered stream of normalized events (`tracer://events` + `tracer_events_list`) |
| **Approvals** | Fail-closed permission interrupts (`approval.requested` / `approval.resolved`) |
| **Changes** | File change / diff summaries when events exist (optional empty when capability absent) |

## 3. Screen hierarchy (MVP)

```text
AppShell
├── GlobalStatusRegion          # app-wide banners (runtime unavailable, storage error, auth)
├── PrimaryNav
│   ├── Projects (home)
│   ├── Active session (when open)
│   └── App / About (minimal)
│
├── ProjectsHome
│   ├── EmptyProjectsState
│   ├── ProjectList
│   └── RegisterProject flow (dialog / full-page)
│
├── ProjectWorkspace
│   ├── ProjectHeader (name, path status, open/missing)
│   ├── SessionList
│   ├── EmptySessionsState
│   └── CreateSession control
│
└── SessionWorkspace            # primary vertical-slice screen
    ├── SessionHeader (status, runtime health, cancel/stop)
    ├── AuthSetupPanel          # only when runtime auth gate blocks session readiness
    ├── CapabilityBanner        # missing/optional caps that change UI affordances
    ├── PromptComposer
    ├── MainSplit
    │   ├── TimelinePane        # messages, tools, plan, protocol notices
    │   └── SidePane (tabs)
    │       ├── Plan
    │       ├── Approvals
    │       ├── Changes
    │       └── Runtime / Diagnostics (collapsed by default)
    └── SessionFooter (capabilities summary, last error, sequence health)
```

### 3.1 Route sketch (logical, not framework-locked)

| Route key | Screen | Entry |
|---|---|---|
| `projects` | Projects home | App start / nav |
| `projects/:projectId` | Project workspace + session list | Open project |
| `projects/:projectId/sessions/:sessionId` | Session workspace | Create or open session |
| `about` | App info (`tracer_app_info`) | Secondary |

Deep linking beyond these is out of MVP scope. No multi-window IDE layout required for Gate 1.

## 4. Primary surfaces and ownership

### 4.1 Projects home

**Purpose:** Register and open local repositories.

| UI element | Data source | Notes |
|---|---|---|
| Project list | `tracer_project_list` | Status: `ready` \| `missing` \| `invalid` |
| Register | `tracer_project_register` (+ optional path dialog) | Absolute path is **user-local runtime data** only |
| Empty state | Local UI | CTA: “Open a local repository” |
| Missing path | Project `status` | Show recovery: re-locate / remove from Tracer (history optional) |

**Must not:** browse remote git hosts, clone, or act as a full SCM client.

### 4.2 Project workspace

**Purpose:** Choose or create a session bound to one project.

| UI element | Data source |
|---|---|
| Session list | `tracer_session_list` |
| Create session | `tracer_session_create` |
| Session row status chip | Session `status` from W0-A catalog |

Creating a session starts the vertical-slice runtime path (combined create+start allowed by Tauri contract). UI immediately enters **Session workspace** with status `creating` / `starting_runtime`.

### 4.3 Session workspace (core)

**Purpose:** Prove the vertical slice loop: prompt → stream → tools/approvals → status honesty → stop/reload.

Owns:

- Prompt submit (`tracer_session_submit_prompt`) only when status allows
- Live timeline via `tracer://events` (+ reload via `tracer_events_list`)
- Approval resolve (`tracer_approval_resolve`)
- Cancel / stop (`tracer_session_cancel`, `tracer_session_stop`)
- Runtime diagnostic readout (`tracer_runtime_status`) as secondary info

Does **not** own:

- Raw ACP method dispatch
- Parsing Grok `x.ai/*` payloads for primary behavior
- Full editor, multi-file tabs, integrated terminal product (terminal **events** may appear as timeline cards if capability present)

## 5. Information architecture principles

### 5.1 Backend status is authoritative

Session status comes from control plane (`session.status.changed`, `tracer_session_get`). Optimistic UI may show “Cancelling…” after a cancel click, but **must reconcile** to event/query truth within a short bound. Never leave “Running” after `runtime.process.exited`.

### 5.2 Normalized events only

UI binds to Tracer Event Protocol v1 `type` strings, for example:

| Product region | Primary event types |
|---|---|
| Runtime health | `runtime.process.started`, `ready`, `stderr`, `exited`, `failed` |
| Session lifecycle | `session.created`, `ready`, `prompt.submitted`, `status.changed`, `completed`, `failed`, `cancelled` |
| Messages / plan | `agent.message.delta`, `agent.message.completed`, `agent.progress.delta`, `agent.plan.updated` |
| Tools | `tool.started`, `tool.updated`, `tool.completed`, `tool.failed` |
| Approvals | `approval.requested`, `approval.resolved` |
| Changes | `file.changed`, `file.diff.available` |
| Terminal (optional) | `terminal.output`, `terminal.exited` |
| Errors / unknown | `storage.error`, `adapter.protocol.error`, `adapter.protocol.unknown` |

Unknown types render as **generic timeline entries** (label = `type`, expandable safe payload). Vendor/raw data under `adapter` metadata is **debug-only**, never required for core affordances.

### 5.3 Auth is a first-class product state

Per Gate 0.1 / W0-B: process can initialize **without** credentials; `session/new` may fail with authentication required. UX must model:

| Product state | Meaning |
|---|---|
| **Runtime process up, unauthenticated** | Initialize/capabilities may exist; prompts not available |
| **Auth required** | User must complete advertised auth method path before session becomes `ready` |
| **Auth failed** | Explicit failure with retry; not a generic “session error” only |
| **Authenticated / session ready** | Prompt composer enabled (when status `ready`) |

Do not collapse auth failures into undifferentiated `failed` copy without an auth-specific explanation when `errorClass` / payload indicates authentication.

### 5.4 Capabilities drive progressive disclosure

Negotiated Tracer capabilities (`runtime.process.ready` payload) control panels:

| Capability false / missing | UI behavior |
|---|---|
| `planUpdates` | Hide Plan tab / show disabled empty with reason |
| `toolCalls` | Tools appear only if events arrive; no fake tools |
| `approvals` | Approvals tab idle; fail-closed if policy requires approval for risky tools |
| `cancellation` | Cancel still offered; explain force-stop fallback |
| `fileChangeNotifications` | Changes tab empty with “runtime does not report file changes” |
| `terminalOutput` | No Terminal sub-panel; do not invent PTY UI |
| `promptStreaming` | Accept final-only messages; still show completed assistant message |
| `sessionResume` | Hide “resume runtime session” affordances if false |

### 5.5 Fail closed on approvals

Approvals map **only** to:

- events: `approval.requested` / `approval.resolved`
- commands: `tracer_approval_list_pending` / `tracer_approval_resolve`

Unknown risk → user must decide; **never** auto-allow. Deny/cancel are first-class.

### 5.6 No full-IDE expansion

MVP layout is **session-centric**:

- One primary timeline column
- One secondary side pane (tabs)
- No multi-editor grid, activity bar clone, or plugin marketplace

Changed files are **review cards**, not an editor workspace. Optional later Wave 2 features may deepen diff viewing without rewriting IA.

## 6. Global regions

### 6.1 Global status region (app shell)

Sticky top (or below title bar) for cross-cutting conditions:

| Condition | Severity | Example copy |
|---|---|---|
| Runtime executable missing | error | “Agent runtime not found. Install or configure the ACP runtime.” |
| Storage error | error | “Could not save session data. History may be incomplete.” |
| App offline from own backend (invoke failures) | error | “Tracer control plane is not responding.” |
| Authenticated runtime unavailable for new sessions | warning | “No ready runtime installation.” |

Always pair color with **icon + text**. See accessibility in `STATE_MATRIX.md`.

### 6.2 Session-scoped banners (session workspace)

| Condition | Placement |
|---|---|
| `starting_runtime` | Session header + timeline skeleton |
| Auth required | Banner + AuthSetupPanel |
| Capability missing that blocks action | Inline near disabled control |
| `awaiting_approval` | High-priority interrupt card + Approvals tab badge |
| `disconnected` / crash | Full-width error banner; disable prompt |
| `cancelling` | Status chip + disable prompt/approve allow |
| Synthetic / demo fixture mode (dev only) | Subtle “Fixture / fake runtime” badge — never imply live model output for synthetic streams |

## 7. Navigation rules

1. **Projects → Project → Session** is the primary path.
2. Leaving a live session does not auto-stop the runtime unless product policy later says so; MVP may keep process bound to session until Stop (document in UI: “Session still running in background” if nav allows). Preferred MVP: warn on leave while `running` / `awaiting_approval`.
3. Browser-style history optional; in-app back to session list required.
4. Deep links into Approvals/Changes open the same Session workspace with tab selection — not separate full pages.

## 8. Content inventory (MVP)

| Content type | Where shown | Source of truth |
|---|---|---|
| Project name / path status | Projects, Project header | Project commands |
| Session title / status | Session list, Session header | Session commands + status events |
| User prompts | Timeline (user role bubbles) | `session.prompt.submitted` (+ local optimistic text) |
| Assistant text | Timeline | `agent.message.*` |
| Progress | Timeline or header subline | `agent.progress.delta` |
| Plan steps | Plan tab + optional timeline summary | `agent.plan.updated` |
| Tool calls | Timeline tool cards | `tool.*` |
| Approvals | Interrupt card + Approvals tab | `approval.*` + list pending |
| File changes / diffs | Changes tab | `file.changed`, `file.diff.available` |
| Stderr / protocol errors | Diagnostics + timeline error cards | `runtime.process.stderr`, `adapter.protocol.*` |
| Runtime capabilities | Footer / Diagnostics | ready payload / `tracer_runtime_status` |
| Vendor raw metadata | Expandable “Adapter metadata” (advanced) | `adapter` field — optional, never required |

## 9. Empty vs populated information states (summary)

Detailed matrices live in `STATE_MATRIX.md`. IA-level empties:

| Surface | Empty meaning | Primary CTA |
|---|---|---|
| Projects | No registered repos | Register project |
| Sessions | No sessions for project | Create session |
| Timeline | Session created, no prompts yet | Submit prompt (if `ready`) |
| Plan | No plan updates | None (passive) |
| Approvals | No pending approvals | None |
| Changes | No file events | None; explain capability if missing |
| Diagnostics | No stderr/errors | Healthy idle copy |

## 10. Accessibility and inclusion (IA-level)

1. Every status has a **text label** (and preferably icon), not color alone.
2. Live regions announce status transitions that block input (`awaiting_approval`, `disconnected`, `failed`).
3. Focus moves to approval interrupt when it appears (see `SESSION_SCREEN_SPEC.md`).
4. Unknown/vendor content never flashes unreadable binary; show safe text summaries.
5. Keyboard: navigate timeline list, resolve approval, submit prompt, cancel session without mouse.

## 11. Explicit exclusions (scope fence)

Out of MVP IA:

- Multi-root workspaces as first-class equal projects (one project per session is enough)
- Chat-only mode without project binding
- Marketplace of runtimes (simple installation list is enough: `tracer_runtime_describe_installations`)
- Subagent graphs, voice mode, MCP app galleries driven by vendor notifications
- Presenting **synthetic** fixture transcripts as “live Grok” evidence in product copy or demos without labeling

## 12. Mapping to vertical slice acceptance

| Slice step | Primary screen |
|---|---|
| Open local repository | Projects home / register |
| Start ACP runtime | Session workspace (`starting_runtime`) |
| Create session | Session workspace (`creating` → `ready` or auth gate) |
| Submit prompt | Prompt composer + timeline |
| Stream events | Timeline |
| Show changed files + runtime state | Changes tab + header status |
| Persist session | Transparent; reload via `tracer_events_list` on reopen |
| Stop / resume safely | Stop/cancel controls; reopen session history |

## 13. Document dependencies

| Doc | Role |
|---|---|
| `SESSION_SCREEN_SPEC.md` | Layout regions, components, density |
| `STATE_MATRIX.md` | Every backend status → UI treatment |
| `INTERACTION_FLOW.md` | End-to-end user journeys |
| `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md` | Event catalog & statuses |
| `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` | Commands & preconditions |
| `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md` | Capabilities & error classes |
| `docs/integration/STAGE_0_1_INTEGRATION_REPORT.md` | Auth gate, synthetic limits, risks for UX |

---

**Document control:** W0-C deliverable. Implementation agents (W1-A / feature modules) must not invent parallel primary navigation that expands into full-IDE scope without a contract and UX revision.
