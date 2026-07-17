# Tracer Agent Workflows (HeliHarness)

**Owner:** W1-H (`tracer-w1-heli-integration`)  
**Audience:** Wave 1+ coding agents operating in the Tracer HeliHarness parent workspace  
**Authority:** Parent `.heli-harness/HARNESS.md` remains the governance source of truth. These docs specialize it for Tracer; they do **not** replace harness semantics.

## Contents

| Document | Purpose |
|---|---|
| [CONCURRENT_CLAIM_CHECKLIST.md](./CONCURRENT_CLAIM_CHECKLIST.md) | Claim / session / lease / target preflight |
| [WAVE1_TASK_TEMPLATES.md](./WAVE1_TASK_TEMPLATES.md) | W1-A…W1-G claimable task templates |
| [MODULE_HANDOFF_REPORT.md](./MODULE_HANDOFF_REPORT.md) | Standard module completion / handoff format |
| [INTEGRATION_AGENT_CHECKLIST.md](./INTEGRATION_AGENT_CHECKLIST.md) | Integrator merge / conflict checklist |
| [CONTRACT_CHANGE_REQUEST.md](./CONTRACT_CHANGE_REQUEST.md) | Frozen contract change process |
| [RUNTIME_RESEARCH_TEMPLATE.md](./RUNTIME_RESEARCH_TEMPLATE.md) | Read-only runtime research task template |

## Library

Read-only status adapter: `crates/tracer-heli/` (see `docs/modules/w1-h/LIBRARY_CHOICE.md`).

```bash
cargo test --manifest-path crates/tracer-heli/Cargo.toml
```

## Task id note

| Work item | Readiness matrix task id | Created Heli task id (Wave 1.1) |
|---|---|---|
| W1-H | `tracer-w1-heliharness-integration` | **`tracer-w1-heli-integration`** |

Prefer the **created** Heli task id for `heli task claim|show|release`.

## Hard rules

1. One writer session per task; prefer one git worktree per parallel task.
2. Never edit parent `.heli-harness/` distribution assets from product tasks.
3. Never `git push` unless the user explicitly authorizes remote publish.
4. Export `HELI_SESSION_ID` after claim so hooks resolve the same session.
5. Target set `tracer` before write work unless the task is explicitly multi-repo.
