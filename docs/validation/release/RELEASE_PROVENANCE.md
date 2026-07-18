# Release Provenance (W2.4.1-A)

## Distinctions

| Layer | Meaning |
|---|---|
| **Provenance** | product, version, sourceSha, tag, platform, toolchain |
| **Integrity** | filename, sizeBytes, sha256 |
| **Signing** | Authenticode class (`UNSIGNED_DEVELOPMENT_RC` for local RC) |
| **Test evidence** | separate RC / upgrade JSON (not embedded in provenance) |

## Generate / verify

```text
pnpm release:provenance
pnpm release:provenance:verify
```

Output (not committed): `target/release-rc/windows/provenance.json`  
Schema docs: `tests/fixtures/releases/provenance.schema.json`

## Rules

- No absolute developer home paths in manifests
- Binaries never committed
- Signing class recorded honestly; never invent SIGNED