/**
 * Project-local paths for WebView driver tooling (W2.2-T).
 * Paths are repo-relative; never hard-code machine usernames in reports.
 */

import path from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/** tools/tauri-driver */
export const TOOLS_DIR = path.resolve(__dirname, "..");
/** repo root (tracer worktree) */
export const REPO_ROOT = path.resolve(TOOLS_DIR, "../..");

/** Gitignored download/install cache */
export const CACHE_DIR = path.join(TOOLS_DIR, ".cache");
/** Optional local bin dir (also gitignored) */
export const BIN_DIR = path.join(TOOLS_DIR, "bin");

export const MSEDGEDRIVER_CACHE_DIR = path.join(CACHE_DIR, "msedgedriver");
export const VERSIONS_FILE = path.join(CACHE_DIR, "versions.local.json");

/**
 * Redact absolute home/user segments for inventory reports.
 * @param {string | null | undefined} p
 */
export function redactPath(p) {
  if (!p) return p ?? null;
  const home = os.homedir();
  let out = String(p);
  if (home) {
    const re = new RegExp(
      home.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
      "gi",
    );
    out = out.replace(re, "%USERPROFILE%");
  }
  out = out.replace(/([A-Za-z]:\\Users\\)[^\\/]+/gi, "$1<user>");
  out = out.replace(/(\/Users\/)[^/]+/g, "$1<user>");
  out = out.replace(/(\/home\/)[^/]+/g, "$1<user>");
  return out;
}

/**
 * Prefer env overrides, then project cache, then PATH.
 * @param {string} platform
 */
export function projectDriverCandidates(platform = process.platform) {
  const fromEnv = process.env.TRACER_NATIVE_DRIVER
    ? [process.env.TRACER_NATIVE_DRIVER]
    : [];
  if (platform === "win32") {
    return [
      ...fromEnv,
      path.join(BIN_DIR, "msedgedriver.exe"),
      path.join(MSEDGEDRIVER_CACHE_DIR, "msedgedriver.exe"),
      path.join(MSEDGEDRIVER_CACHE_DIR, "current", "msedgedriver.exe"),
    ];
  }
  if (platform === "linux") {
    return [
      ...fromEnv,
      path.join(BIN_DIR, "WebKitWebDriver"),
      path.join(CACHE_DIR, "WebKitWebDriver"),
    ];
  }
  return fromEnv;
}

export function projectTauriDriverCandidates() {
  const fromEnv = process.env.TRACER_TAURI_DRIVER
    ? [process.env.TRACER_TAURI_DRIVER]
    : [];
  return [
    ...fromEnv,
    path.join(BIN_DIR, process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver"),
    path.join(CACHE_DIR, process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver"),
  ];
}