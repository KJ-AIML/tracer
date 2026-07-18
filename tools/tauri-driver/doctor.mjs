#!/usr/bin/env node
/**
 * Driver-stack doctor (tauri-driver + native WebDriver only).
 */

import { spawnSync } from "node:child_process";

function which(bin) {
  const cmd = process.platform === "win32" ? "where.exe" : "which";
  const r = spawnSync(cmd, [bin], {
    encoding: "utf8",
    shell: process.platform === "win32",
    windowsHide: true,
  });
  if (r.status !== 0) return null;
  return (r.stdout || "").split(/\r?\n/).map((s) => s.trim()).find(Boolean) || null;
}

const tauriDriver = which("tauri-driver");
const msedgedriver = which("msedgedriver");
const webkit = which("WebKitWebDriver");
const nativeEnv = process.env.TRACER_NATIVE_DRIVER || null;

const platform = process.platform;
let classification = "READY";
const issues = [];

if (platform === "darwin") {
  classification = "UNSUPPORTED_PLATFORM";
  issues.push("external tauri-driver unsupported on macOS");
} else if (!tauriDriver) {
  classification = "DRIVER_UNAVAILABLE";
  issues.push("tauri-driver missing — cargo install tauri-driver --locked");
} else if (platform === "win32" && !msedgedriver && !nativeEnv) {
  classification = "DRIVER_UNAVAILABLE";
  issues.push("msedgedriver missing (or set TRACER_NATIVE_DRIVER)");
} else if (platform === "linux" && !webkit && !nativeEnv) {
  classification = "DRIVER_UNAVAILABLE";
  issues.push("WebKitWebDriver missing (or set TRACER_NATIVE_DRIVER)");
}

const report = {
  schemaVersion: 1,
  module: "W2.2-A",
  component: "tauri-driver",
  classification,
  platform,
  tauriDriver,
  msedgedriver,
  webkitWebDriver: webkit,
  TRACER_NATIVE_DRIVER: nativeEnv,
  issues,
  portDefault: Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444),
};

console.log(JSON.stringify(report, null, 2));
process.exitCode =
  classification === "READY" || classification === "DRIVER_UNAVAILABLE" ? 0 : 2;
