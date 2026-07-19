# Release Distribution Checklist (Gate 2.3 Windows RC)

**Status:** Development RC ready for local distribution testing  
**Signing:** UNSIGNED_DEVELOPMENT_RC - **not** for production channels

## Ship contents (local artifacts)

- [x] NSIS installer `Tracer_0.1.0_x64-setup.exe`
- [x] Portable `tracer-desktop.exe`
- [x] Identity: Tracer / `dev.tracer.desktop` / version 0.1.0
- [x] SHA-256 recorded in Gate 2.3 docs
- [ ] Production Authenticode signature
- [ ] CI secret / cert pipeline
- [ ] Prior-version upgrade fixture (RC-03)
- [ ] MSI (intentionally not selected)
- [ ] macOS / Linux packages

## Operator install check

1. Run NSIS silent or interactive install  
2. Confirm `tracer-desktop.exe` under install dir  
3. Fake ACP smoke (RC-02) or L3-J on demand  
4. Uninstall via `uninstall.exe`  
5. Do not treat unsigned binary as production trust

## Do not

- Commit installers/binaries to git  
- Claim SIGNED without Authenticode evidence  
- Fold release packaging into `pnpm -r test`  
- Auto-run live Grok during distribution smoke

## W2.4.1 supersession note (additive; does not rewrite Gate 2.3 history)

Gate 2.3 recorded RC-03 as **PARTIAL / FIXTURE_LIMITED** (no prior package fixture). That historical classification remains accurate for Gate 2.3.

**Current RC-03 status after W2.4.1: PASS** — real N (0.1.0) → N+1 (0.1.1) NSIS upgrade proven on Gate 2.4.1 with data preservation. See `docs/integration/WAVE_2_4_1_INTEGRATION_REPORT.md` and `WINDOWS_UPGRADE_READINESS.md`.
