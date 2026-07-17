# Shared manifest edits (W1-D)

W1-D applied **minimal** root `Cargo.toml` registration (allowed for new members):

```toml
members += "crates/tracer-acp-client", "crates/tracer-runtime-adapter"
workspace.dependencies +=
  tracer-acp-client = { path = "crates/tracer-acp-client" }
  tracer-runtime-adapter = { path = "crates/tracer-runtime-adapter" }
```

No `package.json` / `pnpm-workspace` changes (optional `packages/runtime-client` TS bindings **not** added — Rust API is sufficient for W1-F).

No edits to forbidden crates/packages.
