# W1-B Shared Manifest Requests

**Task:** `tracer-w1-domain-events`  
**Owner:** W1-B Domain and Event Protocol  
**Date:** 2026-07-17

W1-B **must not** edit root workspace manifests. Integrator / W1-H (or designated
workspace owner) should apply the following when composing the monorepo.

## Cargo workspace (`Cargo.toml` at repo root)

Create or extend root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/tracer-domain",
  # other wave-1 crates as they land…
]

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["serde", "v4"] }
thiserror = "2"
time = { version = "0.3", features = ["serde", "formatting", "parsing", "macros"] }
```

Optional: promote `tracer-domain` deps to `[workspace.dependencies]` and reference
with `workspace = true` inside `crates/tracer-domain/Cargo.toml`.

**Until then:** the crate is runnable standalone:

```bash
cd crates/tracer-domain && cargo test
```

## JavaScript workspace

Create root `package.json` / `pnpm-workspace.yaml` (or extend) to include:

```yaml
packages:
  - "packages/*"
  - "apps/*"
```

Register package name: `@tracer/event-types` → `packages/event-types`.

**Until then:**

```bash
cd packages/event-types && npm install && npm test
```

## CI notes

- T0/T1: `cargo test -p tracer-domain` (after workspace wiring) or path-based `cargo test --manifest-path crates/tracer-domain/Cargo.toml`
- T1 TS: `pnpm --filter @tracer/event-types test` (after workspace wiring)
- Contract fixtures live at `tests/contract/event-protocol/fixtures/` and are
  loaded by both Rust and TS tests via relative paths from package roots.

## Non-requests (explicit)

- Do **not** add `tracer-domain` dependencies on process/storage/ACP crates.
- Do **not** put ACP wire types in `@tracer/event-types`.