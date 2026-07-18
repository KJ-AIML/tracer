# Drain lifecycle integration tests (W2.2-C)

Primary suite lives in-crate to avoid workspace member registration:

```text
crates/tracer-control-plane/tests/drain_lifecycle.rs
```

Run:

```powershell
cargo test -p tracer-control-plane --test drain_lifecycle -- --test-threads=1
```

Evidence class: fake ACP + temp SQLite (file or memory). No live Grok.
