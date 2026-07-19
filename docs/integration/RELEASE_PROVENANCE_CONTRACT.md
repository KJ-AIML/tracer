# Release Provenance Contract (Gate 2.4.1)

**Gate:** 2.4.1  
**Decision:** **PASS**  
**Date:** 2026-07-19  
**Schema:** `tests/fixtures/releases/provenance.schema.json`

## Required fields

| Field | Meaning |
|---|---|
| `product` | Tracer |
| `version` | package semver (0.1.1 for this gate) |
| `platform` | `windows-x64` |
| `artifactType` | `portable` / `nsis` |
| `filename` | artifact basename |
| `sizeBytes` | exact byte length |
| `sha256` | lowercase hex digest |
| `buildSourceSha` | immutable product/tooling commit (`N+1_BUILD_SOURCE_SHA`) |
| `gateTipSha` | gate tip including report-only commits |
| `schemaLogicalVersion` | storage logical schema (`2`) |
| `identifier` | `dev.tracer.desktop` |
| `signing.class` | `UNSIGNED_DEVELOPMENT_RC` (allowed) |
| `buildToolchain` | rustc/cargo/node/pnpm/os |
| test refs | separate RC / upgrade JSON (not embedded secrets) |

## Distinctions

| Layer | Proves |
|---|---|
| Provenance | identity + buildSourceSha + toolchain |
| Integrity | sizeBytes + sha256 |
| Signing | Authenticode classification only |
| Test evidence | upgrade / RC result files |

## Rules

1. `buildSourceSha` must not change after product freeze; report-only commits may advance `gateTipSha`.
2. No absolute developer home paths in manifests.
3. `checkIdentity()` must pass; version drift fails generation.
4. Verify is deterministic: `pnpm release:provenance` then `pnpm release:provenance:verify`.

## This gate

| Key | Value |
|---|---|
| `buildSourceSha` | `e04f81f5089d0414ef8967b0d98384d7b199b9b7` |
| N+1 portable sha256 | `e530bae51a42f81d213e59dcd72680c14efd3814956e4fbbafb715f296acf4f2` |
| N+1 NSIS sha256 | `5ca4452b974070bcb47dc21734d995abaeb502ad1f5354391e7fc79bf5ba5e2a` |
| Signing | `UNSIGNED_DEVELOPMENT_RC` |