# Grok Build ACP Capability Matrix

**Task:** `tracer-w0-grok-runtime-recon` (W0-B)  
**Source revision:** `repos/grok-build` `SOURCE_REV=2ec0f0c8488842da03a71eeee3c61154957ca919`  
**Live binary probed:** `grok 0.2.102` (`agent --no-leader stdio`)  
**ACP crate pin (upstream):** `agent-client-protocol = 0.10.4` (feature `unstable`)  
**Date:** 2026-07-17  
**Scope:** Stock Grok Build as Tracer’s first ACP-compatible runtime.

## 1. Primary start command

| Mode | Command | Transport | Tracer MVP default? |
|---|---|---|---|
| **Stdio ACP server** | `grok agent stdio` | JSON-RPC 2.0 newline-delimited over stdin/stdout | **Yes** |
| Stdio without leader | `grok agent --no-leader stdio` | Same | Recommended for isolation |
| WebSocket serve | `grok agent serve --bind 127.0.0.1:2419 --secret <token>` | WS + secret | No (remote/desktop) |
| Headless relay | `grok agent headless --grok-ws-url wss://…` | Outbound WS relay | No |
| Shared leader | `grok agent leader` / auto-spawn when `use_leader` | Local IPC (Unix socket / Windows named pipe) | Optional later |

**Flags that apply to `grok agent` before the mode subcommand:**

| Flag | Effect |
|---|---|
| `-m, --model <MODEL>` | Default model (e.g. `grok-build`) |
| `--always-approve` / `--yolo` | Auto-approve tool executions |
| `--reauth` | Force authentication before start |
| `--agent-profile <PATH>` | Load agent profile file |
| `--plugin-dir <DIR>` | Process-only trusted plugin dirs (SDK use) |
| `--leader` / `--no-leader` | Force / disable shared leader process |
| `--cli-chat-proxy-base-url` / `--xai-api-base-url` | Endpoint overrides |
| `--debug` / `--debug-file <FILE>` | Logging (stderr/file; keep stdout clean for JSON-RPC) |

Binary build targets (from source): package `xai-grok-pager-bin` → release binary historically named `xai-grok-pager`; installed product name is `grok`.

## 2. Standard ACP methods

| Method | Direction | Supported | Notes |
|---|---|---|---|
| `initialize` | C→A | Yes | Required before session ops. Single-call invariant for auth method selection (reconnect re-init is tolerated but does not re-write auth id). |
| `authenticate` | C→A | Yes | Methods advertised in `initialize.authMethods` (e.g. `grok.com`, `xai.api_key`, enterprise OIDC). |
| `session/new` | C→A | Yes | Requires prior `initialize`. Without auth → error `-32000 Authentication required`. |
| `session/load` | C→A | Yes | `agentCapabilities.loadSession = true`. Resume persisted session. |
| `session/prompt` | C→A | Yes | Streams `session/update` notifications; returns `PromptResponse` with stop reason. |
| `session/cancel` | C→A (notification) | Yes | Cancels in-flight turn; optional `_meta.cancelSubagents`, `_meta.rewindIfPristine`, `_meta.cancelTrigger`. |
| `session/set_mode` | C→A | Yes | Session mode switch (plan/agent etc. as configured). |
| `session/set_model` | C→A | Yes | Model switch; respects `allowed_models` / user-selectable catalog. |
| `session/update` | A→C | Yes | Standard update types + vendor content in `_meta` / extension notifications. |
| `session/request_permission` | A→C | Yes | Blocking reverse-request for tool approval. |
| `fs/read_text_file` / `fs/write_text_file` | A→C | Client-capability gated | Client advertises in `initialize.clientCapabilities.fs`. |
| `terminal/*` (ACP client terminal) | A→C | Client-capability gated | Client advertises `terminal: true`. |

## 3. Initialize capability surface (observed live)

Observed from hermetic `initialize` against `grok 0.2.102` (no credentials in temp `GROK_HOME`):

### Agent capabilities

| Capability | Value |
|---|---|
| `protocolVersion` | `1` |
| `loadSession` | `true` |
| `promptCapabilities.embeddedContext` | `true` |
| `promptCapabilities.image` | `false` (this binary/build) |
| `promptCapabilities.audio` | `false` |
| `mcpCapabilities.http` | `true` |
| `mcpCapabilities.sse` | `true` |
| `agentCapabilities._meta["x.ai/fs_notify"]` | `true` |
| `agentCapabilities._meta["x.ai/hooks"]` | `{ blockingEvents: ["pre_tool_use"], decisions: ["deny"] }` |

### Top-level `_meta` (vendor)

