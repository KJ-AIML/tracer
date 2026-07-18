# Drain lifecycle stress (W2.2-C)

Implemented as:

```text
tests/stress/src/stress_drain_lifecycle.rs
```

Registered on the existing `tracer-vs1-stress` package (no new workspace member).

```powershell
cargo test -p tracer-vs1-stress --test stress_drain_lifecycle -- --test-threads=1
```
