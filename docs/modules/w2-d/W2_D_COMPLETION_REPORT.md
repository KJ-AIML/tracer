# W2-D Completion Report — Live Approval Validation

**Work item:** W2-D  
**Heli task:** `tracer-w2-live-approval-validation`  
**Branch:** `agent/tracer-w2-live-approval-validation`  
**Base:** `56715cc79047d22e4c66a2a8ba257ee7b68d1f3e`  
**Date:** 2026-07-18  

## 1. Outcome

Delivered an **opt-in** live approval reverse-request suite (LVA-01…LVA-07) on top of the VS1-H1 `live-grok-smoke` harness. Dry-run and unit tests are green and CI-safe. Live suite was executed with operator auth present; reverse-request-dependent scenarios are classified honestly as **`NOT_OBSERVED`** (no fabricated PASS). Cancel/terminal/shutdown paths **`PASS`**. Overall live classification: **`PARTIAL`**.

## 2. Owned paths touched

```text
tools/live-grok-smoke/                 # approval.rs + CLI + evidence suite kind
tests/live/grok/                       # LVS policy update
tests/live/grok/approval/              # LVA manual test policy
docs/validation/live-grok/             # LIVE_APPROVAL_VALIDATION.md
docs/modules/w2-d/                     # this report
```

## 3. Deliverables

| Deliverable | Path |
|---|---|
| Live approval validation | `docs/validation/live-grok/LIVE_APPROVAL_VALIDATION.md` |
| Completion | `docs/modules/w2-d/W2_D_COMPLETION_REPORT.md` |
| Harness LVA suite | `tools/live-grok-smoke/src/approval.rs` |
| CLI | `approval-dry-run` / `approval-run` in `tools/live-grok-smoke` |
| Manual test policy | `tests/live/grok/approval/README.md` |

## 4. Harness design

### Commands

| Command | Env | Spawns agent stdio? |
|---|---|---|
| `approval-dry-run` | none | No |
| `approval-run` | `TRACER_LIVE_GROK=1` (or `TRACER_LIVE_SMOKE=1`) | Yes |

### Scenarios

| ID | Intent | Live result (2026-07-18) |
|---|---|---|
| LVA-01 | reverse-request observed | **NOT_OBSERVED** |
| LVA-02 | accept once (allow-once) | **NOT_OBSERVED** |
| LVA-03 | reject once (reject-once) | **NOT_OBSERVED** |
| LVA-04 | cancel while approval pending | **NOT_OBSERVED** |
| LVA-05 | no deadlock | **PASS** |
| LVA-06 | terminal session state | **PASS** |
| LVA-07 | clean shutdown (no orphan) | **PASS** |

### Safety

- Dual gate: env + explicit `approval-run` subcommand  
- Never auto-approve: `resolve_approval` only for LVA-02/03 scenario actions  
- Never claim RR PASS without observed `approval.requested`  
- Public-safe default inducing prompt; secret-looking `--prompt` rejected  
- Sanitized JSON evidence; credentials never printed  
- Standard CI must not invoke live (`matrix.yaml` already forbids `live` on standard jobs)

### Product reuse

- `tracer_runtime_adapter::grok_stdio_spawn_config`  
- `RuntimeAdapter::{start, initialize, create_session, submit_prompt, resolve_approval, cancel_prompt, shutdown, …}`  
- Normalize path already maps `session/request_permission` → `approval.requested` (never auto-approve)

No ACP / control-plane / process-manager / desktop redesign.

## 5. Validation performed

```text
cargo test -p live-grok-smoke
# 24 passed

cargo run -p live-grok-smoke -- approval-dry-run --out target/live-grok-smoke/approval-dry-run.json
# classification: NOT_RUN; LVA-01…07 NOT_RUN; spawnPlan.matchesW0bW1d=true

$env:TRACER_LIVE_GROK = "1"
cargo run -p live-grok-smoke --release -- approval-run --out target/live-grok-smoke/approval-live.json
# classification: PARTIAL (see §6)
```

Local evidence under `target/live-grok-smoke/` is **not committed**.

## 6. Live result classification (authoring host)

**Date:** 2026-07-18  
**Platform:** windows-x86_64  
**Runtime:** `grok 0.2.103 (89c3d36fb6)` via PATH  
**Auth:** session/new succeeded (`auth_state=not_required`; methods `cached_token`, `grok.com` — tokens not printed)  
**Caps:** `approvals=true`, `cancellation=true`, `promptStreaming=true`

