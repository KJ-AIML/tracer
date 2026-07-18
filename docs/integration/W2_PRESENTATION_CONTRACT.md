# W2 Presentation Contract

## Snapshot-authoritative model

1. `PresentationHub` owns the canonical `PresentationSnapshot` and monotonic `revision`.
2. Consumers receive coalescing notifications (default capacity **1** — deliberate).
3. Notifications are **hints**; UI/recovery must re-read `snapshot()`.
4. Lost/duplicated notifications are recoverable via snapshot (INV-05/06).
5. Terminal statuses are sticky until a non-terminal publish for the active session (INV-04).
6. Persist path never blocks on presentation (INV-01).
7. Slow/absent consumers cannot force unbounded growth (INV-02).

## Multi-session interaction

| Operation | Projection behavior |
|---|---|
| `presentation_focus(session)` | Force focus + `publish_snapshot` |
| `session_create` | Force focus to new session |
| `session_submit_prompt` / cancel / approval | `publish_session_update` — **only if focused** |
| Ingest post-persist | `publish_session_update` — **only if focused** |
| `session_stop` focused | Clear focus fields via hub |
| `shutdown_all` | Clear registry + focus |
| `shutdown_presentation` | Tear down consumers/forwarders |

## Constants

- `DEFAULT_NOTIFY_CAPACITY = 1`
- Schema `version` (SNAPSHOT_VERSION) distinct from delivery `revision`

## Desktop surface

- `tracer_presentation_snapshot`
- `tracer_presentation_focus` (W2.1)