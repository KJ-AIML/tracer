# W2.2-B Journey Spec — GJ-01 … GJ-12

All journeys are **real GUI** interactions against the Tauri WebView.

## Shared preconditions

| Precondition | Value |
|---|---|
| Binary | Built `tracer-desktop` |
| Drivers | `tauri-driver` + major-matched `msedgedriver` |
| ACP | Fake only (`TRACER_FAKE_ACP_JS`) |
| Storage | Temp file SQLite (`TRACER_DATABASE_PATH`) |
| Heli probe | Empty dir (non-fatal unavailable) |
| Network / credentials / live Grok | **No** |

## Journeys

| ID | Name | GUI steps (summary) | Pass criteria |
|---|---|---|---|
| **GJ-01** | Startup | Launch → wait `tracer-app-ready` | `data-tracer-backend="tauri"`; shell visible; not mock |
| **GJ-02** | Create first session | Register project path → Create session (happy_prompt_stream) | Session workspace; status → `ready` |
| **GJ-03** | Streaming prompt | Type prompt → Send | Timeline shows prompt and/or agent message / completion events |
| **GJ-04** | Approval accepted | Session with `permission_allow` → prompt → **Allow** | Approval card clears; progress/events continue |
| **GJ-05** | Approval rejected | Session with `permission_deny` → prompt → **Deny** | Approval card clears |
| **GJ-06** | Cancel while approval pending | `cancel_while_permission_pending` → prompt → **Cancel** | Reaches non-deadlocked terminal-ish status |
| **GJ-07** | Two-session focus | Create A → Create B → Open A | Workspace `data-session-id` switches to A |
| **GJ-08** | Runtime crash/EOF | `crash_nonzero_exit` → prompt | Disconnected/failed/crash banner or exit events |
| **GJ-09** | Restart + history | Prompt → relaunch same DB → open session | History events restored (or PARTIAL if empty race) |
| **GJ-10** | Heli unavailable | Empty heli probe | App remains Tauri-usable; banner preferred |
| **GJ-11** | Invoke fail-closed | Invalid project path register | Error surfaced; backend stays `tauri`; no mock controls |
| **GJ-12** | Clean shutdown | Soft stop + harness teardown | No orphans (`tracer-desktop`, drivers) |

## Classification vocabulary

```text
PASS | PARTIAL | BLOCKED_BY_PRODUCT_GAP | BLOCKED_BY_FIXTURE
| BLOCKED_BY_TOOLING | FAIL
```

**BLOCKED_BY_PRODUCT_GAP** requires: missing contract, owning module, smallest remediation — stop that journey; do not copy backend into React/harness.

## Product gap rule

If a journey cannot complete without redesigning `crates/tracer-*` (domain/process/storage/acp/runtime-adapter/control-plane), classify **BLOCKED_BY_PRODUCT_GAP** and leave remediation to the owning module.

## Selector contract

Prefer accessible names and `data-testid="tracer-…"`. See architecture doc §5.
