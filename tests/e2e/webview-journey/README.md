# WebView product journey pointers (W2.2-B + W2.3-C)

L3-J full GUI product journeys live under the Tauri E2E harness:

```text
pnpm test:tauri-e2e:gui
pnpm test:tauri-e2e:repeat-gui
tools/tauri-e2e/l3j-gui.mjs
tools/tauri-e2e/lib/journeys.mjs
tests/e2e/tauri/gui/README.md
```

Reliability (W2.3-C): free ports, state-based waits, Edge doctor resilience,
failure injection + sanitized artifacts, Windows GUI runner contract.

This folder is a **navigation contract** for integrators. Do not re-implement
session/prompt/approval by invoking control-plane `plane_*` handlers from the
harness — journeys must drive the WebView DOM.