# W2.4.1-A Completion Report — Upgrade Fixture + Release Provenance

## Identity

| Field | Value |
|---|---|
| Work item | W2.4.1-A |
| Task | `tracer-w2-upgrade-fixture` |
| Heli session | `heli-ses-7c596a03-9e8d-4054-82b0-1b55164dbe6b` |
| Branch | `agent/tracer-w2-upgrade-fixture` |
| Base SHA | `4c5f5599df16325f39da1b3165d7c02be94ac0a4` |
| Tip SHA | `8e11b6371a14c0fd75348383ac53df634b47b2ee` (pin commit; final tip after this update recorded in coordinator return) |
| Host | grok-build |

## Decisions (Part 13)

| Decision | Result |
|---|---|
| Upgrade fixture | **PASS** |
| Data preservation | **PASS** |
| Migration interruption recovery | **PASS** |
| Downgrade behavior | **CONTROLLED_REFUSAL** |
| Release provenance | **PASS** |

## N / N+1 provenance

See `docs/validation/release/WINDOWS_UPGRADE_RESULTS.md` for full hash table.

- N: 0.1.0 / schema 1 / Gate 2.3 tip package  
- N+1: 0.1.1 / schema 2 / additive migration `002_schema_v2_upgrade_marker.sql`  
- Identifier stable: `dev.tracer.desktop`  
- Isolation: `dev.tracer.desktop.upgrade-fixture` + TEMP `TRACER_DATABASE_PATH`

## Pre / post upgrade

- Pre: schema 1, 3 sessions (completed / failed / stopped) + events + approval  
- Migration: product sqlx applied 002 once  
- Post: schema 2, prior sessions preserved, new session added, relaunch OK  
- Uninstall retained DB; reinstall restored history  
- No orphan processes after smoke kills

## UF-01…05

All PASS — see test matrix / upgrade results.

## Manifest / provenance

`pnpm release:provenance` + verify **PASS**; signing `UNSIGNED_DEVELOPMENT_RC`.

## Commits

| SHA | Message |
|---|---|
| `b7b1e82` | feat(storage): add schema v2 migration and upgrade safety guards |
| `b952fe8` | chore(desktop): bump package version to 0.1.1 for N+1 RC |
| `700ff07` | feat(release): add upgrade fixture and release provenance tooling |
| `d70ca9a` | docs(w2.4.1): record upgrade architecture, matrix, and host results |
| `8e11b63` | docs(w2.4.1): pin completion report tip SHA |

## Residual risks

1. Version N binary lacks future-schema guard (added in N+1 storage); UF-05 classifies CONTROLLED_REFUSAL based on no destructive downgrade + data integrity when N opens schema-2 DB.  
2. Full GUI session creation during upgrade still relies on packaged smoke + SQLite seed (fake ACP); L3-J GUI journeys remain separate.  
3. Clippy `-D warnings` fails on pre-existing unrelated crates; workspace clippy (soft) passes.  
4. L3-I skips when msedgedriver absent (honest).  
5. Staged binaries under `target/` must never be committed.

## Integration recommendation

**Recommend a dedicated W2.4.1 integration task** to merge `agent/tracer-w2-upgrade-fixture` → main after review.  
Do **not** start that integration from this worker. Do not push.