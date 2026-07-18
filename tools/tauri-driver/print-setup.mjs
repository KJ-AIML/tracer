#!/usr/bin/env node
/**
 * Print setup instructions for tauri-driver + native WebDriver.
 * Does not install (network policy of standard CI: no).
 */

const platform = process.platform;

console.log("=== tauri-driver setup (W2.2-A) ===");
console.log(`platform: ${platform}`);
console.log("");
console.log("1) Install tauri-driver:");
console.log("   cargo install tauri-driver --locked");
console.log("");

if (platform === "win32") {
  console.log("2) Windows: Microsoft Edge Driver (msedgedriver)");
  console.log("   - Match Edge version to driver version (mismatch causes hangs)");
  console.log("   - https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/");
  console.log("   - Or: cargo install --git https://github.com/chippers/msedgedriver-tool");
  console.log("         msedgedriver-tool");
  console.log("   - Place msedgedriver.exe on PATH, or set TRACER_NATIVE_DRIVER");
  console.log("");
  console.log("3) WebView2 Runtime (Evergreen) must be installed for the app WebView");
} else if (platform === "linux") {
  console.log("2) Linux: WebKitWebDriver");
  console.log("   - Debian/Ubuntu: webkit2gtk-driver (package name may vary)");
  console.log("   - which WebKitWebDriver");
} else if (platform === "darwin") {
  console.log("2) macOS: external tauri-driver is NOT supported (no WKWebView driver tool).");
  console.log("   Future path: WebdriverIO @wdio/tauri-service with embedded driver + plugins.");
  console.log("   W2.2-A L3-I uses the external driver path only → UNSUPPORTED_PLATFORM on macOS.");
} else {
  console.log("2) Unsupported platform for Tauri desktop driver automation.");
}

console.log("");
console.log("4) Verify:");
console.log("   node tools/tauri-e2e/doctor.mjs");
console.log("   node tools/tauri-driver/doctor.mjs");
console.log("");
console.log("5) Run L3-I (after L2 binary exists):");
console.log("   node tools/tauri-e2e/l3i-infra.mjs");
console.log("");
console.log("Docs: https://v2.tauri.app/develop/tests/webdriver/manual-setup/");
