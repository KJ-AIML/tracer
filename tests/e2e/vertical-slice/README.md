# Vertical-slice e2e (W1-F)

Rust-level vertical slice acceptance is covered by control-plane VS tests against fake ACP:

```bash
cargo test -p tracer-control-plane --test vs_scenarios
```

Desktop UI e2e (full Tauri window automation) is optional stretch for later gates; command surface is wired in `apps/desktop/src-tauri`.
