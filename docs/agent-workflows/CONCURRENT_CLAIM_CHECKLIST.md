# Concurrent Claim Checklist (Tracer)

Use this before any write work in the HeliHarness parent workspace.

## 0. Identity

- [ ] Parent workspace root contains `.heli-harness/HARNESS.md` (walk up from worktree if needed)
- [ ] Heli CLI available (`heli` or `npx github:KJ-AIML/heli-harness`)
- [ ] Assigned **Heli task id** known (created id, not only work-item letter)
- [ ] Assigned **git worktree** path is dedicated to this task (preferred)

## 1. Status before claim

From the task worktree (or workspace root):

```bash
npx github:KJ-AIML/heli-harness status
npx github:KJ-AIML/heli-harness conflicts
npx github:KJ-AIML/heli-harness task show <task-id>
```

- [ ] Task exists and is `active`
- [ ] No other active **write** lease on this task
- [ ] No other active **write** lease on this same worktree path
- [ ] Path-claim overlaps reviewed (if `owns` is populated)

## 2. Claim write lease

```bash
cd <task-worktree>
npx github:KJ-AIML/heli-harness task claim <task-id> --mode write --host <host-id> <task-worktree>
export HELI_SESSION_ID=<session-from-claim>   # PowerShell: $env:HELI_SESSION_ID = "..."
```

- [ ] Claim succeeded (record `session`, `lease`, `expires`)
- [ ] `HELI_SESSION_ID` exported in this shell/session
- [ ] Host id recorded (`grok-build`, `grok`, `claude`, …)

If claim fails with `WORKTREE_WRITER_HELD`: use a **separate worktree**, do not steal another task's cwd.

If claim fails with `LEASE_HELD` / `STALE_LEASE`: coordinate; only use `heli task takeover --confirm` with explicit authority.

## 3. Target

```bash
npx github:KJ-AIML/heli-harness target list
npx github:KJ-AIML/heli-harness target set tracer
```

- [ ] Target repo is `tracer` for product work (or intentional exception documented)
- [ ] Concurrent mode note: task target is source of truth; global `target.json` is advisory

## 4. Session verify

```bash
npx github:KJ-AIML/heli-harness session status
npx github:KJ-AIML/heli-harness status
```

- [ ] Session matches claim
- [ ] Mode is `write`
- [ ] Lease active
- [ ] Worktree path matches this agent’s tree
- [ ] Warnings about metadata/lease worktree drift understood (metadata may lag; lease wins)

## 5. Ownership fence

- [ ] Owned paths from readiness matrix / master plan restated
- [ ] Forbidden paths restated (including `.heli-harness` distribution)
- [ ] No shared write task across parallel agents

## 6. Finish sequence

- [ ] Local commits only (unless user explicitly authorizes push)
- [ ] Completion / handoff report written (see MODULE_HANDOFF_REPORT.md)
- [ ] Release lease:

```bash
npx github:KJ-AIML/heli-harness task release <task-id>
```

- [ ] Never leave a write lease held after the agent run ends when work is complete

## Dry-run acceptance (W1-H first test)

A template is claimable when an agent can:

1. Find the task id in WAVE1_TASK_TEMPLATES.md  
2. Execute this checklist without inventing harness semantics  
3. Confirm target `tracer`  
4. Produce a handoff report in the standard format  
