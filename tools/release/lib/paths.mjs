/**
 * Path constants for Windows release packaging (W2.3-A).
 */

import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export const REPO_ROOT = path.resolve(__dirname, "../../..");
export const DESKTOP_DIR = path.join(REPO_ROOT, "apps/desktop");
export const SRC_TAURI = path.join(DESKTOP_DIR, "src-tauri");
export const TAURI_CONF = path.join(SRC_TAURI, "tauri.conf.json");
export const DESKTOP_PACKAGE_JSON = path.join(DESKTOP_DIR, "package.json");
export const CARGO_TOML = path.join(SRC_TAURI, "Cargo.toml");
export const ICONS_DIR = path.join(SRC_TAURI, "icons");
export const FAKE_ACP_JS = path.join(
  REPO_ROOT,
  "tools/fake-acp-runtime/bin/fake-acp-runtime.js",
);

/** Release binary (portable) candidates under cargo target. */
export function releaseBinaryCandidates() {
  const names =
    process.platform === "win32"
      ? ["tracer-desktop.exe", "Tracer.exe"]
      : ["tracer-desktop", "Tracer"];
  const roots = [
    path.join(REPO_ROOT, "target/release"),
    path.join(SRC_TAURI, "target/release"),
  ];
  const out = [];
  for (const root of roots) {
    for (const name of names) {
      out.push(path.join(root, name));
    }
  }
  return out;
}

/** Bundle output roots after `tauri build`. */
export function bundleRoots() {
  return [
    path.join(REPO_ROOT, "target/release/bundle"),
    path.join(SRC_TAURI, "target/release/bundle"),
  ];
}

export function nsisBundleDirs() {
  return bundleRoots().map((r) => path.join(r, "nsis"));
}

export function msiBundleDirs() {
  return bundleRoots().map((r) => path.join(r, "msi"));
}

/** Default artifact staging under target/ (not committed). */
export function releaseStageDir() {
  return path.join(REPO_ROOT, "target/release-rc/windows");
}
