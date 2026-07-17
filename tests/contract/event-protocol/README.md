# Event Protocol Contract Tests (W1-B)

Fixtures and expectations for Tracer Event Protocol v1.

## Ownership

- **Task:** `tracer-w1-domain-events` (W1-B)
- **Normative contract:** `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`
- **Rust types:** `crates/tracer-domain`
- **TypeScript types:** `packages/event-types`

## Fixtures (`fixtures/`)

| File | Kind | Purpose |
|---|---|---|
| `happy_prompt_stream.json` | stream | ready → prompt → deltas → completed → exit |
| `tool_with_approval.json` | stream | tool + approval.requested/resolved |
| `unknown_vendor_notification.json` | single | `adapter.protocol.unknown` + vendor metadata |
| `protocol_error.json` | single | `adapter.protocol.error` / ProtocolParseError |
| `cancel_mid_tool.json` | stream | cancel while tool active |
| `unexpected_process_exit.json` | stream | crash mid-run honesty |

All fixtures use synthetic UUIDs and relative paths only (no machine-specific absolute paths).

## How to run

### Rust (standalone crate — no root workspace required yet)

```bash
cd crates/tracer-domain
cargo test
```

### TypeScript package

```bash
cd packages/event-types
npm install
npm test
```

## Shared manifest note

Root `Cargo.toml` / `pnpm-workspace.yaml` are **not** edited by W1-B. See
`docs/modules/w1-b/SHARED_MANIFEST_REQUESTS.md` for integrator wiring.