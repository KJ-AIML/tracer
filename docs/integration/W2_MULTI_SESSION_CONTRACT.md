# W2 Multi-Session Contract

## Model

- Many **local** Tracer sessions may be live concurrently (`HashMap` registry).
- Each live session: own adapter process, drain/pump, approvals, `next_sequence`.
- **One prompt in flight per session**; parallel prompts across sessions allowed.
- Shared SQLite sole writer; isolation by `session_id` keys.
- `persist_failed` sticky **per session** only.

## Focus

- Presentation projects **one** focused `active_session_id`.
- Switch with `presentation_focus` without stopping peers.
- Background session work must not steal focus (MS-17).

## Lifecycle

- `session_stop` removes one live entry; peers continue.
- `shutdown_all` deterministic ordered teardown; registry empty.

## Tests

MS-01..MS-17 + stress multi-session suite (see Wave 2.1 test matrix).