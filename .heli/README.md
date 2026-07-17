# Tracer repository-local `.heli/` convention

**This directory is not the parent HeliHarness installation.**

| Location | Role |
|---|---|
| Parent workspace `.heli-harness/` | Governance source of truth (do not edit distribution assets from product tasks) |
| This repo `.heli/` | Lightweight, repo-local pointers for agents working inside `repos/tracer` / Tracer worktrees |

## Rules preserved from parent harness

1. Walk upward to find `.heli-harness/HARNESS.md` before inventing workspace roots.  
2. Claim durable tasks; bind sessions; hold write leases for writes.  
3. Prefer separate git worktrees per parallel task.  
4. Target `tracer` for product edits.  
5. Evidence-backed completion; no push without authorization.

## Pointers

- Workflows: `docs/agent-workflows/`
- Module notes: `docs/modules/w1-h/`
- Read-only status library: `crates/tracer-heli/`
- Parent concurrent discipline: master plan §21; `docs/agent-workflows/CONCURRENT_CLAIM_CHECKLIST.md`

## What must never live here

- Live session/lease copies from the parent harness  
- Secrets or host credentials  
- Forked copies of `.heli-harness` adapter plugins  

Agents may add small Tracer-specific notes under `.heli/` as long as they remain advisory and defer to the parent harness on conflict.
