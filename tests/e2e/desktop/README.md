# Desktop E2E (VS1-H2 scaffold)

Full Playwright/Tauri e2e is **not** required for VS1-H2 acceptance.

Deterministic coverage lives in:

```text
apps/desktop/src/shared/store/snapshotStore.test.ts
```

using the mock command backend (no network, no credentials).

## Future e2e (integration / Wave later)

Suggested journey under Tauri + fake ACP:

1. App open → snapshot load
2. Project register/list
3. Session create
4. Prompt + stream
5. Approval allow/deny/cancel
6. History restore after reopen
7. Heli missing non-fatal

Do not include live Grok credentials in standard CI.