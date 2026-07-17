# Tracer Desktop Shell (W1-A)

Tauri 2 + React + TypeScript **startup shell** for Tracer.

## Scope

- Application chrome, navigation, session workspace **placeholders**
- Presentation containers: empty, loading, running, approval, failed, disconnected, completed, cancelled
- Accessibility baseline: status **text + icon**, never color-only
- **Mock store only** — no ACP parsing, no real control plane

## Non-goals (this module)

- Feature bodies for timeline / approvals / changes / terminal
- SQLite migrations (`src-tauri/migrations/` → **W1-E**)
- `tracer_*` command implementation and `tracer://events` stream → **W1-F**
- Full IDE chrome

## Prerequisites

- Node 20+ and pnpm 9+
- Optional for native window: Rust stable + [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

Root monorepo manifests (`package.json`, `pnpm-workspace.yaml`, workspace `Cargo.toml`) are **not** owned by W1-A. See `docs/modules/w1-a/SHARED_MANIFEST_REQUESTS.md`.

Until shared manifests land, install from this package directory (file dependency on `packages/ui`).

## Commands

```bash
# From apps/desktop (after packages/ui is present)
pnpm install
pnpm test
pnpm build
pnpm dev          # Vite only on http://localhost:1420

# Native shell (requires Tauri toolchain)
pnpm tauri:dev
pnpm tauri:build
```

```bash
# Optional Rust check of the W1-A stub
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

## Mock navigation

1. **Projects** — list / empty state
2. Open demo project → **Sessions**
3. Open session → **Session workspace** shell (status chips, runtime pill, side tabs, composer rules)
4. **Presentation states** — gallery of all presentation containers
5. Header mock controls on session view force STATE_MATRIX statuses

## Contract bindings (names only)

| Surface | Contract |
|---|---|
| Commands | `docs/contracts/TAURI_COMMAND_CONTRACT_V1.md` via `src/shared/commands/invoke.ts` |
| Events channel | `tracer://events` |
| Status labels | `docs/ux/SESSION_SCREEN_SPEC.md`, `STATE_MATRIX.md` |

Temporary types marked `REPLACE_WHEN_W1B_EVENT_TYPES_AVAILABLE` / `REPLACE_WHEN_W1F_CONTROL_PLANE_AVAILABLE`.

## Handoff

- **W1-F:** replace mock invoke, register real commands, event subscription, deep control plane in `src-tauri`
- **W1-B:** replace temporary event/session types with `@tracer/event-types`
- **W1-E:** own `src-tauri/migrations/` only when control plane needs them

See `docs/modules/w1-a/W1-F_HANDOFF.md`.
