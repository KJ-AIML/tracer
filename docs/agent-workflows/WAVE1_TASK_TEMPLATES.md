# Wave 1 Task Templates (Claimable)

Source of truth for **module scope**: `docs/integration/WAVE_1_READINESS_MATRIX.md` and `resources/TRACER_MASTER_BUILD_PLAN.md`.  
Heli task ids below match **created** Wave 1.1 tasks when present.

## Global claim preamble (all modules)

```text
1. Read .heli-harness/HARNESS.md (parent workspace)
2. Read docs/agent-workflows/CONCURRENT_CLAIM_CHECKLIST.md
3. Claim assigned task --mode write on a dedicated worktree
4. Export HELI_SESSION_ID
5. heli target set tracer
6. heli session status && heli status && heli conflicts
7. Stay inside owned paths; no push unless user authorizes
```

---

## W1-A — Desktop Shell

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-desktop-shell` |
| **Work item** | W1-A |
| **Owned** | `apps/desktop/` (shell only), `packages/ui/` |
| **Forbidden** | Session/timeline/approvals feature bodies; `crates/`; inventing backend behavior |
| **First test** | App builds; placeholder shell with mock store; status not color-only |
| **Depends** | None hard |

---

## W1-B — Domain and Event Protocol

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-domain-events` |
| **Work item** | W1-B |
| **Owned** | `crates/tracer-domain/`, `packages/event-types/`, `tests/contract/event-protocol/` |
| **Forbidden** | ACP parsing; SQLite; UI; silent contract edits |
| **First test** | Serde + TS envelope validation; unknown type tolerance; fixture round-trip |
| **Depends** | None |

---

## W1-C — Runtime Process Manager

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-process-manager` |
| **Work item** | W1-C |
| **Owned** | `crates/tracer-process/`, `tests/integration/process/` |
| **Forbidden** | ACP parse; session DB; hardcode machine Grok paths |
| **First test** | Spawn/kill fake child; stdin-close; Windows Job Object no-orphan |
| **Depends** | None for fake child tests |

---

## W1-D — ACP Client and Runtime Adapter

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-acp-adapter` *(create if missing)* |
| **Work item** | W1-D |
| **Owned** | `crates/tracer-acp-client/`, `crates/tracer-runtime-adapter/`, `packages/runtime-client/`, `tests/contract/acp/` |
| **Forbidden** | Own spawn; own SQLite; raw Grok events to UI; auto-approve |
| **First test** | Normalize initialize fixture; auth-required without session.ready |
| **Depends** | Soft: W1-B types; W1-G fake; W1-C I/O |

---

## W1-E — Storage and Session Persistence

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-storage` |
| **Work item** | W1-E |
| **Owned** | `crates/tracer-storage/`, `apps/desktop/src-tauri/migrations/`, `tests/integration/storage/` |
| **Forbidden** | ACP; UI; runtime-as-DB-writer |
| **First test** | Fresh DB + migration; ordered event reload; unknown payload preserved |
| **Depends** | Prefer W1-B ids/types |

---

## W1-F — Control Plane Integration

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-control-plane` *(create when Wave 1.2 starts)* |
| **Work item** | W1-F |
| **Owned** | `crates/tracer-control-plane/`, `crates/tracer-permissions/`, `apps/desktop/src-tauri/src/`, `tests/integration/control-plane/` |
| **Forbidden** | Duplicate adapter/process/storage logic; raw ACP to React; auto-approve |
| **First test** | VS-01 happy path via fake ACP + Tauri commands |
| **Depends** | Hard: W1-B, W1-C, W1-D, W1-E |

---

## W1-G — Fake Runtime and Contract Harness

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-fake-runtime` |
| **Work item** | W1-G |
| **Owned** | `tools/fake-acp-runtime/`, `packages/test-fixtures/`, portions of `tests/fixtures/`, `tests/contract/` |
| **Forbidden** | Provider calls; becoming second production runtime |
| **First test** | Fake NDJSON binary; `happy_prompt_stream` + `auth_required_session_new` |
| **Depends** | Align type names with W1-B |

---

## W1-H — HeliHarness Workspace Integration

| Field | Value |
|---|---|
| **Task id** | `tracer-w1-heli-integration` |
| **Matrix alias** | `tracer-w1-heliharness-integration` |
| **Work item** | W1-H |
| **Owned** | `docs/agent-workflows/`, `docs/modules/w1-h/`, `.heli/`, `crates/tracer-heli/` |
| **Forbidden** | Edit `.heli-harness` distribution; replace harness task semantics; product UI/runtime/storage |
| **First test** | Templates claimable; handoff format documented; target `tracer` checklist; `cargo test -p tracer-heli` |
| **Depends** | Gate 0 on main |

---

## Create missing tasks (coordinator)

```bash
# From parent workspace root — only if task not listed by `heli task list`
npx github:KJ-AIML/heli-harness task create tracer-w1-acp-adapter --work-item W1-D --repo tracer
npx github:KJ-AIML/heli-harness task create tracer-w1-control-plane --work-item W1-F --repo tracer
```
