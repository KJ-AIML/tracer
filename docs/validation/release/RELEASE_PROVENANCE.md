# Release Provenance (W2.4.1)

## Distinctions

| Layer | Meaning |
|---|---|
| **Provenance** | product, version, schemaLogicalVersion, identifier, buildSourceSha, gateTipSha, platform, toolchain |
| **Integrity** | filename, sizeBytes, sha256 |
| **Signing** | Authenticode class (`UNSIGNED_DEVELOPMENT_RC` for local RC) |
| **Test evidence** | separate RC / upgrade JSON (not embedded in provenance) |

## SHA fields

| Field | Meaning |
|---|---|
| `buildSourceSha` | Immutable product/migration/tooling commit that produced the release bytes (`N+1_BUILD_SOURCE_SHA`) |
| `gateTipSha` | Tip of the gate branch after optional report-only commits |
| `sourceSha` | Backward-compatible alias of `buildSourceSha` |

## Generate / verify

```text
pnpm release:provenance
pnpm release:provenance:verify
```

Optional env overrides when regenerating after report commits:

```text
$env:TRACER_BUILD_SOURCE_SHA="<N+1_BUILD_SOURCE_SHA>"
$env:TRACER_GATE_TIP_SHA="<gate tip>"
```

Output (not committed): `target/release-rc/windows/provenance.json`  
Schema docs: `tests/fixtures/releases/provenance.schema.json`

## Rules

- No absolute developer home paths in manifests
- Binaries never committed
- Signing class recorded honestly; never invent SIGNED
- `checkIdentity` must pass (version drift across Cargo / Tauri / package.json fails provenance)
