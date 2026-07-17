# ACP Event Mapping (Grok Build → Tracer)

**Task:** W0-B  
**Purpose:** Map stock Grok Build ACP wire events to Tracer-normalized concepts for the runtime adapter (W1).  
**Upstream ACP crate:** `agent-client-protocol` 0.10.4  
**Protocol version observed:** `1`

This document distinguishes:

1. **Standard ACP** — portable across ACP runtimes  
2. **Vendor (`x.ai/*`)** — Grok/xAI-specific; optional in adapter  

### Normative Tracer event names (Stage 0.1 reconciliation)

Mapping tables in this document use **conceptual** Tracer names for readability (`runtime.initialized`, `message.agent.delta`, `permission.requested`, `turn.started`, etc.).

**Authoritative product event `type` strings** are defined only in:

- `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`

Wave 1 implementers and contract tests MUST emit and assert W0-A catalog names. Example alignments:

| Wire / concept here | Normative W0-A `type` (examples) |
|---|---|
| Successful initialize + caps | `runtime.process.ready` (capabilities in payload) |
| `agent_message_chunk` | `agent.message.delta` |
| `session/request_permission` | `approval.requested` / `approval.resolved` |
| Prompt submit / stream end | `session.prompt.submitted` + agent/session completion events per W0-A |
| Unknown vendor update | `adapter.protocol.unknown` |

Do not introduce parallel product type strings from this research doc without a formal contract revision.

## 1. Wire transport

| Property | Value |
|---|---|
| Framing | Newline-delimited JSON-RPC 2.0 |
| Direction | Client writes requests/notifications on **agent stdin**; agent writes responses/notifications on **stdout** |
| Stderr | Logs only — never mix JSON-RPC |
| Normalization | Grok stdin reader may re-serialize rare escaped-slash method envelopes (ACP 0.6 workaround) |

## 2. Core lifecycle methods → Tracer events

| ACP method | Role | Suggested Tracer normalized event / action |
|---|---|---|
| `initialize` | Capability negotiation | `runtime.initialized` + store capability snapshot |
| `authenticate` | Auth handshake | `runtime.authenticated` / `runtime.auth_required` |
| `session/new` | Create session | `session.created` |
| `session/load` | Resume session | `session.loaded` / `session.resumed` |
| `session/prompt` (request) | Start turn | `turn.started` |
| `session/prompt` (result) | End turn | `turn.completed` + `stop_reason` |
| `session/cancel` | Cancel turn | `turn.cancelled` |
| `session/set_mode` | Mode switch | `session.mode_changed` |
| `session/set_model` | Model switch | `session.model_changed` |
| `session/request_permission` | Permission gate | `permission.requested` → UI → `permission.resolved` |
| Error responses | Failures | `runtime.error` / `session.error` with JSON-RPC code/message/data |

## 3. Standard `session/update` → Tracer stream events

