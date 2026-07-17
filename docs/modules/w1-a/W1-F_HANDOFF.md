# Handoff: W1-A Desktop Shell → W1-F Control Plane

## What W1-A delivered

1. **Frontend shell** under `apps/desktop/src/`:
   - `AppShell`, primary nav (Projects / About / presentation gallery)
   - Projects home + project workspace placeholders
   - Session workspace **layout** placeholder (header, banners, timeline pane, side tabs, composer, footer)
   - Error boundary + global status banners
   - Mock store driving STATE_MATRIX statuses
2. **UI kit** under `packages/ui/` (StatusChip, RuntimePill, Banner, PresentationContainer, tokens)
3. **Minimal Tauri 2 stub** under `apps/desktop/src-tauri/`:
   - Window config, default capability, `app_shell_info` only
   - **No** migrations directory
   - **No** `tracer_*` command registration
   - **No** process/adapter/storage composition

## What W1-F should own next

| Item | Action |
|---|---|
| Command surface | Register contract names from `TAURI_COMMAND_CONTRACT_V1.md` (`tracer_project_*`, `tracer_session_*`, `tracer_approval_*`, `tracer_events_list`, `tracer_runtime_status`, `tracer_app_info`) |
| Invoke wrapper | Replace mock path in `apps/desktop/src/shared/commands/invoke.ts` (`REPLACE_WHEN_W1F_CONTROL_PLANE_AVAILABLE`) |
| Event stream | Subscribe UI to `tracer://events` (single + batch envelopes) |
| `src-tauri/src/lib.rs` | Compose crates from W1-B…W1-E; remove temporary `app_shell_info` or map to `tracer_app_info` |
| Permissions | Wire `tracer_approval_resolve` fail-closed; never auto-allow |
| Auth gate | Surface control-plane auth errors without React parsing raw ACP |
| Mock store | Keep for storybook/tests; default production path must not invent backend behavior |

## Hard constraints (do not regress)

1. UI consumes **normalized** Tracer events only (ADR-002) — no raw vendor/ACP frames in React.
2. Status never color-only — keep `@tracer/ui` StatusChip / RuntimePill text labels.
3. Do not show session **Running** after `runtime.process.exited` / failed.
4. Do not expand shell into full IDE (file tree / multi-editor primary surface).
5. Do not claim ownership of `src-tauri/migrations/` (W1-E).
6. Paths in UI/runtime data stay user-local; no machine-specific absolutes in fixtures.

## Suggested first integration slice

1. `tracer_app_info` + `tracer_project_list` against fake/storage backends  
2. Session create → status events → StatusChip  
3. Prompt submit → event stream → timeline mount point (feature module)  
4. Approval resolve path  

## Files to touch carefully

- `apps/desktop/src-tauri/src/lib.rs` — primary W1-F expansion point  
- `apps/desktop/src/shared/commands/invoke.ts` — frontend bind  
- `apps/desktop/src/shared/store/mockStore.ts` — keep as dev/demo, gate behind flag  
- Prefer **not** rewriting `packages/ui` status catalog unless contract labels change  
