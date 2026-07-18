# Desktop GUI Readiness (Gate 2.1)

## Classification

| Level | Description | Gate 2.1 status |
|---|---|---|
| **L0** | Command registration + typed invoke policy | **PASS** |
| **L1** | Desktop-boundary journeys via `plane_*` + control plane (fake ACP) | **PASS** |
| **L2** | Packaged app smoke (installer/bundle) | **NOT IN SCOPE / PARTIAL** |
| **L3** | Full WebView GUI E2E (tauri-driver / Playwright) | **NOT DONE** |

## Evidence

- `apps/desktop/src-tauri/tests/desktop_boundary_journey.rs` — 9 PASS including multi-session focus
- `apps/desktop/src/shared/commands/invoke.policy.test.ts` — fail-closed Tauri policy
- `node tools/tauri-e2e/run.mjs` — PASS with explicit `desktop-boundary-e2e` classification
- Commands: full `tracer_*` set + `tracer_presentation_focus`

## Honest non-claims

- This gate does **not** claim full WebView automation.
- Preferred GUI path is documented; blockers are tooling (tauri-driver + WebView2), not control-plane readiness.