| Field | Meaning |
|---|---|
| `grokShell` | Identifies Grok shell runtime |
| `defaultAuthMethodId` | Preferred auth method id (null if none ready) |
| `x.ai/mcp/sdk` | MCP SDK capability flag |
| `x.ai/pluginDirs` | Supports session/plugin dir injection |
| `currentWorkingDirectory` | Process launch cwd |
| `agentVersion` | e.g. `0.2.102` |
| `agentId` / `agentInstanceId` | Stable agent id vs process instance id |
| `hostname` | Host name |
| `modelState` | `{ currentModelId, availableModels }` |
| `mcpServers` | Initially empty list at init (populated later) |
| `mcpApps` | Echo of client init support |
| `availableCommands` | Slash commands exposed to clients |
| `cancelRewind` | Cancel-can-rewind feature flag |
| `sessionRecap` | Session recap feature flag |
| `voiceMode` | Voice mode feature flag |

### Auth methods (environment-dependent)

| Condition | Typical `authMethods` |
|---|---|
| No credentials / no API key | `[{ id: "grok.com", name: "Grok", … }]` |
| `XAI_API_KEY` or stored API key present | `xai.api_key` advertised (often first when BYOK) |
| Enterprise OIDC configured | Enterprise OIDC method + policy-driven preferred method |

## 4. Client capabilities Grok understands

From `initialize` client `_meta` / capabilities (source + test harness):

| Client capability / meta | Purpose |
|---|---|
| `clientCapabilities.fs.readTextFile` / `writeTextFile` | Allow agent to ask client to read/write files |
| `clientCapabilities.terminal` | ACP terminal reverse-requests |
| `_meta.clientType` / `_meta.clientIdentifier` | Telemetry / behavior variants (`generic`, `grok-desktop`, `nebula`, `tracer`, …) |
| `_meta.clientVersion` | Client version string |
| `_meta.startupHints.nonInteractive` | Skip interactive prompts |
| `_meta.startupHints.skipGitStatus` / `skipProjectLayout` | Faster hermetic startup |
| `_meta.mcpApps` | Client supports MCP Apps |
| `_meta.bufferingSettings` | Chunk merge / buffering for streaming |
| Code-nav / interactive-trust capability meta | Optional advanced features |

**Tracer recommendation:** identify as `clientIdentifier: "tracer"` (or similar) and set `nonInteractive`/`skip*` for control-plane spawns; advertise `fs` + `terminal` only when UI can answer reverse-requests.

## 5. Session `_meta` options (`session/new`)

| Field | Purpose |
|---|---|
| `sessionId` | Client-supplied UUID (must parse as UUID) |
| `modelId` | Pre-select model |
| `yoloMode` | Session always-approve |
| Auto-mode fields | Permission auto policy (resolved with defaults) |
| `rules` / `systemPromptOverride` / `agentProfile` | Prompt/profile overrides (docs) |
| `x.ai/mcp/servers` | Attach managed/local MCP servers |
| Session kind / computer-session fields | Chat vs coding session variants |

## 6. Prompt / streaming capabilities

| Feature | Status |
|---|---|
| Text prompt content blocks | Yes |
| Embedded context | Yes (`embeddedContext: true`) |
| Images / audio in prompt | Advertised false on probed binary |
| Streaming agent message chunks | Yes (`agent_message_chunk`) |
| Thought/reasoning chunks | Yes (`agent_thought_chunk`) |
| Tool call + tool call update | Yes |
| Plan updates | Yes |
| Available commands update | Yes |
| Current mode update | Yes |
| Tool arg delta chunks | Vendor (`tool_call_delta_chunk` via `x.ai/session_notification`) |
| Prompt queue / `sendNow` / `verbatim` | Vendor `_meta` on `session/prompt` |
| Prompt modes (plan etc.) | `_meta.mode` or session current mode |
| Usage in response `_meta` | Present for prompt usage accounting |

## 7. Permission / human-in-the-loop

| Mechanism | Protocol | Blocking? |
|---|---|---|
| Tool permission | ACP `session/request_permission` | Yes (turn parks) |
| User question | Vendor `x.ai/ask_user_question` | Yes |
| Plan approval | Vendor `x.ai/exit_plan_mode` | Yes (may persist gate) |
| Always-approve | CLI `--always-approve` / session yolo / slash command | Disables prompts |
| Auto mode | Config + `_meta` / notifications | Policy-based auto decisions |
| Pre-tool hooks | `x.ai/hooks` capability | Can deny tool without user UI |

Pending interactions are **not** persisted; clients must answer reverse-requests while the process is live. Roster surfaces `NeedsInput` when a park is open.

