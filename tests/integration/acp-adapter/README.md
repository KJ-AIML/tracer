# ACP adapter integration

Integration scenarios are implemented as Rust tests in
`crates/tracer-runtime-adapter/tests/fake_scenarios.rs` so they share the
workspace dependency graph without a second Cargo package.

Scenarios driven by `tools/fake-acp-runtime` (W1-G).
