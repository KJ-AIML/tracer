#!/usr/bin/env node
/**
 * tauri-driver + msedgedriver setup (W2.2-T).
 *
 * Plan mode (default): inventory + planned actions, NO install/download.
 * Apply mode (opt-in): cargo install tauri-driver + project-local msedgedriver.
 *
 * Apply authorization:
 *   TRACER_TAURI_E2E_SETUP=1
 *   or --apply
 *
 * Never commits binaries. Cache under tools/tauri-driver/.cache/ (gitignored).
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import {
  detectEdgeVersionWindows,
  downloadMsEdgeDriver,
  evaluateEdgeDriverCompatibility,
  readMsEdgeDriverVersion,
  resolveLocalMsEdgeDriver,
  readRecordedVersions,
} from "./lib/edge.mjs";
import {
  installTauriDriver,
  resolveTauriDriverPath,
  readTauriDriverVersion,
} from "./lib/install.mjs";
import { redactPath } from "./lib/paths.mjs";

const args = new Set(process.argv.slice(2));
const wantApply =
  args.has("--apply") ||
  process.env.TRACER_TAURI_E2E_SETUP === "1" ||
  process.env.TRACER_TAURI_E2E_SETUP === "true";
const jsonOut = args.has("--json");
const skipTauriDriver = args.has("--skip-tauri-driver");
const skipEdgeDriver = args.has("--skip-edge-driver");

function whichMsEdgeDriver() {
  if (process.platform !== "win32") return null;
  const r = spawnSync("where.exe", ["msedgedriver.exe"], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 8_000,
  });
  if (r.status !== 0 || !r.stdout) return null;
  const first = r.stdout
    .split(/\r?\n/)
    .map((s) => s.trim())
    .find(Boolean);
  return first && existsSync(first) ? first : null;
}

function inventory() {
  const platform = process.platform;
  const edge =
    platform === "win32"
      ? detectEdgeVersionWindows()
      : {
          available: false,
          path: null,
          version: null,
          major: null,
          method: "n/a",
        };

  const tauriPath = resolveTauriDriverPath();
  const tauri = readTauriDriverVersion(tauriPath);

  let nativePath =
    resolveLocalMsEdgeDriver() ||
    process.env.TRACER_NATIVE_DRIVER ||
    whichMsEdgeDriver() ||
    null;
  const native = readMsEdgeDriverVersion(nativePath);
  const compatibility =
    platform === "win32"
      ? evaluateEdgeDriverCompatibility(edge, {
          ...native,
          path: nativePath,
        })
      : {
          compatible: platform !== "win32",
          code: platform === "linux" ? "USE_WEBKIT_WEBDRIVER" : "N/A",
          message: "msedgedriver applies on Windows only",
        };

  const actions = [];
  if (!tauri.available) {
    actions.push({
      id: "install_tauri_driver",
      command: "cargo install tauri-driver --locked",
      reason: "TAURI_DRIVER_NOT_FOUND",
    });
  }
  if (platform === "win32" && !compatibility.compatible) {
    actions.push({
      id: "download_msedgedriver",
      command:
        "node tools/tauri-driver/setup.mjs --apply  # cache: tools/tauri-driver/.cache/",
      reason: compatibility.code,
      detail: compatibility.message,
    });
  }

  return {
    mode: wantApply ? "apply" : "plan",
    platform,
    edge: { ...edge, path: redactPath(edge.path) },
    tauriDriver: {
      available: tauri.available,
      version: tauri.version,
      path: redactPath(tauri.path),
    },
    edgeDriver: {
      available: Boolean(native.available),
      version: native.version,
      major: native.major,
      path: redactPath(native.path),
      compatibility,
    },
    recorded: readRecordedVersions(),
    plannedActions: actions,
    applyAuthorized: wantApply,
    notes: [
      "Apply never modifies system-wide PATH permanently.",
      "Drivers land in user cargo bin and/or tools/tauri-driver/.cache|bin (gitignored).",
      "Do not commit downloaded binaries.",
      "Compatibility rule: major(msedgedriver) == major(Edge).",
    ],
  };
}

async function apply(plan) {
  const results = [];
  if (!skipTauriDriver && !plan.tauriDriver.available) {
    const r = installTauriDriver({ copyToProjectBin: true });
    results.push({ step: "install_tauri_driver", ...r });
    if (!r.ok) {
      return { ok: false, results, blocker: r };
    }
  } else {
    results.push({
      step: "install_tauri_driver",
      ok: true,
      skipped: true,
      message: plan.tauriDriver.available
        ? "already present"
        : "skipped by flag",
    });
  }

  if (process.platform === "win32" && !skipEdgeDriver) {
    const edge = detectEdgeVersionWindows();
    const nativePath =
      resolveLocalMsEdgeDriver() ||
      process.env.TRACER_NATIVE_DRIVER ||
      whichMsEdgeDriver();
    const native = readMsEdgeDriverVersion(nativePath);
    const compat = evaluateEdgeDriverCompatibility(edge, {
      ...native,
      path: nativePath,
    });
    if (compat.compatible) {
      results.push({
        step: "download_msedgedriver",
        ok: true,
        skipped: true,
        message: compat.message,
        path: redactPath(nativePath),
      });
    } else {
      const r = await downloadMsEdgeDriver(edge);
      results.push({ step: "download_msedgedriver", ...r, path: redactPath(r.path) });
      if (!r.ok) {
        return { ok: false, results, blocker: r };
      }
    }
  }

  return { ok: true, results };
}

async function main() {
  const before = inventory();
  let applyResult = null;

  if (wantApply) {
    console.log("[setup] APPLY mode authorized (TRACER_TAURI_E2E_SETUP or --apply)");
    applyResult = await apply(before);
  } else {
    console.log("[setup] PLAN mode (no install). Re-run with --apply or TRACER_TAURI_E2E_SETUP=1 to provision.");
  }

  const after = inventory();
  const report = {
    schemaVersion: 1,
    module: "W2.2-T",
    component: "tauri-driver-setup",
    before,
    apply: applyResult,
    after: wantApply ? after : undefined,
    ready:
      after.tauriDriver.available &&
      (process.platform !== "win32" ||
        after.edgeDriver.compatibility?.compatible === true),
  };

  if (jsonOut) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log("");
    console.log(`mode: ${before.mode}`);
    console.log(
      `tauri-driver: ${after.tauriDriver.available ? after.tauriDriver.version + " @ " + after.tauriDriver.path : "MISSING"}`,
    );
    if (process.platform === "win32") {
      console.log(
        `edge: ${after.edge.available ? after.edge.version : "MISSING"}`,
      );
      console.log(
        `msedgedriver: ${
          after.edgeDriver.available
            ? `${after.edgeDriver.version} @ ${after.edgeDriver.path}`
            : "MISSING"
        }`,
      );
      console.log(
        `compatibility: ${after.edgeDriver.compatibility?.code} — ${after.edgeDriver.compatibility?.message}`,
      );
    }
    if (before.plannedActions.length && !wantApply) {
      console.log("");
      console.log("Planned actions:");
      for (const a of before.plannedActions) {
        console.log(`  - [${a.reason}] ${a.command}`);
        if (a.detail) console.log(`      ${a.detail}`);
      }
    }
    if (applyResult) {
      console.log("");
      console.log(`apply ok: ${applyResult.ok}`);
      for (const s of applyResult.results || []) {
        console.log(
          `  - ${s.step}: ${s.ok ? "ok" : "FAIL"}${s.skipped ? " (skipped)" : ""} ${s.message || s.code || ""}`,
        );
      }
      if (applyResult.blocker) {
        console.log(
          `blocker: ${applyResult.blocker.code} — ${applyResult.blocker.message}`,
        );
      }
    }
    console.log("");
    console.log(`driver stack ready: ${report.ready}`);
  }

  if (wantApply && applyResult && !applyResult.ok) {
    process.exitCode = 2;
  } else if (!report.ready && wantApply) {
    process.exitCode = 2;
  } else {
    process.exitCode = 0;
  }
}

main().catch((e) => {
  console.error("[setup] FAILED:", e instanceof Error ? e.message : e);
  process.exitCode = 1;
});