# tauri-driver tools (W2.2-A)

Helpers for installing, detecting, and running [`tauri-driver`](https://crates.io/crates/tauri-driver) safely.

## Commands

```powershell
node tools/tauri-driver/print-setup.mjs
node tools/tauri-driver/doctor.mjs
node tools/tauri-driver/start-driver.mjs
```

## Install (manual / GUI runner)

```powershell
cargo install tauri-driver --locked
# Windows native driver:
# download msedgedriver matching Edge, or msedgedriver-tool
```

## Safety

- Prefer starting driver via `tools/tauri-e2e/l3i-infra.mjs` (owned process + orphan reap).
- `start-driver.mjs` is for interactive debugging; Ctrl+C kills the process tree on Windows.

## Related

- `tools/tauri-e2e/l3i-infra.mjs` — L3-I infrastructure smoke
- `docs/validation/tauri/TAURI_E2E_DOCTOR.md`
