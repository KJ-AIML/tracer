# Shared Manifest Request — W1-C Process Manager

**Task:** `tracer-w1-process-manager`  
**Requester session:** `heli-ses-ed0f4270-dfd9-400a-b9db-1c86a30f6c3a`  
**Date:** 2026-07-17  
**Status:** request only (W1-C does not edit root workspace manifests)

## Requested root `Cargo.toml` additions

When the Tracer workspace root Cargo manifest is introduced or updated by the authorized owner:

```toml
[workspace]
members = [
  # ... existing / peer wave crates ...
  "crates/tracer-process",
]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"
rust-version = "1.75"

# optional: shared dep versions
[workspace.dependencies]
thiserror = "2"
uuid = { version = "1", features = ["v4", "serde"] }
```

## Optional future domain coupling

W1-C currently defines local `ProcessErrorClass` wire strings aligned with
`RUNTIME_ADAPTER_CONTRACT_V1` so it can land without a hard path dependency on
`tracer-domain` (W1-B parallel). A later integration pass may:

```toml
# crates/tracer-process/Cargo.toml
[dependencies]
tracer-domain = { path = "../tracer-domain", optional = true }
```

Not required for W1-C acceptance.

## Not requested

- No changes under `repos/grok-build`
- No hard-coded machine Grok paths in workspace config
- No SQLite / UI package entries from this task
