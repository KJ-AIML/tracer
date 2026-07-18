# Wave 2.2.2 Test Matrix — WebView Tooling Integration

**Gate:** 2.2.2 · **Branch:** `integration/tracer-w2-webview-tooling` · **Date:** 2026-07-18  
**CI class:** standard — network no · credentials no · live Grok no · fake ACP yes  
**Platform-gated:** L2 smoke / L3-I (Windows GUI host) · Doctor READY on authoring host

## Level model (do not collapse)

| Level | Meaning | Gate 2.2.2 |
|---|---|---|
| L0 | Frontend invoke policy | PASS (standard CI) |
| L1 | Desktop boundary journey | PASS (standard CI) |
| L2 | Built application process launch smoke | **PASS** |
| L3-I | WebView driver infrastructure | **PASS** |
| L3-J | Full GUI product journey | **NOT_STARTED** |
| Doctor | Host/driver component readiness | **READY** |

## WebView tooling (W2.2-T integrated)

| # | Surface | Command | Classification | Notes |
|---|---|---|---|---|
| 1 | Setup plan | `node tools/tauri-driver/setup.mjs` | plan only | No install without opt-in |
| 2 | Setup apply | `--apply` or `TRACER_TAURI_E2E_SETUP=1` | opt-in | Not run during integration re-validation (used existing cache) |
| 3 | Doctor | `pnpm test:tauri-e2e:doctor` | **READY** | Critical components OK |
| 4 | L2 smoke | `pnpm test:tauri-e2e:l2` | **PASS** | Process ownership + cleanup |
| 5 | L3-I infra | `pnpm test:tauri-e2e:l3i` | **PASS** | Driver + WebView probes; not product journey |
| 6 | L3-J journey | — | **NOT_STARTED** | Future W2.2-B only |

### Doctor components (authoring host)

| Component | Status |
|---|---|
| TAURI_DRIVER | OK |
| EDGE_BROWSER | OK 150.0.4078.65 |
| WEBVIEW2_RUNTIME | OK 150.0.4078.65 |
| EDGE_DRIVER | OK 150.0.4078.65 exact match |
| APPLICATION_BINARY | OK debug |
| FRONTEND_DIST | OK |
| PORT_AVAILABILITY | OK 127.0.0.1:4444 |
| PROCESS_CLEANUP_CAPABILITY | OK |
| tauri_cli (optional) | MISSING (advisory; cargo-only path valid) |

### Compatibility rule

```text
major(msedgedriver) == major(Microsoft Edge)
observed: exact 150.0.4078.65
```

### L2 stages

| Stage | Result |
|---|---|
| frontend_build | PASS |
| backend_build | PASS |
| packaging_test_binary | PASS |
| app_launch | PASS |
| readiness | PASS |
| smoke | PASS |
| app_shutdown | PASS |
| orphan_verification | PASS |

### L3-I stages

| Stage | Result |
|---|---|
| frontend_build / backend_build | PASS |
| packaging_test_binary | PASS |
| driver_startup | PASS |
| app_launch (WebDriver session) | PASS |
| readiness (title Tracer, readyState complete) | PASS |
| smoke (root + Tauri internals surface) | PASS |
| app_shutdown | PASS |
| driver_shutdown | PASS |
| orphan_verification | PASS |

## Regression — presentation / multi-session / VS / soak / drain

| Suite | Count | Result |
|---|---|---|
| presentation_delivery | 19 | PASS |
| multi_session MS-01..17 | 17 | PASS |
| vs_scenarios | 23 | PASS |
| drain_lifecycle | 14 | PASS |
| session::lifecycle unit | 5 | PASS |
| stress_drain_lifecycle | 3 | PASS |
| tracer-vs1-soak | 8 | PASS |
| stress multi-session + sequential | 3 | PASS |
| desktop_boundary_journey | 9 | PASS |
| live-grok-smoke unit/dry | 24 | PASS (no live) |

## CI isolation evidence

| Command | GUI / drivers launched? | Result |
|---|---|---|
| `pnpm -r test` | **No** (L0+L1 only via `tools/tauri-e2e/run.mjs`) | PASS |
| `pnpm test:tauri-e2e` | No L2/L3-I | PASS |
| `pnpm test:tauri-e2e:l2` | App process only | PASS |
| `pnpm test:tauri-e2e:l3i` | tauri-driver + msedgedriver + app | PASS |

## Aggregate workspace

| Layer | Result |
|---|---|
| `pnpm install --frozen-lockfile` | PASS |
| `pnpm -r test` / `pnpm -r build` | PASS |
| `cargo fmt --all --check` | PASS |
| `cargo check --workspace` | PASS |
| `cargo test --workspace -- --test-threads=1` | PASS |
| `cargo clippy --workspace --all-targets` | PASS (warnings only) |
| Gate decision | **PASS** |

## Non-claims

- No L3-J product journey authored or executed  
- No W2.2-B task created/claimed  
- No live Grok  
- No automatic driver install in standard CI  