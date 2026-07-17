# Test specifications (Wave 0)

Machine-readable **acceptance and scenario specifications** for Tracer’s vertical slice.

These are **not** executable test suites. Wave 1 implements tests that consume these files.

## Layout

```text
tests/specifications/
├── README.md                 # this file
├── scenarios/
│   └── catalog.yaml          # scenario ids, tiers, provenance, wire hooks
├── expected-events/
│   └── *.json                # ordered W0-A event type expectations
└── ci/
    └── matrix.yaml           # standard vs optional CI jobs
```

## Normative rules

1. Event `type` strings must match `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md` (W0-A).
2. Do not use W0-B conceptual names as product types.
3. Every scenario declares `evidence` provenance (`synthetic` | `live-scrubbed` | `fake-runtime` | `live-authenticated` | `unit-generated`).
4. Paths in examples use placeholders (`{{PROJECT_ROOT}}`), never machine-absolute paths.
5. No credentials or private prompts.

## Related docs

- `docs/testing/TEST_STRATEGY.md`
- `docs/testing/VERTICAL_SLICE_ACCEPTANCE.md`
- `docs/testing/FAILURE_MATRIX.md`
- `tests/fixtures/acp/` (wire fixtures; owned by W0-B)

## Wire fixtures vs specifications

| Tree | Owner | Content |
|---|---|---|
| `tests/fixtures/acp/` | W0-B | Sanitized ACP JSON-RPC captures / synthetic wire frames |
| `tests/specifications/` | W0-D | Product acceptance: scenarios, expected **normalized** events, CI tiers |

Implementers map wire fixtures → normalizer → assert against `expected-events/`.
