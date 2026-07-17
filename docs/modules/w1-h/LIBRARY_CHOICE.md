# W1-H Library Path Choice

**Task:** `tracer-w1-heli-integration` (work item **W1-H**)  
**Date:** 2026-07-17  
**Decision:** use **`crates/tracer-heli/`** (Rust), not `packages/heli-client/`.

## Options considered

| Path | Pros | Cons |
|---|---|---|
| `crates/tracer-heli/` | Aligns with Wave 1 Rust crates (`tracer-domain`, process, storage, control-plane); zero JS runtime in product path; unit-testable with `cargo test` | Requires future workspace `Cargo.toml` membership (shared manifest request when integration lands) |
| `packages/heli-client/` | Familiar to web tooling | Diverges from Rust-first control plane; risk of dual clients |

## Decision

Ship a **read-only** Rust adapter under `crates/tracer-heli/`:

- workspace discovery by walking up for `.heli-harness/HARNESS.md` (no fixed absolute paths)
- task / session / lease / binding / conflict status types
- task-to-worktree projection (lease → session → binding → task metadata)
- safe missing-workspace behavior (`WorkspaceProbe`, `try_load_workspace_status`)

## Non-goals

- Mutating Heli workspace state (claim/release/takeover) — use installed `heli` CLI
- Editing parent `.heli-harness/` distribution assets
- Replacing Grok native subagent orchestration
- Product UI, runtime ACP, or session storage

## Manifest note

No root `Cargo.toml` is added by W1-H (forbidden without shared-manifest coordination). The crate is standalone and tested via:

```bash
cargo test --manifest-path crates/tracer-heli/Cargo.toml
```
