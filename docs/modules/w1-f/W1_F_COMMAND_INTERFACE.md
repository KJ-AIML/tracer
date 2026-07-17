# W1-F Command Interface

Implements `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` via `tracer-control-plane` + Tauri glue.

## Registered commands

| Command | Handler |
|---|---|
| `tracer_app_info` | App version / protocol / platform |
| `tracer_presentation_snapshot` | Versioned shell snapshot |
| `tracer_heli_status` | Read-only Heli probe |
| `tracer_project_list` / `register` / `get` | Projects |
| `tracer_session_list` / `create` / `get` | Sessions |
| `tracer_session_submit_prompt` | Prompt (blocks until terminal) |
| `tracer_session_cancel` / `stop` | Cancel / process stop |
| `tracer_events_list` | History (storage order) |
| `tracer_approval_list_pending` / `resolve` | Approvals (fail-closed) |
| `tracer_runtime_status` | Process/protocol/session gates |

## Rules

- Validate inputs; return structured `CommandError` (`errorClass`, `message`, `retryable`, `details`).
- No raw ACP payloads in args or results.
- No direct SQLite from command handlers.
- No direct process management outside control plane / adapter.
- Distinct error classes: `AuthenticationRequired` ≠ `AuthenticationFailed` ≠ `RuntimeCrashed` ≠ `RuntimeDisconnected` ≠ protocol classes.

## Event stream

- Contract channel name: `tracer://events`
- Presentation events are normalized envelopes with **storage sequences**.
- Shell may call `tracer_events_list` / snapshot to restore after missed live events.