## 8. Vendor extension method families (`x.ai/*`)

Non-exhaustive (discovered from `ext_method` dispatch in shell). Treat as unstable across releases.

| Family | Examples | Tracer need (MVP) |
|---|---|---|
| Session admin | `x.ai/session/info`, `close`, `list`, `fork`, `rename`, `delete`, `repair`, `updates`, `load_history`, `search` | Medium (list/close later) |
| Auth | `x.ai/auth/*`, `getApiKey`, `setApiKey` | High (auth UX) |
| FS | `x.ai/fs/*` | Optional (prefer ACP fs reverse-requests) |
| Git / worktree | `x.ai/git/*`, `x.ai/git/worktree/*` | Later |
| Terminal | `x.ai/terminal/*` | Later / optional |
| Search | `x.ai/search/*` | Later |
| Memory / compact | `x.ai/memory/*`, `x.ai/compact_conversation` | Later |
| Skills / plugins / marketplace / hooks | `x.ai/skills/*`, `plugins/*`, `marketplace/*`, `hooks/*` | Later |
| Tasks / scheduler / subagent | `x.ai/task/*`, `scheduler/*`, `subagent/*` | Later |
| Feedback / recap / suggest / billing / privacy / cloud | various | Later |
| Code nav | `x.ai/code/*` | Later |

### Vendor notifications (agent → client)

| Method | Purpose |
|---|---|
| `session/update` | Standard ACP streaming |
| `x.ai/session_notification` | Rich session updates (diff review, retry, compact, subagents, goals, pending_interaction, …) |
| `x.ai/session/update` | Additional session update channel (docs) |
| `x.ai/fs_notify` / `x.ai/fs/index` / `x.ai/fs/index/delta` | FS index |
| `x.ai/search/fuzzy/status` | Fuzzy search |
| `x.ai/git/worktree/status` | Worktree progress |
| `x.ai/leader_reconnected` | Leader reconnect (stdio bridge) |
| `_x.ai/mcp/servers_updated` | MCP server list changed (underscore prefix observed live) |

## 9. Minimum capabilities Tracer needs (vertical slice)

| Need | Source | Status on stock Grok |
|---|---|---|
| Spawn local runtime | `grok agent stdio` | Yes |
| Initialize + version/capability discovery | `initialize` | Yes |
| Auth | `authenticate` + methods | Yes (requires user credentials) |
| Create session for repo cwd | `session/new` | Yes after auth |
| Prompt + stream text | `session/prompt` + `agent_message_chunk` | Yes |
| Tool visibility | `tool_call` / `tool_call_update` | Yes |
| Permission UI | `session/request_permission` | Yes |
| Cancel turn | `session/cancel` | Yes |
| Stop process cleanly | Close stdin / kill child; agent closes PTYs | Yes (see PROCESS_LIFECYCLE) |
| Resume session | `session/load` | Yes (persisted under grok home) |
| Multi-runtime adapter neutrality | Adapter boundary | Design-only (W0-A) |

## 10. Gaps / non-goals for stock runtime

- **No single “ready” JSON-RPC notification** after process start — readiness is “process alive + successful `initialize` response”.
- **Kernel sandbox** (Landlock/Seatbelt) is Unix-oriented; Windows runs without OS sandbox enforce.
- **Windows source builds** are best-effort; released Windows binaries exist and work for stdio (probed).
- **Huge vendor surface** should not be required for MVP; isolate behind adapter optional traits.
- **Auth is mandatory** for `session/new` in real runs; hermetic tests use mock inference + api key path.

## 11. Evidence paths (read-only)

| Topic | Path under `repos/grok-build` |
|---|---|
| CLI agent modes | `crates/codegen/xai-grok-pager/src/app/cli.rs` |
| Binary dispatch | `crates/codegen/xai-grok-pager-bin/src/main.rs` |
| Stdio entry | `crates/codegen/xai-grok-shell/src/agent/app.rs` (`run_stdio_agent`) |
| ACP Agent impl | `crates/codegen/xai-grok-shell/src/agent/mvp_agent/acp_agent.rs` |
| Extension notifications | `crates/codegen/xai-grok-shell/src/extensions/notification.rs` |
| Stdin transport | `crates/codegen/xai-acp-lib/src/stdin_reader.rs` |
| Test harness | `crates/codegen/xai-grok-test-support/src/acp_client.rs` |
| User docs | `crates/codegen/xai-grok-pager/docs/user-guide/15-agent-mode.md` |
| Live fixtures | `tests/fixtures/acp/` (this worktree) |
