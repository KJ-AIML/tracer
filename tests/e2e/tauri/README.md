# Tauri E2E (W2.2-A infrastructure + Gate 2.1 boundary)

**Tasks:**  
- Gate 2.1 / W2-B: `tracer-w2-tauri-gui-e2e` (L0+L1 desktop-boundary)  
- W2.2-A: `tracer-w2-tauri-e2e-infrastructure` (doctor, L2 smoke, L3-I driver infra)

## Levels

| Level | Suite | Path / command |
|---|---|---|
| L0 | Invoke policy | `apps/desktop/src/shared/commands/invoke.policy.test.ts` |
| L1 | Desktop boundary journey | `apps/desktop/src-tauri/tests/desktop_boundary_journey.rs` |
| L2 | Packaged app launch smoke | `node tools/tauri-e2e/l2-smoke.mjs` |
| L3-I | WebView driver infrastructure | `node tools/tauri-e2e/l3i-infra.mjs` + `tests/e2e/webview-infrastructure/` |
| L3-J | Full GUI product journey | **DEFERRED** — not in this folder |

## Run

```powershell
# Doctor
node tools/tauri-e2e/doctor.mjs

# L0 + L1 (standard CI)
node tools/tauri-e2e/run.mjs

# L2 (GUI host / platform-gated)
node tools/tauri-e2e/l2-smoke.mjs

# L3-I (requires tauri-driver + msedgedriver on Windows)
node tools/tauri-e2e/l3i-infra.mjs
```

## Explicit non-claim

This tree does **not** host a full product GUI journey (create session → prompt → approval → history through DOM clicks). That is L3-J / future W2.2-B.

## Docs

- `docs/modules/w2-2-a/`
- `docs/modules/w2-b/`
- `docs/validation/tauri/TAURI_E2E_DOCTOR.md`
