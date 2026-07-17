# W1-B Completion Report â€” Domain and Event Protocol

**Work item:** W1-B  
**Task ID:** `tracer-w1-domain-events`  
**Host:** `grok-build`  
**Heli session:** `heli-ses-74310c22-7536-4713-b9bb-80303ef38606`  
**Lease:** `heli-lease-454d640b-f3e6-406a-81f0-db7ea0539c7b`  
**Branch:** `agent/tracer-w1-domain-events`  
**Base (Gate 0 main tip):** `e104d8d21a3370193decd9472036e037741ad3e7`  
**Date:** 2026-07-17

## 1. Decision

| Field | Value |
|---|---|
| **W1-B module** | **COMPLETE** (domain types + event protocol packages + contract fixtures) |
| Gate 1 vertical slice | Not claimed â€” foundation only |
| Shared manifests edited | **No** (forbidden); requests documented |

## 2. Owned vs forbidden (reaffirmed)

### Owned (written)

| Path | Content |
|---|---|
| `crates/tracer-domain/` | Rust Event Protocol v1 envelope, domain model, unit + fixture tests |
| `packages/event-types/` | TypeScript types/validators + fixture tests |
| `tests/contract/event-protocol/` | Fixtures + README |
| `docs/modules/w1-b/` | Completion report + shared-manifest requests |

### Forbidden (not touched)

- `crates/tracer-process`, `tracer-storage`, `tracer-acp*`, `tracer-control*`, `tracer-permissions`
- `apps/`, `tools/fake-acp-runtime/`
- `packages/ui/`, `packages/test-fixtures/`
- Root `Cargo.toml` / `package.json` / `pnpm-workspace.yaml`
- ACP parsing, SQLite, Tauri commands, UI, process spawn

## 3. Deliverables

### 3.1 `crates/tracer-domain`

| Module | Responsibility |
|---|---|
| `envelope` | Versioned v1 envelope (`eventVersion`, ids, sequence, timestamp, type, payload, adapter, severity) |
| `event_type` | Catalog + `Unknown(String)` preservation |
| `ids` | `EventId`, `ProjectId`, `SessionId`, `AgentRunId` (UUID) |
| `sequence` | `SequenceTracker`, gap/reorder validation |
| `session` | Status catalog + transition graph + terminal helpers |
| `auth` | Authentication states + prompt gate |
| `capabilities` | Negotiated caps + unknown vendor key preservation |
| `error` | `ErrorClass`, `ErrorCategory` (protocol/process/authentication/permission/storage/â€¦), `TracerError` |
| `adapter` | Adapter metadata + `extensions` for vendor preserve |
| `payload` | Builders for common payloads; tool/approval/plan enums |
| `validate` | Semantic envelope + stream validation |
| `severity` | info/warn/error |

### 3.2 `packages/event-types` (`@tracer/event-types`)

- Type mirrors for envelope, statuses, auth, caps, error classes/categories
- `parseEnvelope` (required fields, UUID checks, unknown-field tolerance)
- `validateSequenceOrder` / `validateSessionEventStream` / `roundTripEnvelope`
- Node test suite against shared fixtures

### 3.3 Contract fixtures (`tests/contract/event-protocol/fixtures/`)

| Fixture | Covers protocol Â§10 corpus item |
|---|---|
| `happy_prompt_stream.json` | Happy-path stream |
| `tool_with_approval.json` | Tool + approval request/resolve |
| `unknown_vendor_notification.json` | Unknown notification â†’ `adapter.protocol.unknown` |
| `protocol_error.json` | Malformed â†’ `adapter.protocol.error` |
| `cancel_mid_tool.json` | Cancel mid-tool |
| `unexpected_process_exit.json` | Unexpected process exit mid-run |
| Replay covered in tests | Sort by `sequence` after reverse |

Fixtures use synthetic UUIDs and relative paths only.

## 4. Verification evidence

### Rust

```text
cd crates/tracer-domain
cargo test
# 27 lib unit tests + 14 integration (envelope_roundtrip) = 41 passed
```

### TypeScript

```text
cd packages/event-types
npm install
npm test
# 11 passed
```

## 5. Shared-manifest requests

See [`SHARED_MANIFEST_REQUESTS.md`](./SHARED_MANIFEST_REQUESTS.md):

- Root Cargo workspace member: `crates/tracer-domain`
- pnpm workspace package: `packages/event-types` â†’ `@tracer/event-types`

## 6. Additive contract notes (not contract rewrites)

Per Gate 0.1 / Gate 0 recommendations, domain types include:

- `AuthenticationRequired`
- `AuthenticationFailed`

as stable `ErrorClass` values mapped to category `authentication`.

## 7. Explicit non-scope (confirmed)

- No ACP framing/parsing
- No SQLite / storage schema
- No Tauri commands
- No UI components
- No process spawn / job objects
- No edits to frozen `docs/contracts/*` (implementation follows them)

## 8. Blockers

| Item | Status |
|---|---|
| Missing root Cargo/pnpm workspace | **Non-blocking** â€” packages test standalone; integrator wiring documented |
| Fake ACP / adapter | Out of scope (W1-D / W1-G) |

## 9. Commit SHAs


| SHA | Message |
|---|---|
| `eb17ce04ed6843868d3b8e0d3a2a816545acfe76` | `feat(w1-b): domain types and event protocol v1 packages` |

## 10. Handoff

Downstream consumers:

- **W1-D** adapter normalizer should emit `type` + `payload` + `adapter`; control plane wraps with envelope identity/sequence
- **W1-E** storage should persist full envelope JSON ordered by `sequence`
- **W1-F** control plane owns `SequenceTracker` and status transitions
- **W1-A** UI should import `@tracer/event-types` only (not raw ACP)

---

**Document control:** W1-B completion artifact. Do not push from worker; integrator merges.
