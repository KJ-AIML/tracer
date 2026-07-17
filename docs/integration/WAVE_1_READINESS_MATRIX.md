# Wave 1 Readiness Matrix

**Status:** Gate 0 output  
**Version:** 1.0.0  
**Owner task:** `tracer-w0-final-integration`  
**Depends on:** Final Gate 0 PASS and FF onto `main`  
**Normative contracts:** `docs/contracts/*`, `docs/architecture/*`, `docs/decisions/*`  
**UX freeze:** `docs/ux/*`  
**Test freeze:** `docs/testing/*`, `tests/specifications/*`  
**Wire evidence:** `docs/research/grok-build/*`, `tests/fixtures/acp/*`

## 1. Purpose

Authorize and sequence Wave 1 foundation modules without starting their implementation from this task. For each module: required inputs, contract availability, dependencies, blockers, owned paths, first acceptance test, parallel-start safety, and recommended launch wave.

## 2. Global readiness

| Check | State |
|---|---|
| Gate 0 | **PASS** (see `FINAL_GATE_0_REPORT.md`) |
| Wave 1 authorized | **YES** after final main tip includes Gate 0 artifacts |
| Application source on main | None yet (docs + fixtures/specs only) — expected |
| Stock Grok required for first PR CI | **No** — fake ACP default |
| Live credentials required for Gate 1 | **No** for standard CI; optional T6 |

## 3. Module matrix

### W1-A — Desktop Shell

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-desktop-shell` |
| **Owned paths** | `apps/desktop/` (shell only), `packages/ui/` |
| **Excluded** | Session/timeline/approvals/changes/terminal feature bodies; `crates/`; `packages/event-types/` |
| **Required inputs** | UX IA + session layout placeholders (`docs/ux/INFORMATION_ARCHITECTURE.md`, `SESSION_SCREEN_SPEC.md`); Tauri command names for invoke wrappers (`TAURI_COMMAND_CONTRACT_V1.md`) |
| **Contracts available** | Yes — commands + event channel name; statuses for banner placeholders |
| **Dependencies** | None hard for scaffold; event types package later from W1-B for typed stores |
| **Blockers** | None for placeholder shell |
| **First acceptance test** | App builds; frontend unit smoke; shell renders projects/session **placeholders** with mock store (no real ACP); status not color-only for mock states |
| **Safe parallel start** | **Yes** |
| **Recommended launch wave** | **Wave 1.0 (immediate)** |
| **Must not** | Invent backend behavior; parse ACP; expand into full IDE |

### W1-B — Domain and Event Protocol

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-domain-events` |
| **Owned paths** | `crates/tracer-domain/`, `packages/event-types/`, `tests/contract/event-protocol/` |
| **Required inputs** | `TRACER_EVENT_PROTOCOL_V1.md`; expected-event schema patterns; domain vocabulary in vertical slice |
| **Contracts available** | Yes — envelope + type catalog frozen |
| **Dependencies** | None |
| **Blockers** | None |
| **First acceptance test** | Serde + TS validation of envelope; unknown type tolerance; fixture round-trip; reject missing required fields |
| **Safe parallel start** | **Yes** |
| **Recommended launch wave** | **Wave 1.0 (immediate)** |
| **Must not** | ACP parsing; SQLite; UI components; silent contract edits |

### W1-C — Runtime Process Manager

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-process-manager` |
| **Owned paths** | `crates/tracer-process/`, `tests/integration/process/` |
| **Required inputs** | ADR-001; PROCESS_LIFECYCLE.md; Stage 0.1 stock spawn note; FAILURE_MATRIX F-P*, F-W01 |
| **Contracts available** | Yes — process events and error classes on adapter/command surfaces (implement lifecycle emissions for control plane) |
| **Dependencies** | None for process crate; stock binary optional for smoke |
| **Blockers** | None for fake child process tests |
| **First acceptance test** | Spawn temp helper / fake binary; capture stdout/stderr; graceful stdin-close; force kill; **Windows Job Object** no-orphan (on Windows hosts) |
| **Safe parallel start** | **Yes** |
| **Recommended launch wave** | **Wave 1.0 (immediate)** |
| **Must not** | Parse ACP; write session DB; hardcode machine Grok paths |

### W1-D — ACP Client and Runtime Adapter

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-acp-adapter` |
| **Owned paths** | `crates/tracer-acp-client/`, `crates/tracer-runtime-adapter/`, `packages/runtime-client/`, `tests/contract/acp/` |
| **Required inputs** | RUNTIME_ADAPTER + EVENT protocol; ACP_EVENT_MAPPING (concept→W0-A); fixtures; expected-events; catalog scenarios |
| **Contracts available** | Yes |
| **Dependencies** | **W1-B** types; **W1-G** fake runtime (or interim in-process double); process I/O API from **W1-C** (compose at F or via traits) |
| **Blockers** | Soft: fake scenario driver not yet implemented (W1-G) — can start framing + fixture normalizer tests in parallel |
| **First acceptance test** | Parse `initialize-response` live-scrubbed fixture → capabilities + ready synthesis; map auth-required fixture without `session.ready`; unknown vendor → `adapter.protocol.unknown` |
| **Safe parallel start** | **Partial** — framing/normalizer yes; full process integration after C+G |
| **Recommended launch wave** | **Wave 1.0 start (contract tests); Wave 1.1 full integration** |
| **Must not** | Own spawn; own SQLite; expose raw Grok events to UI; auto-approve |

