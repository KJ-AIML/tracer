# W2.4.2 Signing Architecture

**Work item:** W2.4.2-A Authenticode Signing Readiness  
**Product:** Tracer (`dev.tracer.desktop`)  
**Date:** 2026-07-19  
**Baseline:** `d83a873f0cbad9478ee311315e53f6ca22035970` (Gate 2.4.1 PASS)

> Gate 2.4.1 Heli footnote: integration session `heli-ses-26b01af7` is correct for that gate (initial claim `heli-ses-ee781bf9`, then takeover). This W2.4.2-A session is separate.

## Modes

| Mode | Purpose | Production claim |
|---|---|---|
| `UNSIGNED` | Default RC / internal testers | None |
| `SELF_SIGNED_TEST` | Pipeline mechanics proof only | Never trusted distribution |
| `TRUSTED_AUTHENTICODE` | Real publisher / managed signing | Requires org cert + explicit auth |

## Recommended approach (Tracer)

**Near-term (this wave):** Windows SDK `signtool` + PowerShell Authenticode for self-signed mechanics; certificate material via protected secret store / thumbprint — **not** committed PFX.

**Production target (when org cert exists):** Organization OV/EV code-signing certificate in hardware-backed token or managed cloud signing (e.g. Azure Trusted Signing / AzureSignTool), invoked only from an explicit release workflow with `TRACER_SIGNING_AUTHORIZED=1` and `TRACER_RELEASE_SIGNING_WORKFLOW=1`.

| Approach | Key exposure | CI | Timestamp | Rotation | Cost / complexity | Tauri/NSIS |
|---|---|---|---|---|---|---|
| Windows cert store + thumbprint | Low if non-exportable | Medium | Explicit URL | Manual | Low | Compatible |
| PFX via secret store | Medium (file at rest) | High if vaulted | Explicit URL | Manual | Medium | Compatible |
| Hardware token | Lowest | Harder (agent access) | Explicit URL | Device policy | Higher | Compatible |
| Managed/cloud signing | Lowest | Best | Provider | Provider | Subscription | Compatible |

**Not claimed:** access to a production CA, EV enrollment, or Azure Trusted Signing account.

## Tooling (detect, don't assume)

Wrappers under `tools/release/signing/` detect:

- `signtool.exe` (Windows SDK paths)
- `AzureSignTool` (optional)
- PowerShell `Set-AuthenticodeSignature` / `Get-AuthenticodeSignature`

SHA-256 digests for file integrity and signing hash algorithm. Timestamping is opt-in via `TRACER_TIMESTAMP_URL` (no silent unbounded retries).

## Commands

```text
pnpm release:windows          # build RC (default unsigned)
pnpm release:sign:doctor      # non-destructive readiness
pnpm release:sign:test        # self-signed mechanics (copies + cleanup)
pnpm release:sign             # UNSIGNED no-op, or TRUSTED with auth+cert
pnpm release:verify-signature
pnpm release:provenance
```

## Secret-handling

See `docs/validation/release/SIGNING_SECRET_HANDLING.md` (10 rules). Never commit PFX/P12/passwords/tokens.

## Provenance

Per-artifact fields include `artifactSha256`, `preSignSha256`, `postSignSha256`, `signaturePresent`, `signatureStatus`, certificate metadata, timestamp fields, `signingMode`, `signingTool`, `signingToolVersion`. Unsigned artifacts emit explicit `signaturePresent: false` / `signingMode: UNSIGNED`.

## Ownership

Owned: `tools/release/signing/`, release CLIs, `tests/release/windows/signing/`, docs under `docs/modules/w2-4-2/` and `docs/validation/release/`, minimal Tauri `digestAlgorithm`, root release scripts.  
Not owned: storage, runtime, control plane, desktop UI, provider.
