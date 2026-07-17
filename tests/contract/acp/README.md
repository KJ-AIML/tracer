# ACP contract tests (W1-D)

Primary executable coverage lives in:

- `crates/tracer-acp-client/tests/` — framing, codec, state machine
- `crates/tracer-runtime-adapter/tests/fake_scenarios.rs` — fake ACP end-to-end

Fixtures under `tests/fixtures/acp/` are shared read-only inputs.

Run:

```powershell
cargo test -p tracer-acp-client
cargo test -p tracer-runtime-adapter
```

Evidence class: **fake-runtime** / synthetic. No live Grok, network, or credentials.
