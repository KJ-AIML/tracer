# Windows Release Validation

## W2.3-A RC scenarios

```bash
pnpm release:windows
pnpm test:release:windows
```

## W2.4.1-A upgrade fixture

```bash
# Requires staged version N under target/release-rc/upgrade-fixture/vN/
pnpm test:release:upgrade
# or reuse already-built N+1:
node tools/release/upgrade-fixture.mjs --skip-build-n1
```

## Provenance

```bash
pnpm release:provenance
pnpm release:provenance:verify
```

Evidence JSON (not committed): `target/release-rc/windows/*.json`