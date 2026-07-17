# Grok Build ACP Process Lifecycle

**Task:** W0-B `tracer-w0-grok-runtime-recon`  
**Primary command:** `grok agent stdio` (recommend `--no-leader` for Tracer-owned child processes)  
**Probed binary:** `grok 0.2.102` on Windows  
**Upstream SOURCE_REV:** `2ec0f0c8488842da03a71eeee3c61154957ca919`

## 1. Lifecycle overview

```text
spawn(grok agent [--no-leader] stdio)
        │
        ▼
 process start → load config → optional model prefetch
        │
        ▼
 stdio bridge live (blocking stdin reader thread)
        │  no separate "ready" event
        ▼
 client → initialize  ─────────────────────────────┐
        │                                          │
        ▼                                          │
 client → authenticate (if required)               │
        │                                          │
        ▼                                          │
 client → session/new | session/load               │
        │                                          │
        ▼                                          │
 loop: session/prompt ↔ session/update*            │
       session/request_permission ↔ response       │
       session/cancel / set_model / set_mode       │
       x.ai/* extensions                           │
        │                                          │
        ▼                                          │
 client closes stdin OR kills process              │
        │                                          │
        ▼                                          │
 agent flushes handlers, closes PTYs, exits ◄──────┘
```

## 2. Exact start command for stock ACP mode

### Product / released binary

```bash
grok agent stdio
```

Recommended Tracer spawn (isolation, no shared leader):

```bash
grok agent --no-leader stdio
```

Optional:

```bash
grok agent --no-leader --model grok-build --always-approve stdio
```

### From source monorepo tree

```bash
cargo run -p xai-grok-pager-bin -- agent --no-leader stdio
# release:
cargo build -p xai-grok-pager-bin --release
# binary path: target/release/xai-grok-pager  (product renames to grok)
```

### Process environment (control plane)

| Variable | Role |
|---|---|
| `GROK_HOME` | Override config/state home (prefer hermetic dirs for tests) |
| `XAI_API_KEY` | API key auth path |
| `GROK_CLIENT_VERSION` | Logged early for diagnostics (desktop sets this) |
| `GROK_AGENT_SECRET` | Serve mode secret |
| `GROK_DEBUG_LOG` / `GROK_LOG_FILE` | Debug logging — must not corrupt stdout JSON-RPC |
| `HOME` / `USERPROFILE` | May influence paths; prefer explicit `GROK_HOME` |

**Spawn rules for Tracer:**

1. Pipe **stdin** and **stdout**; capture **stderr** separately for logs.
2. Set **cwd** to the opened repository root (or explicit project path).
3. Do **not** write non-JSON human banners to stdout on stdio path (stdio suppresses version banner).
4. Prefer `--no-leader` so one Tracer session maps to one agent process.
5. Keep stdin open for the entire session; closing stdin is a shutdown signal.

## 3. Readiness behavior

| Question | Answer |
|---|---|
| Is there a ready byte / banner on stdout? | **No.** Stdio path avoids the version banner printed by other agent modes. |
| When is the process usable? | When it accepts JSON-RPC on stdin and returns `initialize` result. |
| Practical readiness probe | Send `initialize` with timeout (test harness uses ~20s). |
| Failure modes before ready | Missing binary, crash on config load, hung stdin reader (mitigated on Windows by dedicated thread + stdin dup). |

**Tracer should treat readiness as:** child process running **and** successful `initialize` response within N seconds.

## 4. Initialize

