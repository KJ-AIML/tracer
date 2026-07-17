# ACP fixtures (sanitized)

Fixtures for Tracer’s ACP client/adapter tests and contract validation.

## Policy

- **No credentials, tokens, API keys, cookies, or auth codes.**
- **No private user prompts** from real sessions.
- **No fixed machine absolute paths** (use placeholders like `/workspace/project` or `{{PROJECT_ROOT}}`).
- **No real hostnames, usernames, or home directories.**
- Live captures must be scrubbed before commit.

## Provenance

| File | Origin |
|---|---|
| `initialize-request.json` | Canonical client request used in recon |
| `initialize-response.json` | Live `grok 0.2.102` response, scrubbed |
| `session-new-auth-required.json` | Live unauthenticated `session/new` error |
| `session-prompt-stream.jsonl` | Synthetic stream (not live model output) |
| `permission-request.json` | Synthetic reverse-request shape |
| `cancel-notification.json` | Synthetic cancel |

Live recon used hermetic `GROK_HOME` and did not complete authenticated prompt turns (auth required). Streaming/permission fixtures are structural, derived from upstream ACP types + Grok shell source.

## Source pin

- Upstream tree: `repos/grok-build` `SOURCE_REV=2ec0f0c8488842da03a71eeee3c61154957ca919`
- Probed binary: `grok 0.2.102`