### W1-E — Storage and Session Persistence

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-storage` |
| **Owned paths** | `crates/tracer-storage/`, `apps/desktop/src-tauri/migrations/`, `tests/integration/storage/` |
| **Required inputs** | Vertical slice persistence section; event envelope; session status model; VS-10 / F-S* |
| **Contracts available** | Yes |
| **Dependencies** | Prefer **W1-B** domain IDs/types |
| **Blockers** | Soft: domain types package |
| **First acceptance test** | Fresh DB + migration; insert ordered events; reload by sequence; unknown payload preserved; no secrets columns for tokens |
| **Safe parallel start** | **Yes** (with stub IDs if B slightly lags) |
| **Recommended launch wave** | **Wave 1.0 (immediate)** |
| **Must not** | ACP; UI; runtime-as-DB-writer |

### W1-F — Control Plane Integration

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-control-plane` |
| **Owned paths** | `crates/tracer-control-plane/`, `crates/tracer-permissions/`, `apps/desktop/src-tauri/src/`, `tests/integration/control-plane/` |
| **Required inputs** | Full Tauri command contract; UX status transitions; failure matrix; VS-01…VS-14 |
| **Contracts available** | Yes |
| **Dependencies** | **Hard:** W1-B, W1-C, W1-D, W1-E (scaffold interfaces early; integrate when ready) |
| **Blockers** | Incomplete B–E crates |
| **First acceptance test** | VS-01 happy path against fake ACP end-to-end via Tauri commands + event stream; VS-02 auth gate; VS-06 crash honesty |
| **Safe parallel start** | **Scaffold only** in Wave 1.0; **integrate Wave 1.2** |
| **Recommended launch wave** | **Wave 1.2 (after 1.0/1.1 lands)** |
| **Must not** | Duplicate adapter/process/storage logic; raw ACP to React; auto-approve |

### W1-G — Fake Runtime and Contract Harness

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-fake-runtime` |
| **Owned paths** | `tools/fake-acp-runtime/`, `packages/test-fixtures/`, portions of `tests/fixtures/`, `tests/contract/` (shared harness) |
| **Required inputs** | `tests/specifications/scenarios/catalog.yaml`; expected-events packs; fixture provenance rules; TEST_STRATEGY §5 |
| **Contracts available** | Yes — scenario IDs and NDJSON transport frozen |
| **Dependencies** | None hard; align type names with W1-B |
| **Blockers** | None |
| **First acceptance test** | Fake binary speaks NDJSON; scenario `happy_prompt_stream` + `auth_required_session_new` selectable; deterministic ordering; no network |
| **Safe parallel start** | **Yes** — **critical path enabler** |
| **Recommended launch wave** | **Wave 1.0 (immediate)** |
| **Must not** | Provider calls; become second production runtime |

### W1-H — HeliHarness Workspace Integration

| Field | Detail |
|---|---|
| **Task ID** | `tracer-w1-heliharness-integration` |
| **Owned paths** | `resources/prompts/` (coordinator-approved), `resources/reports/heliharness/` if used, `repos/tracer/docs/agent-workflows/`, `repos/tracer/.heli/` (or harness-conventional local dir) |
| **Required inputs** | Master plan Wave 1 task IDs; Gate 0 reports; concurrent claim discipline |
| **Contracts available** | N/A product contracts; harness policy applies |
| **Dependencies** | Gate 0 on main for accurate templates |
| **Blockers** | None |
| **First acceptance test** | Task templates for W1-A…W1-G claimable; handoff report format documented; target set tracer verified in checklist |
| **Safe parallel start** | **Yes** |
| **Recommended launch wave** | **Wave 1.0 (immediate)** |
| **Must not** | Edit `.heli-harness` distribution assets; replace harness task semantics |

## 4. Launch waves (recommended)

```text
Wave 1.0 (parallel, post Gate 0 on main)
  W1-B Domain/events
  W1-C Process manager
  W1-E Storage
  W1-G Fake ACP runtime
  W1-A Desktop shell (placeholders)
  W1-H HeliHarness docs/templates
  W1-D (framing + fixture normalizer only)

Wave 1.1
  W1-D full adapter against process I/O + fake scenarios

Wave 1.2
  W1-F control plane composition + Tauri commands + permissions
  Wire W1-A to real commands/events (still no feature-module polish beyond slice)

Gate 1 evidence
  VS-01…VS-14 as applicable via fake path
  Platform orphan tests on at least one OS
  Optional T6 live smoke documented separately
```

## 5. Dependency graph (summary)

```text
W1-B ──┐
W1-C ──┼──► W1-D ──┐
W1-G ──┘           ├──► W1-F ──► Gate 1 vertical slice
W1-E ──────────────┘
W1-A ◄── mock ──────► later binds to W1-F
W1-H (orthogonal governance docs)
```

## 6. First acceptance tests index

| Module | First test (short) | Spec link |
|---|---|---|
| W1-A | Shell build + mock status labels | UX STATE_MATRIX accessibility |
| W1-B | Envelope schema + unknown types | EVENT_PROTOCOL_V1 |
| W1-C | Spawn/kill/orphan | FAILURE_MATRIX F-P*, F-W01 |
| W1-D | Fixture normalize + auth gate | fixtures + VS-02 pack |
| W1-E | Ordered event replay | VS-10 / F-S04 |
| W1-F | VS-01 E2E fake | VERTICAL_SLICE_ACCEPTANCE |
| W1-G | Scenario driver catalog ids | scenarios/catalog.yaml |
| W1-H | Template checklist dry-run | master plan §15 |

## 7. Explicit non-starts

| Not authorized as Wave 1 “done” | Why |
|---|---|
| Wave 2 feature modules (rich timeline, diff viewer productization, terminal product) | After Gate 1 |
| Grok Build fork | FORK_RISK_REPORT / ADR-001: do not fork for slice |
| Standard CI live authenticated runs | Evidence policy |
| Full IDE shell | UX MVP fence |

---

**Document control:** Update when Gate 1 re-scopes module ownership; keep module IDs stable.
