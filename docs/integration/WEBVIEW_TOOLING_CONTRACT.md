# WebView Tooling Contract (Gate 2.2.2 integrated)

**Authority:** Gate 2.2.2 integration on main  
**Implements:** W2.2-T plan/apply tooling + doctor + L3-I harness  
**Supersedes posture from Gate 2.2.1:** L3-I moves from `BLOCKED_BY_TOOLING` to **PASS** when drivers present; doctor from `DRIVER_UNAVAILABLE` to **READY** on provisioned hosts.

## 1. Scope

| In contract | Out of contract |
|---|---|
| Opt-in plan/apply driver provisioning | Silent install in product CI |
| Edge major ↔ msedgedriver compatibility | Committing driver binaries |
| Doctor component matrix + classifications | Desktop product redesign |
| L2 process smoke | L3-J DOM product journeys |
| L3-I WebDriver infrastructure smoke | Creating/claiming W2.2-B |
| Root script aliases for doctor/L2/L3-I/setup | IDE/ALMS/plugins/marketplace features |

## 2. Level definitions (normative)

```text
L0   Frontend invoke policy — standard CI
L1   Desktop command-boundary journey — standard CI
L2   Built application process launch smoke — platform-gated
L3-I WebView driver infrastructure — platform-gated; PASS requires executable evidence
L3-J Full GUI product journey — NOT_STARTED until product-authorized W2.2-B
```

**Claim rules**

1. **Doctor READY** only when all critical components are OK (optional `tauri_cli` excluded).
2. **L3-I PASS** only with: driver start → WebDriver session → readiness → infra smoke → teardown → no orphans.
3. Missing tooling → `DRIVER_UNAVAILABLE` / `BLOCKED_BY_TOOLING` — never false product PASS.
4. **L3-J** must not be claimed from tooling work alone.

## 3. Provisioning contract

| Mode | Authorization | Side effects |
|---|---|---|
| **plan** (default) | always | Inventory + planned actions; **no** download/install |
| **apply** | `--apply` **or** `TRACER_TAURI_E2E_SETUP=1` | `cargo install tauri-driver --locked`; download msedgedriver into **gitignored** project cache |

### Paths

| Role | Location |
|---|---|
| Setup entry | `tools/tauri-driver/setup.mjs` |
| Edge/driver helpers | `tools/tauri-driver/lib/{edge,install,paths}.mjs` |
| Project cache (gitignored) | `tools/tauri-driver/.cache/` |
| Project bin (gitignored) | `tools/tauri-driver/bin/` |
| Doctor | `tools/tauri-e2e/doctor.mjs` |
| L2 | `tools/tauri-e2e/l2-smoke.mjs` |
| L3-I | `tools/tauri-e2e/l3i-infra.mjs` |

### Compatibility (Windows)

```text
major(msedgedriver) MUST equal major(installed Microsoft Edge)
```

- Prefer exact full-version match when served by Microsoft endpoints.
- File presence alone is insufficient — version must parse via `--version`.
- Codes: `EDGE_DRIVER_COMPATIBLE` | `EDGE_DRIVER_NOT_FOUND` | `EDGE_DRIVER_VERSION_MISMATCH` | `EDGE_DRIVER_VERSION_UNVERIFIED` | `EDGE_BROWSER_VERSION_UNKNOWN`

## 4. Doctor components

| Id | Meaning |
|---|---|
| `TAURI_DRIVER` | tauri-driver binary resolve + version |
| `EDGE_BROWSER` | Microsoft Edge (Windows) |
| `WEBVIEW2_RUNTIME` | WebView2 Evergreen runtime |
| `EDGE_DRIVER` | msedgedriver (Windows) / WebKitWebDriver (Linux) |
| `APPLICATION_BINARY` | `tracer-desktop` cargo artifact |
| `FRONTEND_DIST` | `apps/desktop/dist` |
| `PORT_AVAILABILITY` | default `127.0.0.1:4444` |
| `PROCESS_CLEANUP_CAPABILITY` | taskkill/tasklist (or kill) |

Statuses: `OK | MISSING | MISMATCH | UNVERIFIED | IN_USE | UNKNOWN | N/A`

## 5. Root scripts (integrated)

```json
"test:tauri-e2e": "node tools/tauri-e2e/run.mjs",
"test:tauri-e2e:doctor": "node tools/tauri-e2e/doctor.mjs",
"test:tauri-e2e:l2": "node tools/tauri-e2e/l2-smoke.mjs",
"test:tauri-e2e:l3i": "node tools/tauri-e2e/l3i-infra.mjs",
"test:tauri-e2e:setup": "node tools/tauri-driver/setup.mjs"
```

`pnpm -r test` must **not** invoke L2/L3-I or apply setup.

## 6. CI classes

| Class | Surfaces |
|---|---|
| `standard_ci` | L0, L1, cargo/pnpm unit and integration without GUI drivers |
| `windows_gui_runner` / `platform_gated_ci` | Doctor, L2, L3-I on Windows hosts with drivers |
| `manual_local` | Authoring host plan/apply + full stack |

## 7. Binary & hygiene rules

1. Never commit `msedgedriver`, `tauri-driver` binaries, zips, or `.cache/` contents.
2. Redact absolute home/user paths in reports (`%USERPROFILE%` / `<user>`).
3. No credentials, API keys, or fixed personal usernames in tracked files.
4. Process cleanup after L2/L3-I is mandatory (orphan verification).

## 8. Related documents

| Doc | Role |
|---|---|
| `docs/modules/w2-2-tooling/*` | Worker architecture, setup, matrix, completion |
| `docs/validation/tauri/WEBVIEW_DRIVER_READINESS.md` | Operator readiness evidence |
| `docs/integration/WAVE_2_2_2_INTEGRATION_REPORT.md` | Integration PASS report |
| `docs/integration/WAVE_2_2_2_TEST_MATRIX.md` | Integrated test matrix |
| `docs/integration/W2_2_B_LAUNCH_AUTHORIZATION_FINAL.md` | W2.2-B tooling YES / start NO |
| `docs/integration/W2_2_B_ENTRY_CRITERIA.md` | Gate 2.2.1 entry criteria (historical) |