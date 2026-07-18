# WebView product journey pointers (W2.2-B)

L3-J full GUI product journeys live under the Tauri E2E harness:

```text
pnpm test:tauri-e2e:gui
tools/tauri-e2e/l3j-gui.mjs
tools/tauri-e2e/lib/journeys.mjs
tests/e2e/tauri/gui/README.md
```

This folder is a **navigation contract** for integrators. Do not re-implement
session/prompt/approval by invoking control-plane `plane_*` handlers from the
harness — journeys must drive the WebView DOM.
