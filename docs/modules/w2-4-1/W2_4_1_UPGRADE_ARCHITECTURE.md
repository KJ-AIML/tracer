# W2.4.1 Upgrade Architecture

**Task:** `tracer-w2-upgrade-fixture`  
**Work item:** W2.4.1-A  
**Branch:** `agent/tracer-w2-upgrade-fixture`  
**Base SHA:** `4c5f5599df16325f39da1b3165d7c02be94ac0a4` (Gate 2.3 PASS)

## Ownership

**OWN:** `tools/release/`, `tests/release/windows/`, `tests/fixtures/releases/`, `docs/modules/w2-4-1/`, `docs/validation/release/`, minimal version metadata, minimal storage migration tests.

**DO NOT:** redesign domain/process/runtime/control-plane/desktop UI/live-provider; signing; live Grok; cross-platform; IDE; ALMS; plugins.

## Version N and N+1

| Field | Version N | Version N+1 |
|---|---|---|
| Semver | `0.1.0` | `0.1.1` |
| Source SHA | `4c5f5599df16325f39da1b3165d7c02be94ac0a4` | branch tip after this task |
| Schema logical version | `1` | `2` |
| App identifier | `dev.tracer.desktop` | `dev.tracer.desktop` (stable) |
| Fixture isolation id | `dev.tracer.desktop.upgrade-fixture` | same (test-only path namespace) |
| Artifact types | portable + NSIS | portable + NSIS |

**Strategy:** Build genuine Windows packages from Gate 2.3 tip as N (schema 1, semver 0.1.0), then bump patch + additive migration `002` for N+1. Not the same package installed twice.

**Data root compatibility:** Product identifier stays `dev.tracer.desktop`. Upgrade fixtures always use isolated `TRACER_DATABASE_PATH` under `%TEMP%\tracer-upgrade-fixture-*` so they never collide with the operator's `%LOCALAPPDATA%\dev.tracer.desktop\`.

## Migration model

- `001_init.sql` — schema logical version 1  
- `002_schema_v2_upgrade_marker.sql` — additive bump to logical version 2 + `upgrade_marker_w2_4_1`  
- sqlx bookkeeping is idempotent; each migration runs transactionally (UF-02)  
- `refuse_unsupported_future_schema` refuses DB advertising version > binary (UF-01 / UF-05)  
- Corrupt DB open fails with diagnostics; file is not silently replaced (UF-03)

## Safety matrix

| Concern | Fixture posture |
|---|---|
| Runtime | fake ACP only (`TRACER_FAKE_ACP_JS`) |
| Database | isolated file SQLite via `TRACER_DATABASE_PATH` |
| Network | no |
| Credentials | no |
| Live Grok | no |

## Commands

```text
pnpm release:windows
pnpm test:release:upgrade
pnpm test:release:windows -- --skip-build
# with prior N NSIS:
#   $env:TRACER_RC_PRIOR_NSIS="target/release-rc/upgrade-fixture/vN/Tracer_0.1.0_x64-setup.exe"
pnpm release:provenance
pnpm release:provenance:verify
```

## Provenance vs integrity vs signing vs tests

| Layer | What it proves | Artifact |
|---|---|---|
| Provenance | product/version/sourceSha/toolchain | `provenance.json` |
| Integrity | sizeBytes + sha256 | same manifest `artifacts[]` |
| Signing | Authenticode class | `signing.class` (UNSIGNED_DEVELOPMENT_RC allowed) |
| Test evidence | RC / upgrade results | `rc-validation.json`, `upgrade-fixture-results.json` |

No absolute developer home paths are embedded in manifests.