### Client → agent

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "clientCapabilities": {
      "fs": { "readTextFile": true, "writeTextFile": true },
      "terminal": true
    },
    "_meta": {
      "clientType": "tracer",
      "clientIdentifier": "tracer",
      "clientVersion": "0.1.0",
      "startupHints": {
        "nonInteractive": true,
        "skipGitStatus": true,
        "skipProjectLayout": true
      }
    }
  }
}
```

### Agent side effects (source)

On `initialize`, Grok:

- Starts subagent coordinator
- Cleans stale worktrees / sessions / permissions (background)
- Bootstraps search index
- Parses client type, code-nav, interactive-trust, MCP apps, buffering settings
- Reloads auth from disk; may silent-refresh tokens
- Builds `authMethods` list
- Spawns managed MCP setup, announcements, heap profile monitor
- Returns `InitializeResponse` with `AgentCapabilities` + `_meta`

### Observed live result fields (sanitized)

See `tests/fixtures/acp/initialize-response.json`.

Key points:

- `protocolVersion: 1`
- `loadSession: true`
- `embeddedContext: true`
- `mcpCapabilities.http/sse: true`
- Vendor `_meta`: `grokShell`, `agentVersion`, `modelState`, `availableCommands`, feature flags

## 5. Authenticate

```text
initialize → authenticate(methodId) → session/new
```

Without authentication, **live probe** returned:

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "error": {
    "code": -32000,
    "message": "Authentication required",
    "data": "no auth method id provided"
  }
}
```

Auth methods are dynamic. Headless/test clients often pick `xai.api_key` with `_meta.headless: true`. Interactive clients use `grok.com` device/login flows (`x.ai/auth/*` extensions).

## 6. Session create / load

### `session/new`

Parameters:

- `cwd` — absolute path of workspace
- `mcpServers` — array (may be empty)
- `_meta` — optional `sessionId` (UUID), `modelId`, `yoloMode`, rules, MCP config, etc.

Behavior highlights:

- Requires prior `initialize`
- Resolves folder trust for `cwd`
- Resolves MCP servers (client + managed)
- Allocates session id (client UUID or UUIDv7)
- Starts session actor, persistence (unless chat-kind), optional relay sync
- Returns `sessionId` (+ modes / model state depending on version)

### `session/load`

- Restores persisted session (updates.jsonl, etc.)
- Reconnect-safe: ongoing work can continue when a client reloads
- Used for resume after Tracer restart if grok home + session id preserved