ACP notification:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "<id>",
    "update": { "sessionUpdate": "<type>", "...": "..." }
  }
}
```

| `sessionUpdate` (ACP) | Payload highlights | Tracer mapping |
|---|---|---|
| `agent_message_chunk` | `content` text block | `message.agent.delta` |
| `agent_thought_chunk` | thought text | `message.thought.delta` (collapsible) |
| `user_message_chunk` | user text (rare stream) | `message.user.delta` |
| `tool_call` | `toolCallId`, `title`, `kind`, `status`, locations, content | `tool.started` / `tool.updated` |
| `tool_call_update` | status/content patches | `tool.updated` / `tool.completed` / `tool.failed` |
| `plan` | plan entries | `plan.updated` |
| `available_commands_update` | slash commands | `commands.updated` |
| `current_mode_update` | mode id | `session.mode_changed` |

### Tool kinds (ACP `ToolKind`)

Map to Tracer tool categories without dropping unknown kinds:

| ACP kind (typical) | Tracer category |
|---|---|
| `read` | Read |
| `edit` | Edit (surface diffs) |
| `delete` | Delete |
| `move` | Move |
| `search` | Search |
| `execute` | Shell/execute |
| `think` | Internal |
| `fetch` | Network |
| other / unknown | `other` + raw kind string |

### Tool statuses

| ACP status | Tracer |
|---|---|
| `pending` | Pending |
| `in_progress` | Running |
| `completed` | Completed |
| `failed` | Failed |

### Diff content

When `tool_call` / update includes `ToolCallContent::Diff`, map to Tracer `file.diff` / changed-files panel. Prefer structured old/new text when present; never assume absolute machine paths in fixtures.

## 4. Permission reverse-request mapping

| Wire | Tracer |
|---|---|
| Request `session/request_permission` | Open modal / permission card |
| Options `AllowOnce` / `AllowAlways` / `RejectOnce` / … | Normalize to `allow_once`, `allow_always`, `reject_once`, `cancel` |
| Response selected `optionId` | Resume runtime; log decision |
| Outcome `cancelled` | Treat as user cancel |

Vendor pending markers:

| Vendor update | Tracer |
|---|---|
| `pending_interaction` (`kind=permission\|question\|plan_approval`) | `interaction.pending` |
| `interaction_resolved` | `interaction.resolved` |

## 5. Stop reasons

| ACP / observed stop | Tracer `turn.stop_reason` |
|---|---|
| `end_turn` | `end_turn` |
| `cancelled` | `cancelled` |
| `refusal` | `refusal` (content filter / safety) |
| max tokens / truncated | `max_tokens` |
| max turns | `max_turns` |
| error path (JSON-RPC error instead of result) | `error` |

## 6. Vendor notifications → optional Tracer channels

Method: `x.ai/session_notification` (primary) with body shaped like:

```json
{
  "sessionId": "<id>",
  "update": { "sessionUpdate": "<vendor_type>", "...": "..." }
}
```

| Vendor `sessionUpdate` | Meaning | Tracer mapping recommendation |
|---|---|---|
| `diff_review` | Multi-file review request | `review.diff_requested` |
| `retry_state` | Transient retry | `runtime.retry` |
| `auto_compact_started` / `completed` / `failed` / `cancelled` | Context compaction | `context.compact.*` |
| `memory_flush_*` / `memory_dream_completed` / `memory_session_saved` | Memory subsystem | Optional / ignore in MVP |
| `auto_recovery_started` / `exhausted` | Recovery | `runtime.recovery.*` |
| `hook_annotation` / `hook_execution` / `hooks_changed` | Hooks | Optional |
| `plugins_changed` / `plugin_updates_installed` | Plugins | Optional |
| `session_summary_generated` | Title | `session.title` |
| `session_recap` / `session_recap_unavailable` | Recap | Optional |
| `task_completed` / `task_backgrounded` | Background tasks | `task.*` |
| `subagent_spawned` / `progress` / `finished` | Subagents | `subagent.*` (later wave) |
| `scheduled_task_*` / `monitor_event` | Scheduler/monitors | Later |
| `model_auto_switched` / `model_changed` | Model changes | `session.model_changed` |
| `tool_call_delta_chunk` | Streaming tool args | `tool.args.delta` |
| `goal_updated` | Autonomous goal | Later |
| `turn_completed` | Vendor turn end marker | Correlate with prompt result |
| `pending_interaction` / `interaction_resolved` | HITL park | See §4 |
| `git_branch_update` | Branch tip | Optional status bar |
| unknown future tags | Forward-compat | `vendor.unknown` keep raw |

Other vendor methods:

| Method | Tracer handling |
|---|---|
| `_x.ai/mcp/servers_updated` | `mcp.servers_updated` (note leading underscore observed live) |
| `x.ai/fs_notify` / `x.ai/fs/index*` | Optional file index feed |
| `x.ai/leader_reconnected` | `runtime.leader_reconnected` — rebind session if needed |
| `x.ai/search/fuzzy/status` | Ignore unless UI search uses Grok fuzzy |

## 7. Vendor extension requests (client-initiated)

Adapter should expose a small **required** set and a generic `ext_request` escape hatch.

### MVP-useful

| Method | Use |
|---|---|
| `x.ai/session/info` | Inspect session |
| `x.ai/session/close` | Graceful session close |
| `x.ai/session/list` / `x.ai/sessions/list` | Session picker |
| `x.ai/auth/*` | Login URL / code submit |
| `x.ai/commands/list` | Refresh slash commands |

### Explicitly non-MVP (do not block vertical slice)

Git, worktree, marketplace, skills, billing, cloud env, code-nav, heap/debug, PR, etc.

Unknown methods return ACP `method_not_found` — adapter must not crash.

## 8. Prompt `_meta` mapping

| Grok `_meta` | Tracer |
|---|---|
| `promptId` | Stable turn id (generate if absent) |
| `mode` | Prompt mode (`agent` / `plan` / …) |
| `sendNow` | Bypass queue |
| `verbatim` | Skip expansions |
| usage fields on response | `turn.usage` |

## 9. Session `_meta` mapping (`session/new`)

| Field | Tracer |
|---|---|
| `sessionId` | Allow Tracer-owned UUID injection |
| `modelId` | Initial model |
| `yoloMode` / auto mode | Permission policy seed |
| `rules` / `systemPromptOverride` / `agentProfile` | Advanced; optional |
| `x.ai/mcp/servers` | MCP attach list |

## 10. Normalization rules for the adapter

1. **Never leak raw ACP into React.** Control plane normalizes first (W0-A / W1).  
2. **Preserve `sessionId` and `toolCallId`** for correlation.  
3. **Buffer text chunks** for UI efficiency but keep replay fidelity for persistence.  
4. **Vendor events are optional:** MVP works with standard stream only.  
5. **Absolute paths** from runtime should be relativized to project root for UI when possible.  
6. **Auth tokens never enter event logs or fixtures.**  
7. **Unknown `sessionUpdate` tags** → soft-ignore with debug metric, not hard error.  
8. **Permission reverse-requests** must be answered or cancelled; ignoring deadlocks the turn.

## 11. Example end-to-end mapping (happy path)

| Step | Wire | Tracer |
|---|---|---|
| 1 | spawn + `initialize` | `runtime.starting` → `runtime.initialized` |
| 2 | `authenticate` | `runtime.authenticated` |
| 3 | `session/new` | `session.created` |
| 4 | `session/prompt` | `turn.started` |
| 5 | `session/update` agent chunks | `message.agent.delta` (×N) |
| 6 | `session/update` tool_call | `tool.started` |
| 7 | `session/request_permission` | `permission.requested` |
| 8 | permission response | `permission.resolved` |
| 9 | `tool_call_update` completed | `tool.completed` |
| 10 | more agent chunks | `message.agent.delta` |
| 11 | prompt result `end_turn` | `turn.completed` |

## 12. Fixture cross-reference

| Fixture | Content |
|---|---|
| `tests/fixtures/acp/initialize-request.json` | Client initialize |
| `tests/fixtures/acp/initialize-response.json` | Sanitized live response |
| `tests/fixtures/acp/session-new-auth-required.json` | Live unauthenticated error |
| `tests/fixtures/acp/session-prompt-stream.jsonl` | Synthetic stream sequence |
| `tests/fixtures/acp/permission-request.json` | Synthetic permission reverse-request |
| `tests/fixtures/acp/README.md` | Sanitization policy |
