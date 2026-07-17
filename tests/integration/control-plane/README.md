# Control-plane integration tests

Primary VS-01…VS-14 suite lives in:

```text
crates/tracer-control-plane/tests/vs_scenarios.rs
```

Run:

```bash
cargo test -p tracer-control-plane --test vs_scenarios -- --test-threads=1
```

CI class: standard (fake ACP, temp SQLite, no network/credentials/live Grok).