### Stages

| Stage | Status | Notes |
|---|---|---|
| discovery | pass | binary + version |
| startup | pass | process_alive |
| initialize | pass | protocol_ready; approvals/cancellation advertised |
| auth_requirement | pass | methods listed without tokens |
| session | pass | session_ready |
| prompt | pass | 3 approval-inducing attempts |
| stream | pass | normalized event types observed |
| approval | pass (stage) | **no** `approval.requested` — honesty path, not fabricated PASS |
| cancel | pass | cancel-while-pending path attempted; within deadlock budget |
| shutdown | pass | no orphan |

### Scenarios

| ID | Status | Detail |
|---|---|---|
| LVA-01 | **NOT_OBSERVED** | `approval.requested` not observed within wait budget |
| LVA-02 | **NOT_OBSERVED** | reverse-request not observed; decision path not exercised |
| LVA-03 | **NOT_OBSERVED** | reverse-request not observed; decision path not exercised |
| LVA-04 | **NOT_OBSERVED** | reverse-request not observed; decision path not exercised |
| LVA-05 | **PASS** | no deadlock: prompt/control returned within budget |
| LVA-06 | **PASS** | terminal `session.cancelled` observed |
| LVA-07 | **PASS** | shutdown leaves no orphan process |

### Overall

| Mode | Classification |
|---|---|
| Unit tests + `approval-dry-run` | **`NOT_RUN`** (construction Pass) |
| Live `approval-run` | **`PARTIAL`** |
| Live RR decision paths (LVA-01…04) | **`NOT_OBSERVED`** — **do not claim reverse-request PASS** |

### Observed sanitized event types (live)

```text
runtime.process.started
session.created
runtime.process.ready
adapter.protocol.unknown
session.ready
session.cancelled
runtime.process.exited
```

No `approval.requested` / `approval.resolved` on this host with the public inducing prompt and stock session defaults. Runtime advertises approval capability (`caps.approvals=true`) but the live inducing turn did not surface `session/request_permission` within the observation budget.

### Interpretation

Harness plumbing for observe / allow-once / reject-once / cancel-while-pending / shutdown is in place and exercised. **Live reverse-request parity is not proven on this host** and must not be overstated. Operators may re-run with alternate public-safe `--prompt` or updated stock agent policy; classifications will remain honest.

## 7. Assumptions

1. W1-D adapter APIs are the correct product surface for stock spawn + approval resolve/cancel.  
2. W0-B / VS1-H1 stock path (`grok agent --no-leader stdio`) remains the spawn contract.  
3. Standard CI continues to exclude live jobs; unit tests only cover dry-run/classification.  
4. Credentials remain in operator environment only (`GROK_HOME` / stock login).  
5. `NOT_OBSERVED` is preferred over fabricated PASS when RR is absent.

## 8. Risks

| Risk | Mitigation |
|---|---|
| Operators treat dry-run as live RR proof | `NOT_RUN` classification + docs |
| Operators treat PARTIAL as full LVA PASS | Per-scenario matrix + honesty notes |
| Accidental CI live spend | Env gate + CI matrix forbids live on standard jobs |
| Auto-approve regressions | Resolve only on explicit LVA-02/03 actions |
| Inducing prompt never triggers tools | Document `NOT_OBSERVED` / `UNSUPPORTED_BY_PROMPT`; allow `--prompt` override |

## 9. Integration requirements

1. Keep `tools/live-grok-smoke` as workspace member (already registered).  
2. Do **not** wire `approval-run` into standard CI.  
3. Optional manual/nightly may set `TRACER_LIVE_GROK=1` with secrets from a secret store.  
4. Control plane / desktop may later call the same adapter APIs; this harness is not a product dependency.

## 10. Forbidden work (confirmed not done)

- Enabling live Grok in standard CI  
- Control-plane redesign  
- ACP adapter redesign  
- Process-manager redesign  
- Desktop product work  
- Committed credentials / tokens / private prompts  
- Push to remote  

## 11. Commit SHAs

| Item | SHA |
|---|---|
| Base | `56715cc79047d22e4c66a2a8ba257ee7b68d1f3e` |
| Harness + docs (this delivery) | `205246fa83a4bcc0363b7035bafa5d132b0a0c47` (harness); `4350f05d492e5408c1afefdd855d4adf82c16966` (docs) |
