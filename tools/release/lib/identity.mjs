/**
 * Product identity consistency for Windows RC packaging.
 *
 * Canonical identity (W2.3-A):
 * - productName: Tracer
 * - package id / reverse-DNS: dev.tracer.desktop
 * - stable exe name: tracer-desktop(.exe)
 * - semver: aligned across tauri.conf.json, Cargo.toml, apps/desktop package.json
 * - app-data root (Windows): %LOCALAPPDATA%\dev.tracer.desktop\
 */

import { readFileSync, existsSync } from "node:fs";
import path from "node:path";
import {
  TAURI_CONF,
  DESKTOP_PACKAGE_JSON,
  CARGO_TOML,
  ICONS_DIR,
} from "./paths.mjs";

export const CANONICAL = Object.freeze({
  productName: "Tracer",
  identifier: "dev.tracer.desktop",
  mainBinaryName: "tracer-desktop",
  exeWindows: "tracer-desktop.exe",
  version: "0.1.0",
  /** Windows app-data root (Tauri identifier → LocalAppData). */
  appDataWindowsTemplate: "%LOCALAPPDATA%\\dev.tracer.desktop",
  /** Storage relative path when control plane uses platform app-data root. */
  storageRelative: "tracer\\tracer.db",
});

function parseCargoVersion(text) {
  // Prefer [package] version of tracer-desktop, not workspace deps.
  const m = text.match(
    /\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
  );
  return m ? m[1] : null;
}

function parseCargoName(text) {
  const m = text.match(/\[package\][\s\S]*?^name\s*=\s*"([^"]+)"/m);
  return m ? m[1] : null;
}

/**
 * Load and compare identity sources. Returns a structured report.
 * @returns {{ ok: boolean, version: string, checks: Array<object>, identity: object }}
 */
export function checkIdentity() {
  const checks = [];
  let ok = true;

  if (!existsSync(TAURI_CONF)) {
    checks.push({ id: "tauri.conf", status: "fail", message: "missing tauri.conf.json" });
    return { ok: false, version: null, checks, identity: CANONICAL };
  }

  const conf = JSON.parse(readFileSync(TAURI_CONF, "utf8"));
  const pkg = JSON.parse(readFileSync(DESKTOP_PACKAGE_JSON, "utf8"));
  const cargoText = readFileSync(CARGO_TOML, "utf8");
  const cargoVersion = parseCargoVersion(cargoText);
  const cargoName = parseCargoName(cargoText);

  const expect = (id, actual, expected, note) => {
    const pass = actual === expected;
    if (!pass) ok = false;
    checks.push({
      id,
      status: pass ? "pass" : "fail",
      actual,
      expected,
      note: note || null,
    });
  };

  expect("productName", conf.productName, CANONICAL.productName);
  expect("identifier", conf.identifier, CANONICAL.identifier);
  expect(
    "mainBinaryName",
    conf.mainBinaryName ?? cargoName,
    CANONICAL.mainBinaryName,
    "stable exe stem; keeps L2/L3 harness names stable",
  );
  expect("version.tauri", conf.version, CANONICAL.version);
  expect("version.package.json", pkg.version, CANONICAL.version);
  expect("version.Cargo.toml", cargoVersion, CANONICAL.version);
  expect("cargo.package.name", cargoName, "tracer-desktop");

  // Bundle posture for Windows RC
  const bundle = conf.bundle || {};
  checks.push({
    id: "bundle.active",
    status: bundle.active === true ? "pass" : "fail",
    actual: bundle.active,
    expected: true,
    note: "W2.3-A enables packaging; L2 harness still uses cargo binary",
  });
  if (bundle.active !== true) ok = false;

  const targets = bundle.targets;
  const targetList = Array.isArray(targets)
    ? targets
    : targets === "all"
      ? ["all"]
      : [targets];
  const hasNsis =
    targetList.includes("nsis") || targetList.includes("all");
  checks.push({
    id: "bundle.targets.nsis",
    status: hasNsis ? "pass" : "fail",
    actual: targets,
    expected: 'includes "nsis" (primary RC)',
  });
  if (!hasNsis) ok = false;

  // Icons
  const requiredIcons = [
    "icon.ico",
    "icon.png",
    "32x32.png",
    "128x128.png",
  ];
  for (const name of requiredIcons) {
    const p = path.join(ICONS_DIR, name);
    const present = existsSync(p);
    if (!present) ok = false;
    checks.push({
      id: `icon.${name}`,
      status: present ? "pass" : "fail",
      path: p,
    });
  }

  // Signing honesty: no thumbprint in config for development RC
  const thumb =
    bundle.windows && bundle.windows.certificateThumbprint
      ? bundle.windows.certificateThumbprint
      : null;
  checks.push({
    id: "signing.no_fake_thumbprint",
    status: thumb == null || thumb === "" ? "pass" : "warn",
    actual: thumb,
    note: "empty/null thumbprint required for UNSIGNED_DEVELOPMENT_RC; do not invent SIGNED",
  });

  return {
    ok,
    version: CANONICAL.version,
    checks,
    identity: {
      ...CANONICAL,
      resolved: {
        productName: conf.productName,
        identifier: conf.identifier,
        mainBinaryName: conf.mainBinaryName,
        versions: {
          tauri: conf.version,
          packageJson: pkg.version,
          cargo: cargoVersion,
        },
        bundleTargets: targets,
        certificateThumbprint: thumb,
      },
    },
  };
}
