# W1-H — HeliHarness Workspace Integration

| Field | Value |
|---|---|
| **Work item** | W1-H |
| **Heli task id (created)** | `tracer-w1-heli-integration` |
| **Readiness matrix id** | `tracer-w1-heliharness-integration` (alias / naming drift — use **created** id) |
| **Mode** | write |
| **Primary target** | `tracer` |
| **Library** | `crates/tracer-heli/` (see [LIBRARY_CHOICE.md](./LIBRARY_CHOICE.md)) |

## Owned paths

```text
docs/agent-workflows/
docs/modules/w1-h/
.heli/                          # repo-local harness convention notes (not parent distribution)
crates/tracer-heli/             # read-only status adapter
```

Optional parent-level resources (coordinator-approved only; not required for this delivery):

```text
resources/prompts/
resources/reports/heliharness/
```

## Forbidden

- Editing parent workspace `.heli-harness/` distribution assets
- Replacing HeliHarness task semantics or Grok native subagent orchestration
- Product UI / runtime / storage implementation
- Mutating Heli state from the product adapter (CLI remains authoritative for writes)
- Root manifests unless requested via `SHARED_MANIFEST_REQUESTS.md`

## Deliverables

1. **Read-only adapter** — `crates/tracer-heli`
2. **Wave 1 claim / handoff templates** — `docs/agent-workflows/`
3. **Repo-local `.heli/` convention** — pointers that preserve parent harness rules
4. **Fixtures + deterministic tests** — `crates/tracer-heli/tests/`

## Verification

```bash
cargo test --manifest-path crates/tracer-heli/Cargo.toml
```

## Related docs

- Master plan §15 Agent W1-H, §21 concurrent task usage
- `docs/integration/WAVE_1_READINESS_MATRIX.md` § W1-H
- Parent `.heli-harness/HARNESS.md`
