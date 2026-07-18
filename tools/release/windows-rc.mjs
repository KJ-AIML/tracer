#!/usr/bin/env node
/**
 * Windows release candidate packaging entrypoint (W2.3-A).
 *
 * Command (repo root):
 *   pnpm release:windows
 *   node tools/release/windows-rc.mjs
 *
 * Flags:
 *   --skip-build     reuse existing release artifacts
 *   --no-bundle      cargo release binary only (portable), skip tauri bundler/NSIS
 *   --identity-only  run identity check and exit
 *   --json           machine-readable summary on stdout
 *
 * Supported outputs (decision):
 *   - NSIS installer (primary)
 *   - portable release binary tracer-desktop.exe (secondary)
 *   - MSI not selected for this RC
 *
 * Signing: classified honestly; UNSIGNED_DEVELOPMENT_RC is a valid local PASS.
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { checkIdentity } from "./lib/identity.mjs";
import { classifyReleaseSigning } from "./lib/signing.mjs";
import { discoverArtifacts, stageArtifacts } from "./lib/artifacts.mjs";
import {
  REPO_ROOT,
  DESKTOP_DIR,
  SRC_TAURI,
  FAKE_ACP_JS,
  releaseStageDir,
} from "./lib/paths.mjs";

const args = new Set(process.argv.slice(2));
const json = args.has("--json");
const skipBuild = args.has("--skip-build");
const noBundle = args.has("--no-bundle");
const identityOnly = args.has("--identity-only");

function log(msg) {
  if (!json) console.log(msg);
}

function run(cmd, cmdArgs, opts = {}) {
  log(`$ ${cmd} ${cmdArgs.join(" ")}`);
  const r = spawnSync(cmd, cmdArgs, {
    cwd: opts.cwd || REPO_ROOT,
    encoding: "utf8",
    windowsHide: true,
    shell: opts.shell === true,
    env: { ...process.env, ...(opts.env || {}) },
    stdio: json ? "pipe" : "inherit",
  });
  return {
    ok: r.status === 0,
    status: r.status,
    stdout: r.stdout || "",
    stderr: r.stderr || "",
    error: r.error ? String(r.error.message || r.error) : null,
  };
}

function findTauriCli() {
  // Prefer local node_modules, then npx.
  const localCandidates = [
    path.join(REPO_ROOT, "node_modules/@tauri-apps/cli/tauri.js"),
    path.join(DESKTOP_DIR, "node_modules/@tauri-apps/cli/tauri.js"),
    path.join(
      REPO_ROOT,
      "node_modules/.bin/tauri" + (process.platform === "win32" ? ".cmd" : ""),
    ),
  ];
  for (const c of localCandidates) {
    if (existsSync(c)) return { kind: "node-script", path: c };
  }
  return { kind: "npx", path: null };
}

function buildPortable() {
  // Frontend dist for consistency (not embedded without tauri build, but
  // keeps release tree honest when callers need dist).
  const fe = run(
    process.platform === "win32" ? "pnpm.cmd" : "pnpm",
    ["--filter", "@tracer/desktop", "build"],
    { shell: process.platform === "win32" },
  );
  if (!fe.ok) {
    return { ok: false, step: "frontend_build", detail: fe };
  }
  const cargo = run("cargo", ["build", "-p", "tracer-desktop", "--release"]);
  if (!cargo.ok) {
    return { ok: false, step: "cargo_release", detail: cargo };
  }
  return { ok: true, step: "portable" };
}

function buildBundle() {
  const cli = findTauriCli();
  let r;
  if (cli.kind === "node-script") {
    r = run(process.execPath, [cli.path, "build"], { cwd: DESKTOP_DIR });
  } else {
    // npx downloads @tauri-apps/cli when not installed
    r = run(
      process.platform === "win32" ? "npx.cmd" : "npx",
      ["--yes", "@tauri-apps/cli@2", "build"],
      { cwd: DESKTOP_DIR, shell: process.platform === "win32" },
    );
  }
  if (!r.ok) {
    return { ok: false, step: "tauri_build", detail: r };
  }
  return { ok: true, step: "nsis_bundle" };
}

function main() {
  const startedAt = new Date().toISOString();
  log("=== Tracer Windows RC packaging (W2.3-A) ===");
  log(`cwd: ${REPO_ROOT}`);
  log(`platform: ${process.platform}/${process.arch}`);

  if (process.platform !== "win32") {
    const summary = {
      result: "UNSUPPORTED_PLATFORM",
      message: "Windows RC packaging requires a Windows host",
      platform: process.platform,
    };
    if (json) console.log(JSON.stringify(summary, null, 2));
    else console.error(summary.message);
    process.exit(3);
  }

  const identity = checkIdentity();
  if (!identity.ok) {
    const summary = {
      result: "FAIL",
      step: "identity",
      identity,
    };
    if (json) console.log(JSON.stringify(summary, null, 2));
    else {
      console.error("Identity check failed. Run: node tools/release/identity-check.mjs");
      for (const c of identity.checks.filter((x) => x.status === "fail")) {
        console.error(`  FAIL ${c.id}`, c);
      }
    }
    process.exit(1);
  }
  log("identity: PASS");

  if (identityOnly) {
    const summary = { result: "PASS", step: "identity_only", identity };
    if (json) console.log(JSON.stringify(summary, null, 2));
    process.exit(0);
  }

  let buildResult = { ok: true, step: "skipped" };
  if (!skipBuild) {
    if (noBundle) {
      log("build mode: portable only (--no-bundle)");
      buildResult = buildPortable();
    } else {
      log("build mode: tauri build (NSIS primary)");
      buildResult = buildBundle();
      // Ensure portable release binary exists even if bundler partial-fails mid-way
      if (!buildResult.ok) {
        log("tauri build failed; attempting portable cargo --release fallback");
        const portable = buildPortable();
        if (portable.ok) {
          buildResult = {
            ok: true,
            step: "portable_fallback",
            priorFailure: buildResult,
          };
        }
      }
    }
  } else {
    log("build: skipped (--skip-build)");
  }

  const found = discoverArtifacts();
  const signing = classifyReleaseSigning(found.all);
  const staged = stageArtifacts({
    signingClass: signing.class,
    identityVersion: identity.version,
    buildStep: buildResult.step,
  });

  const hasPrimary =
    found.nsis.length > 0 || found.portable != null;

  let result = "PASS";
  let message = "Windows RC packaging complete";
  if (!buildResult.ok && !hasPrimary) {
    result = "FAIL";
    message = `build failed (${buildResult.step}) and no artifacts found`;
  } else if (!hasPrimary) {
    result = "FAIL";
    message = "no NSIS or portable artifacts produced";
  } else if (signing.class === "BLOCKED") {
    result = "BLOCKED";
    message = "signing classification BLOCKED";
  } else if (found.nsis.length === 0 && found.portable) {
    result = "PARTIAL";
    message =
      "portable binary only (NSIS installer not produced); UNSIGNED_DEVELOPMENT_RC still classifiable";
  }

  const summary = {
    schemaVersion: 1,
    kind: "windows-rc",
    result,
    message,
    startedAt,
    finishedAt: new Date().toISOString(),
    packagingDecision: {
      primary: "nsis",
      secondary: "portable",
      msi: "not_selected_for_rc",
      msiRationale:
        "MSI deferred for this RC (WiX + VBScript optional feature weight). Config targets nsis only.",
    },
    identity: {
      productName: identity.identity.productName,
      identifier: identity.identity.identifier,
      mainBinaryName: identity.identity.mainBinaryName,
      version: identity.version,
      appDataWindows: identity.identity.appDataWindowsTemplate,
    },
    signing: {
      class: signing.class,
      note: signing.note,
    },
    artifacts: {
      portable: found.portable,
      nsis: found.nsis,
      msi: found.msi,
      stagedDir: staged.stage,
      manifest: staged.manifestPath,
    },
    fakeRuntime: {
      script: FAKE_ACP_JS,
      present: existsSync(FAKE_ACP_JS),
    },
    build: {
      ok: buildResult.ok,
      step: buildResult.step,
      skipBuild,
      noBundle,
    },
    srcTauri: SRC_TAURI,
  };

  // Persist summary next to staged artifacts
  try {
    mkdirSync(releaseStageDir(), { recursive: true });
    writeFileSync(
      path.join(releaseStageDir(), "rc-summary.json"),
      JSON.stringify(summary, null, 2),
      "utf8",
    );
  } catch {
    /* non-fatal */
  }

  if (json) {
    console.log(JSON.stringify(summary, null, 2));
  } else {
    console.log("");
    console.log("=== RC summary ===");
    console.log(`result:  ${result}`);
    console.log(`message: ${message}`);
    console.log(`signing: ${signing.class}`);
    console.log(`portable: ${found.portable || "(none)"}`);
    console.log(`nsis:     ${found.nsis[0] || "(none)"}`);
    console.log(`msi:      ${found.msi[0] || "(not selected)"}`);
    console.log(`stage:    ${staged.stage}`);
  }

  if (result === "FAIL") process.exit(1);
  if (result === "BLOCKED") process.exit(2);
  // PASS and PARTIAL exit 0 (PARTIAL is honest success with reduced artifact set)
  process.exit(0);
}

main();
