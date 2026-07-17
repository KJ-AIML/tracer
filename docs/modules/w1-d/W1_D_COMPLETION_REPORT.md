# W1-D Completion Report — ACP Client and Runtime Adapter

| Field | Value |
|---|---|
| **Task ID** | `tracer-w1-acp-adapter` |
| **Work item** | W1-D |
| **Branch** | `agent/tracer-w1-acp-adapter` |
| **Base** | Gate 1.1 `bfcd205832fd9befa9d78dd204cb6916c7ad6385` (tag `tracer-wave1.1-foundation`) |
| **Heli session** | `heli-ses-6c28d7c8-3ca0-44dd-b4a7-83784a0f1450` |
| **Lease** | `heli-lease-f21b21f6-3196-485a-9343-da87191769af` |
| **Host** | `grok-build` |
| **Target** | `tracer` (`repos/worktrees/tracer-w1-d`) |
| **Date** | 2026-07-17 |
| **Status** | **COMPLETE** (local commits; not pushed) |

## 1. Ownership (reaffirmed)

### OWNED (written)

| Path | Content |
|---|---|
| `crates/tracer-acp-client/` | Transport, codec, session protocol SM |
| `crates/tracer-runtime-adapter/` | Process composition, normalizer, public API |
| `tests/contract/acp/` | README pointer to crate tests |
| `tests/integration/acp-adapter/` | README pointer to fake scenarios |
| `docs/modules/w1-d/` | Architecture, public interface, test matrix, this report |

### FORBIDDEN (not modified)

- `crates/tracer-domain`, `tracer-process`, `tracer-storage`, `tracer-heli`
- `apps/desktop`, `packages/ui`, `packages/event-types`, `packages/test-fixtures` source
- W1-F control plane / Tauri composition

### Shared manifests

Minimal root `Cargo.toml` member registration only (documented in `SHARED_MANIFEST_REQUESTS.md`).

## 2. Architecture

See `W1_D_ARCHITECTURE.md`. Layers preserved:

transport → codec → session SM → adapter → domain events.

## 3. Tests

| Suite | Result (Windows, this host) |
|---|---|
| `cargo test -p tracer-acp-client` | **15 passed** (10 lib + 5 integration) |
| `cargo test -p tracer-runtime-adapter --lib` | **3 passed** |
| `cargo test -p tracer-runtime-adapter --test fake_scenarios -- --test-threads=1` | **20 passed** |

### Fake scenarios covered

init+session success; auth required; permission allow/deny; cancel mid-stream; cancel while permission; malformed; unknown vendor; multi-msg/partial (codec); EOF mid-prompt; crash; cancel unsupported; duplicate id; capability minimal; clean shutdown; sequence order; shutdown without orphan; fresh session restart.

### Risk tests

| Risk | Status |
|---|---|
| Permission-cancel deadlock (time-bounded) | PASS |
| Process vs session readiness APIs | PASS |
| Orphan cleanup via W1-C | PASS |
| Synthetic labeling | PASS |
| Error taxonomy distinct | PASS |
| Unknown events no crash | PASS |

### Live smoke

**Not run** as gate. Classification: optional only. Failure must not fail standard acceptance.

### Network / credentials

None required. Fake runtime only.

## 4. Assumptions

1. Fake ACP NDJSON shapes from W1-G are authoritative for CI.
2. Adapter may assign live-stream `sequence`/`eventId`; W1-F remains sole SQLite writer and may re-key.
3. `session/cancel` is a notification; completion observed via prompt RPC result.
4. Stock Grok path is `grok agent --no-leader stdio`; not exercised in standard CI.

## 5. Platform limits

- Windows Job Object orphan prevention depends on W1-C.
- Blocking `submit_prompt` requires concurrent threads for cancel/approval.
- Unbounded event mpsc — W1-F must drain.

## 6. Unresolved questions

1. Should W1-F always re-sequence adapter envelopes at storage boundary, or adopt adapter sequences?
2. Auth product events beyond session/new error (e.g. dedicated `runtime.auth_*`) — not in W0-A catalog; deferred.
3. Content-Length framing alternative — not needed for Grok/fake (NDJSON only).

## 7. W1-F handoff readiness

| Item | Ready? |
|---|---|
| Public API documented | Yes — `W1_D_PUBLIC_INTERFACE.md` |
| Normalized events only | Yes |
| Process compose via tracer-process | Yes |
| Fake CI path | Yes |
| Approval never auto | Yes |
| Readiness gates explicit | Yes |

**Recommendation:** W1-F may claim and integrate against this adapter.

## 8. Gate 1.2 recommendation

**PROCEED** with W1-F integration toward Gate 1 vertical slice.  
W1-D standard acceptance evidence is fake-runtime complete; live Grok remains optional non-gate.

## 9. Commits

(Filled after local commit.)

## 10. Explicit non-starts

- W1-F not started
- No push
- No grok-build repo edits
- No headless/watchdog launchers
