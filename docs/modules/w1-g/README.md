# W1-G — Fake Runtime and Contract Harness

**Task ID:** `tracer-w1-fake-runtime`  
**Owned paths:**

```text
tools/fake-acp-runtime/
packages/test-fixtures/
tests/contract/fake-runtime/
docs/modules/w1-g/
```

Additive only under `tests/fixtures/acp/` when needed (Gate 0 fixtures remain W0-B ownership).

## Deliverables

| Path | Role |
|---|---|
| `tools/fake-acp-runtime/` | Deterministic fake ACP NDJSON process (scenario driver) |
| `packages/test-fixtures/` | Catalog / expected-events / provenance loaders |
| `tests/contract/fake-runtime/` | Harness contract tests (no live Grok, no network) |
| `docs/modules/w1-g/` | Module docs and completion report |

## Quick start

```bash
# List scenarios
node tools/fake-acp-runtime/bin/fake-acp-runtime.js --list-scenarios

# Manual drive (example)
#   type NDJSON initialize / session/new / session/prompt on stdin
node tools/fake-acp-runtime/bin/fake-acp-runtime.js --scenario happy_prompt_stream

# Contract harness
node --test tests/contract/fake-runtime/*.test.js
# or
npm test --prefix tests/contract/fake-runtime
```

Env equivalents:

```text
TRACER_FAKE_ACP_SCENARIO=<id>
TRACER_FAKE_ACP_CHUNK_DELAY_MS=<n>
TRACER_FAKE_ACP_CANCEL_DELAY_MS=<n>
```

## Evidence separation

| Label | Source |
|---|---|
| `fake-runtime` | This fake binary |
| `synthetic` | Structural fixtures / vendor-unknown scenario |
| `live-scrubbed` | Gate 0 captures (auth-required wire shape mirrored) |
| `live-authenticated` | **Not** produced here; opt-in T6 only |

**Rule:** Never claim synthetic or fake-runtime streams as live multi-turn stock Grok parity.

## Catalog IDs

All `standardCi: true` ids from `tests/specifications/scenarios/catalog.yaml` are implemented. Live-only ids are rejected with exit code 2.

See [SCENARIO_DRIVER.md](./SCENARIO_DRIVER.md).

## Out of scope (forbidden)

- Live Grok / network / provider credentials in standard tests
- Production runtime code
- UI, storage, process manager ownership
- `tests/contract/event-protocol/` (W1-B)
- Root workspace manifests (request-only)

## Consumers

- **W1-D** runtime adapter: spawn this binary for T2 fake integration
- **W1-C** process manager: orphan / kill paths with crash & slow_cancel scenarios
- **W1-F** control plane: approval / cancel composition against fake
- **W1-B** event protocol: assert normative types using expected-events packs via `@tracer/test-fixtures`