## 7. Prompt and streaming

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "session/prompt",
  "params": {
    "sessionId": "<uuid>",
    "prompt": [{ "type": "text", "text": "List files in the project root" }],
    "_meta": {
      "promptId": "<optional-uuid>",
      "mode": "agent",
      "sendNow": false,
      "verbatim": false
    }
  }
}
```

### Stream

While the prompt is in flight, agent sends notifications:

```json
{
  "jsonrpc": "2.0",
  "method": "session/update",
  "params": {
    "sessionId": "<uuid>",
    "update": {
      "sessionUpdate": "agent_message_chunk",
      "content": { "type": "text", "text": "…" }
    }
  }
}
```

Common `sessionUpdate` values (standard ACP):

| Value | Meaning |
|---|---|
| `agent_message_chunk` | Assistant text stream |
| `agent_thought_chunk` | Reasoning stream |
| `tool_call` | New tool invocation |
| `tool_call_update` | Tool status/result |
| `plan` | Plan items |
| `available_commands_update` | Slash command list change |
| `current_mode_update` | Mode change |

Vendor-rich updates often arrive as `x.ai/session_notification` with a tagged `sessionUpdate` (subagents, compact, goals, pending_interaction, model_changed, turn_completed, …).

### Response

`session/prompt` completes with a result containing `stopReason` (e.g. `end_turn`, `cancelled`, `refusal`, max-tokens variants). Usage may appear under `_meta`.

Prompt intake is serialized per session (intake lock). Unknown session id → invalid params.

## 8. Permission requests

When a tool requires approval (and yolo/auto does not auto-approve):

1. Agent emits ACP reverse-request `session/request_permission` with tool call context + options (`AllowOnce`, `AllowAlways`, `RejectOnce`, …).
2. Session registers a **pending interaction** (`Permission`) keyed by `tool_call_id`.
3. Optional fire-and-forget `pending_interaction` vendor notification for UI badges.
4. Client responds with selected option or cancelled.
5. Guard drops → `interaction_resolved`.

Related blocking reverse-requests:

- `x.ai/ask_user_question`
- `x.ai/exit_plan_mode` (plan approval; may persist awaiting state)

**Tracer control plane must implement a Client handler** for permission reverse-requests or run with `--always-approve` only for trusted automation.

## 9. Cancellation

Client sends ACP cancel notification:

```json
{
  "jsonrpc": "2.0",
  "method": "session/cancel",
  "params": {
    "sessionId": "<uuid>",
    "_meta": {
      "cancelSubagents": true,
      "rewindIfPristine": false,
      "cancelTrigger": "user"
    }
  }
}
```

Agent behavior:

- Looks up session handle (waits if still loading)
- Sends `SessionCommand::Cancel` to session actor
- Defaults: cancel subagents `true`; kill background tasks `false` unless specified
- Optional pristine rewind when configured (`cancelRewind` capability)

Cancel during permission prompt maps to cancelled tool loop / cancelled stop reason.

## 10. Shutdown

| Action | Expected behavior |
|---|---|
| Close client stdin | Stdin reader EOF → simplex shutdown after short grace → ACP IO ends → process exits |
| Drop last WS client (serve mode) | Agent thread may persist for reconnect (serve design) |
| Kill process (SIGKILL / TerminateProcess) | Abrupt; PTYs may be reaped by Job Object on Windows |
| Ctrl+C on agent process | Non-unix path: telemetry flush with exit code 130 |

`run_stdio_agent` cleanup:

- On exit path, **close all PTY sessions** so shell children do not outlive the agent.

**Tracer recommendation:**

1. Prefer graceful: cancel active prompt → optional `x.ai/session/close` → close stdin → wait with timeout → kill if needed.
2. On Windows, use a Job Object (or ensure child tree kill) when force-stopping.
3. Persist Tracer-side session mapping separately from grok’s on-disk session store.

## 11. Crash and exit behavior

| Event | Behavior |
|---|---|
| Panic in agent | Panic hook installed via telemetry; process dies; client sees broken pipe |
| Leader disconnect (stdio bridge mode) | Bridge attempts reconnect; may emit `x.ai/leader_reconnected` |
| Persistent serve agent thread dies | Server respawns agent thread on next connection |
| Auth token expiry mid-session | May silent refresh; else errors / reauth required |
| Model unavailable | Prompt may be blocked with end_turn + model auto-switch notifications |
| Stdin hang (historical Windows) | Mitigated by dedicated blocking stdin thread + Windows stdin handle duplication to `NUL` for stray readers |

No durable “crash recovery protocol” beyond:

- `session/load` of persisted transcript
- Leader reconnect for multi-client setups
- Tracer-level process supervisor (product responsibility)

## 12. Leader mode vs direct stdio

| | Direct `stdio` (`--no-leader`) | Leader-backed stdio |
|---|---|---|
| Process count | 1 agent process | Follower + shared leader |
| Isolation | Strong | Shared backend across clients |
| Reconnect | New process | Leader may keep sessions alive |
| Tracer MVP | **Prefer** | Avoid until multi-window sharing needed |

## 13. Platform notes (lifecycle)

### Windows

- Released `grok.exe` supports `agent stdio` (live-probed).
- Stdin uses dedicated OS thread; process stdin redirected to `NUL` after private handle dup to prevent deadlocks.
- Leader IPC uses **named pipes** (not AF_UNIX).
- Process groups: Job Object with kill-on-close semantics for child trees.
- OS sandbox enforce not applied like Linux/macOS.
- Source builds from this tree are **best-effort** per README.

### macOS / Linux

- First-class build hosts.
- Leader uses Unix domain sockets.
- Sandbox via Seatbelt (macOS) / Landlock (+ related) (Linux) when enforce feature enabled.
- Process group / setsid based child management.

## 14. Suggested Tracer process state machine

```text
Stopped → Starting → Initializing → Authenticating → Ready
  Ready → Prompting → Streaming → AwaitingPermission → Prompting
  Ready → Cancelling → Ready
  * → Stopping → Stopped
  * → Crashed → (user/agent restart) Starting
```

Map external signals:

| Runtime signal | Tracer state |
|---|---|
| Child spawn OK | Starting |
| `initialize` OK | Initializing done |
| `authenticate` OK | Authenticating done |
| `session/new` OK | Ready |
| `session/update` | Streaming |
| `session/request_permission` | AwaitingPermission |
| `session/prompt` result | Ready (or error) |
| stdin close / exit 0 | Stopped |
| unexpected exit | Crashed |
