# Shared manifest requests (W1-A)

W1-A **does not own** root workspace manifests. The following are requested for a coordinator / shared-manifest owner so monorepo install and CI match the master plan acceptance commands.

## Requested root files

### `package.json` (repo root)

```json
{
  "name": "tracer",
  "private": true,
  "packageManager": "pnpm@9.15.0",
  "scripts": {
    "build": "pnpm -r run build",
    "test": "pnpm -r run test",
    "typecheck": "pnpm -r run typecheck"
  }
}
```

### `pnpm-workspace.yaml`

```yaml
packages:
  - "apps/*"
  - "packages/*"
```

### `Cargo.toml` (workspace)

```toml
[workspace]
resolver = "2"
members = [
  "apps/desktop/src-tauri",
  # W1-B…W1-F crates join as they land:
  # "crates/tracer-domain",
  # "crates/tracer-process",
  # "crates/tracer-storage",
  # "crates/tracer-control-plane",
]
```

### `.npmrc` (optional)

```text
shamefully-hoist=false
strict-peer-dependencies=false
```

## Owned packages already provided by W1-A

| Path | Name | Notes |
|---|---|---|
| `packages/ui` | `@tracer/ui` | Design tokens + StatusChip / RuntimePill / presentation |
| `apps/desktop` | `@tracer/desktop` | Vite+React shell + minimal Tauri stub |

Until root manifests exist, install with:

```bash
pnpm --dir packages/ui install
pnpm --dir packages/ui test
pnpm --dir apps/desktop install
pnpm --dir apps/desktop test
pnpm --dir apps/desktop build
```

## Do not add (wrong owners)

| Path | Owner |
|---|---|
| `packages/event-types` | W1-B |
| `packages/test-fixtures` | test/fixture owners |
| `apps/desktop/src-tauri/migrations/` | W1-E |
| `crates/*` | W1-B…W1-F as assigned |

## Version pins used by W1-A (guidance)

- React 18.3.x
- Vite 6.x
- TypeScript 5.7.x
- Vitest 3.x
- Tauri 2.x (apps/desktop/src-tauri)
