# Full GUI Journey Results (L3-J / W2.2-B)

**Task:** `tracer-w2-webview-gui-journey`  
**Host:** Windows GUI (msedgedriver + tauri-driver)  
**Date:** 2026-07-18  
**Command:** `pnpm test:tauri-e2e:gui` / `node tools/tauri-e2e/l3j-gui.mjs --skip-build`

## Environment

| Item | Value |
|---|---|
| Edge / msedgedriver | 150.0.4078.65 (compatible) |
| tauri-driver | present (cargo bin) |
| App binary | `target/debug/tracer-desktop.exe` |
| Fake ACP | `tools/fake-acp-runtime/bin/fake-acp-runtime.js` |
| Isolation | temp SQLite via `--tracer-e2e-env=` file; empty heli probe |
| Live Grok / network / credentials | **no** |

## L3-J overall

| Field | Result |
|---|---|
| **L3-J decision** | **PASS** |
| Journeys | **12/12 PASS** |
| Orphans after teardown | none |

## Per-journey results

| ID | Result | Notes |
|---|---|---|
| GJ-01 | **PASS** | Tauri mode; `tracer-app-ready` |
| GJ-02 | **PASS** | Register path + create session (happy_prompt_stream) |
| GJ-03 | **PASS** | Streaming events in timeline |
| GJ-04 | **PASS** | Allow clears approval card |
| GJ-05 | **PASS** | Deny clears approval card |
| GJ-06 | **PASS** | Cancel while approval pending (no deadlock) |
| GJ-07 | **PASS** | Two-session focus switch via GUI |
| GJ-08 | **PASS** | Crash/EOF reflected in session UX |
| GJ-09 | **PASS** | Restart same temp DB restores history |
| GJ-10 | **PASS** | Heli unavailable non-fatal |
| GJ-11 | **PASS** | Fail-closed; backend remains tauri |
| GJ-12 | **PASS** | Clean shutdown; orphan verify stage green |

## Harness notes

1. **Env injection:** `tauri:options.env` is not reliable on this host. Desktop accepts `--tracer-e2e-env=<file>` (dotenv KEY=VALUE) for temp DB / fake ACP / heli / readiness marker.
2. **Prompt concurrency:** GUI submit does not hold a global busy lock across the blocking CP prompt RPC so approval/cancel remain clickable.
3. **Artifacts:** `artifacts/tauri-e2e/<run-id>/` (gitignored) on failure.

## Independent of L0–L3-I

This L3-J PASS does not substitute for L0/L1 standard CI or L2/L3-I platform gates. Those remain separately executable.
