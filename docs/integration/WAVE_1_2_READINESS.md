# Wave 1.2 Readiness — After Gate 1.1 Foundation

**Status:** Gate 1.1 output  
**Version:** 1.0.0  
**Owner task:** `tracer-w1-1-integration`  
**Depends on:** Gate 1.1 **PASS** and FF onto `main`  
**Date:** 2026-07-17

## 1. Purpose

Authorize Wave 1.2 modules **without starting them** from the Wave 1.1 integrator. Record what is ready, blocked, and first acceptance targets for:

- **W1-D** — ACP Client and Runtime Adapter  
- **W1-F** — Control Plane Integration  

## 2. Global readiness after Gate 1.1

| Check | State |
|---|---|
| Gate 0 | **PASS** (on main pre-1.1) |
| Gate 1.1 foundation (A/B/C/E/G/H) | **PASS** (this wave) |
| Application source on main | Yes — domain, process, storage, heli, fake runtime, desktop shell + workspace wiring |
| Stock Grok required for standard CI | **No** — fake ACP default |
| Live credentials required for Gate 1 | **No** for standard CI; optional T6 later |
| W1-D / W1-F started by W1.1-I | **No** |

## 3. Foundation inputs available to W1-D / W1-F

| Module | Path(s) | Usable for next wave |
|---|---|---|
| W1-B Domain | `crates/tracer-domain`, `packages/event-types`, `tests/contract/event-protocol/` | **Yes** — canonical envelope, IDs, statuses, errors |
| W1-C Process | `crates/tracer-process`, `tests/integration/process/` | **Yes** — spawn/pipes/shutdown/Job Object; process-only readiness |
| W1-E Storage | `crates/tracer-storage`, migrations, VS-10 tests | **Yes** — ordered events, sole-writer design for control plane |
| W1-G Fake ACP | `tools/fake-acp-runtime`, `packages/test-fixtures`, `tests/contract/fake-runtime` | **Yes** — catalog scenarios without W1-D (stdio NDJSON) |
| W1-A Shell | `apps/desktop`, `packages/ui` | **Yes** — placeholders + invoke wrappers; mock store |
| W1-H Heli | `crates/tracer-heli`, `docs/agent-workflows/` | **Yes** — read-only status; orthogonal to product runtime |
| Workspace | root `Cargo.toml`, `package.json`, locks | **Yes** — monorepo install/test |

## 4. W1-D — ACP Client and Runtime Adapter

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-acp-adapter` (per readiness matrix) |
| **Owned paths (planned)** | `crates/tracer-acp-client/`, `crates/tracer-runtime-adapter/`, `packages/runtime-client/`, `tests/contract/acp/` |
| **Required inputs** | Runtime adapter + event contracts; ACP mapping; fixtures; expected-events; process I/O; fake scenarios |
| **Contracts available** | Yes (Gate 0) |
| **Dependencies met** | W1-B types **yes**; W1-G fake **yes**; W1-C process I/O **yes** (compose via traits/APIs) |
| **Blockers** | None hard for full adapter work |
| **First acceptance test** | Parse `initialize-response` fixture → capabilities + ready synthesis; map auth-required without `session.ready`; unknown vendor → `adapter.protocol.unknown` |
| **Safe to claim after main tip includes Gate 1.1** | **Yes** |
| **Must not** | Own spawn; own SQLite; expose raw Grok events to UI; auto-approve |
| **Integration notes** | Prefer consuming `tracer-process` handles for pipes; emit domain envelopes (`tracer-domain` / `@tracer/event-types`); drive CI with `fake-acp-runtime` scenario IDs |

## 5. W1-F — Control Plane Integration

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-control-plane` |
| **Owned paths (planned)** | `crates/tracer-control-plane/`, `crates/tracer-permissions/`, `apps/desktop/src-tauri/src/` (command composition), `tests/integration/control-plane/` |
| **Required inputs** | Full Tauri command contract; UX status transitions; failure matrix; VS-01…VS-14 |
| **Contracts available** | Yes |
| **Dependencies** | **Hard:** W1-B, W1-C, W1-D, W1-E — B/C/E landed; **W1-D still required** before full VS E2E |
| **Blockers** | Soft start: scaffold interfaces against B/C/E; hard integrate after W1-D |
| **First acceptance test** | VS-01 happy path against fake ACP end-to-end via Tauri commands + event stream; VS-02 auth gate; VS-06 crash honesty |
| **Recommended launch** | Claim after W1-D progress or in parallel for scaffold only; full Gate 1 evidence after D+F wire |
| **Must not** | Duplicate adapter/process/storage logic; raw ACP to React; auto-approve; UI as DB writer |
| **Integration notes** | Sole SQLite writer; shell invoke wrappers already named per contract; migrate desktop mock store → real events |

## 6. Dependency graph (post-1.1)

```text
W1-B ──┐
W1-C ──┼──► W1-D ──┐
W1-G ──┘           ├──► W1-F ──► Gate 1 vertical slice
W1-E ──────────────┘
W1-A ◄── mock now ──► real commands/events when F lands
W1-H (orthogonal governance)
```

## 7. Explicit non-starts (this task)

| Item | Status |
|---|---|
| Create/start W1-D implementation | **Not done** |
| Create/start W1-F implementation | **Not done** |
| Push remote | **Not done** |
| Live Grok T6 as gate requirement | **Not required** |

## 8. Recommended Wave 1.2 sequence

1. Claim **W1-D**; implement framing + normalizer + process composition against fake scenarios.  
2. Scaffold **W1-F** interfaces (optional parallel).  
3. Integrate **W1-F** commands, permissions, storage writes, event fan-out to shell.  
4. Collect VS-01…VS-14 evidence on fake path; platform orphan already green on Windows from W1-C.  
5. Gate 1 integration when D+F complete.

## 9. Residual foundation debt for D/F consumers

- Promote remaining storage-local IDs (`ProcessId`, `ApprovalId`, `ArtifactId`) into domain if shared.  
- Optional: hard-depend `@tracer/ui` on `@tracer/event-types` for single `SessionStatus` export.  
- Ensure CI builds frontend before `tracer-desktop` check (Tauri `frontendDist`).  
- Clippy hygiene on domain ID macros / sequence naming.
