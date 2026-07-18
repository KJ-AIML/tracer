#!/usr/bin/env node
/**
 * L2 — built/packaged application launch smoke.
 *
 * Stages: frontend build → backend build → binary resolve → (no driver) →
 *         app launch → readiness → smoke → app shutdown → orphan verification
 *
 * Does NOT claim L3-I or L3-J. WebView content / Tauri IPC from inside the
 * window are best-effort without a driver (window handle + process liveness).
 *
 * Classifications: PASS | PARTIAL | BLOCKED_BY_TOOLING | BLOCKED_BY_WEBVIEW |
 *                  UNSUPPORTED_PLATFORM | FAIL
 */

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { ResultClass, StageId, Level } from "./lib/classify.mjs";
import {
  REPO_ROOT,
  DESKTOP_DIR,
  FAKE_ACP_JS,
  FRONTEND_DIST,
  discoverEnvironment,
  doctorIssues,
  resolvePreferredBinary,
} from "./lib/discover.mjs";
import {
  spawnOwned,
  waitFor,
  processAlive,
  windowsProcessHasMainWindow,
  findOrphans,
  reapOrphans,
  uniqueTempDir,
  installExitHooks,
  stopAllOwned,
} from "./lib/process.mjs";
import { createStageReport } from "./lib/stages.mjs";

const args = new Set(process.argv.slice(2));
const skipBuild = args.has("--skip-build");
const jsonOut = args.has("--json");
const profile = process.env.TRACER_E2E_PROFILE || (args.has("--release") ? "release" : "debug");

installExitHooks();

function runCapture(cmd, cmdArgs, opts = {}) {
  const r = spawnSync(cmd, cmdArgs, {
    cwd: opts.cwd ?? REPO_ROOT,
    env: { ...process.env, ...(opts.env || {}) },
    encoding: "utf8",
    shell: process.platform === "win32",
    windowsHide: true,
    timeout: opts.timeoutMs ?? 600_000,
  });
  return r;
}

function ensureFrontendDist({ build }) {
  const indexHtml = path.join(FRONTEND_DIST, "index.html");
  if (existsSync(indexHtml) && !build) {
    return { built: false, path: FRONTEND_DIST };
  }
  if (!build && !existsSync(indexHtml)) {
    // Real L2 wants a Vite build; allow explicit skip only with stub warning
    throw new Error(
      "frontend dist missing; run with build enabled or: pnpm --filter @tracer/desktop build",
    );
  }
  if (build) {
    const r = runCapture("pnpm", ["--filter", "@tracer/desktop", "build"], {
      timeoutMs: 600_000,
    });
    if (r.status !== 0) {
      throw new Error(
        `frontend build failed (exit ${r.status}): ${(r.stderr || r.stdout || "").slice(0, 800)}`,
      );
    }
  }
  if (!existsSync(indexHtml)) {
    throw new Error(`frontend dist still missing after build: ${indexHtml}`);
  }
  return { built: Boolean(build), path: FRONTEND_DIST };
}

function ensureBackendBinary({ build }) {
  if (process.env.TRACER_E2E_APP_BINARY && existsSync(process.env.TRACER_E2E_APP_BINARY)) {
    return {
      path: process.env.TRACER_E2E_APP_BINARY,
      profile: "env",
      built: false,
    };
  }
  let bin = resolvePreferredBinary({ preferRelease: profile === "release" });
  if (bin && !build) return { ...bin, built: false };
  if (!build && !bin) {
    throw new Error(
      "app binary missing; run cargo build -p tracer-desktop or pass --skip-build after building",
    );
  }
  if (build) {
    // Ensure dist exists for generate_context!
    if (!existsSync(path.join(FRONTEND_DIST, "index.html"))) {
      mkdirSync(FRONTEND_DIST, { recursive: true });
      writeFileSync(
        path.join(FRONTEND_DIST, "index.html"),
        `<!doctype html><html><body><div id="root">tracer e2e</div></body></html>\n`,
        "utf8",
      );
    }
    const cargoArgs = ["build", "-p", "tracer-desktop"];
    if (profile === "release") cargoArgs.push("--release");
    const r = runCapture("cargo", cargoArgs, { timeoutMs: 900_000 });
    if (r.status !== 0) {
      throw new Error(
        `backend build failed (exit ${r.status}): ${(r.stderr || r.stdout || "").slice(0, 1200)}`,
      );
    }
  }
  bin = resolvePreferredBinary({ preferRelease: profile === "release" });
  if (!bin) throw new Error("binary still missing after cargo build");
  return { ...bin, built: Boolean(build) };
}

async function main() {
  const env = discoverEnvironment();
  const issues = doctorIssues(env);
  const report = createStageReport();
  const workDir = uniqueTempDir("tracer-l2-smoke");
  const logDir = path.join(workDir, "logs");
  mkdirSync(logDir, { recursive: true });
  const dbPath = path.join(workDir, "tracer-e2e.sqlite");

  /** @type {import('./lib/process.mjs').OwnedProcess | null} */
  let app = null;
  const orphanNames = ["tracer-desktop", "tracer_desktop"];

  console.log("=== L2 packaged application launch smoke (W2.2-A) ===");
  console.log(`workDir: ${workDir}`);
  console.log(`profile: ${profile}`);

  try {
    // Platform gate
    if (!env.os.supportedForL2) {
      report.skip(StageId.FRONTEND_BUILD, `unsupported platform ${env.os.platform}`, ResultClass.UNSUPPORTED_PLATFORM);
      report.skip(StageId.BACKEND_BUILD, "skipped", ResultClass.UNSUPPORTED_PLATFORM);
      report.skip(StageId.PACKAGING, "skipped", ResultClass.UNSUPPORTED_PLATFORM);
      report.skip(StageId.DRIVER_STARTUP, "L2 does not start driver", ResultClass.PASS);
      report.skip(StageId.APP_LAUNCH, "unsupported", ResultClass.UNSUPPORTED_PLATFORM);
      const summary = report.summary();
      return finish(summary, { level: Level.L2_PACKAGED_SMOKE, workDir });
    }

    if (env.os.platform === "win32" && !env.webview.available) {
      report.skip(
        StageId.FRONTEND_BUILD,
        "WebView2 unavailable",
        ResultClass.BLOCKED_BY_WEBVIEW,
      );
      report.skip(StageId.APP_LAUNCH, "WebView2 unavailable", ResultClass.BLOCKED_BY_WEBVIEW);
      const summary = report.summary();
      return finish(summary, {
        level: Level.L2_PACKAGED_SMOKE,
        workDir,
        resultOverride: ResultClass.BLOCKED_BY_WEBVIEW,
      });
    }

    // 1) frontend build
    await report.run(StageId.FRONTEND_BUILD, async () => {
      if (skipBuild && existsSync(path.join(FRONTEND_DIST, "index.html"))) {
        return {
          status: "pass",
          message: "using existing frontend dist",
          detail: { path: FRONTEND_DIST },
        };
      }
      if (skipBuild) {
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message: "skip-build set and dist missing or incomplete — will try backend only",
        };
      }
      const r = ensureFrontendDist({ build: true });
      return { status: "pass", message: "frontend built", detail: r };
    });

    // 2) backend build
    let binaryInfo = null;
    await report.run(StageId.BACKEND_BUILD, async () => {
      binaryInfo = ensureBackendBinary({ build: !skipBuild });
      return {
        status: "pass",
        message: `binary ${binaryInfo.path}`,
        detail: binaryInfo,
      };
    });

    // 3) packaging / test binary resolve
    await report.run(StageId.PACKAGING, async () => {
      if (!binaryInfo?.path || !existsSync(binaryInfo.path)) {
        throw new Error("test binary path missing");
      }
      return {
        status: "pass",
        message: "test binary ready (cargo artifact; bundle.active=false in tauri.conf)",
        detail: {
          path: binaryInfo.path,
          profile: binaryInfo.profile,
          note: "Not an MSI/NSIS installer package — executable artifact smoke",
        },
      };
    });

    // 4) driver — N/A for L2
    report.skip(
      StageId.DRIVER_STARTUP,
      "L2 process smoke does not start tauri-driver (see L3-I)",
      ResultClass.PASS,
    );

    // 5) app launch
    await report.run(StageId.APP_LAUNCH, async () => {
      const appEnv = {
        TRACER_DATABASE_PATH: dbPath,
        TRACER_FAKE_ACP_JS: FAKE_ACP_JS,
        TRACER_HELI_PROBE_PATH: path.join(workDir, "heli-empty"),
        TRACER_NODE_BIN: process.env.TRACER_NODE_BIN || "node",
        // Avoid interactive prompts
        RUST_BACKTRACE: process.env.RUST_BACKTRACE || "1",
      };
      mkdirSync(appEnv.TRACER_HELI_PROBE_PATH, { recursive: true });
      app = spawnOwned(binaryInfo.path, [], {
        label: "tracer-desktop",
        logDir,
        env: appEnv,
        cwd: path.dirname(binaryInfo.path),
        windowsHide: false, // allow GUI
      });
      if (!app.pid) throw new Error("failed to spawn app (no pid)");
      // brief settle
      await delay(500);
      if (!processAlive(app.pid)) {
        throw new Error(
          `app exited immediately (pid ${app.pid}); see logs in ${logDir}`,
        );
      }
      return {
        status: "pass",
        message: `spawned pid=${app.pid}`,
        detail: { pid: app.pid, logs: app.logPaths },
      };
    });

    // 6) readiness
    let hasWindow = null;
    await report.run(StageId.READINESS, async () => {
      await waitFor(() => processAlive(app.pid), {
        timeoutMs: 15_000,
        label: "app process alive",
      });
      // Give WebView time to initialize
      await delay(2_500);
      if (!processAlive(app.pid)) {
        throw new Error("app died during readiness wait");
      }
      if (process.platform === "win32") {
        // Retry window handle a few times
        for (let i = 0; i < 10; i++) {
          hasWindow = windowsProcessHasMainWindow(app.pid);
          if (hasWindow) break;
          await delay(500);
        }
      }
      return {
        status: hasWindow === false ? "partial" : "pass",
        classification:
          hasWindow === false ? ResultClass.PARTIAL : ResultClass.PASS,
        message:
          hasWindow === true
            ? "process alive + main window present"
            : hasWindow === false
              ? "process alive but main window not detected yet (PARTIAL)"
              : "process alive (window check N/A on this OS)",
        detail: { pid: app.pid, mainWindow: hasWindow },
      };
    });

    // 7) smoke checklist (1–10 subset for L2 without driver)
    await report.run(StageId.SMOKE, async () => {
      const checks = {
        build: true,
        launch: processAlive(app.pid),
        webviewInit: hasWindow === true || hasWindow === null,
        frontendRoot: null, // requires driver/DOM
        tauriApiDetect: null, // requires driver execute
        appInfo: null,
        initialSnapshot: null,
        cleanExit: null, // filled after shutdown
        driverExit: "n/a",
        noOrphans: null,
      };
      const executable = ["build", "launch"].every((k) => checks[k]);
      if (!executable) throw new Error("smoke launch/build checks failed");
      return {
        status:
          hasWindow === true
            ? "pass"
            : "partial",
        classification:
          hasWindow === true ? ResultClass.PASS : ResultClass.PARTIAL,
        message:
          "L2 smoke: process launch verified; DOM/Tauri API checks deferred to L3-I",
        detail: { checks, level: Level.L2_PACKAGED_SMOKE },
      };
    });

    // 8) app shutdown
    await report.run(StageId.APP_SHUTDOWN, async () => {
      if (!app) return { status: "pass", message: "no app handle" };
      const pid = app.pid;
      await app.stop();
      await delay(500);
      const alive = processAlive(pid);
      if (alive) {
        throw new Error(`app pid ${pid} still alive after stop`);
      }
      return { status: "pass", message: `stopped pid=${pid}` };
    });

    // 9) driver shutdown N/A
    report.skip(StageId.DRIVER_SHUTDOWN, "no driver in L2", ResultClass.PASS);

    // 10) orphan verification
    await report.run(StageId.ORPHAN_VERIFY, async () => {
      await delay(400);
      let orphans = findOrphans(orphanNames);
      if (orphans.length) {
        const reaped = reapOrphans(orphanNames);
        orphans = findOrphans(orphanNames);
        if (orphans.length) {
          throw new Error(
            `orphans remain: ${JSON.stringify(orphans)}; reaped=${JSON.stringify(reaped)}`,
          );
        }
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message: "orphans found and reaped",
          detail: { reaped },
        };
      }
      return { status: "pass", message: "no orphans" };
    });

    const summary = report.summary();
    return finish(summary, {
      level: Level.L2_PACKAGED_SMOKE,
      workDir,
      binary: binaryInfo?.path,
      issues,
    });
  } catch (e) {
    console.error("[l2-smoke] FAILED:", e instanceof Error ? e.message : e);
    try {
      await stopAllOwned();
      reapOrphans(orphanNames);
    } catch {
      /* ignore */
    }
    const summary = report.summary();
    // Ensure FAIL if exception escaped
    if (summary.result === ResultClass.PASS) {
      summary.result = ResultClass.FAIL;
    }
    return finish(summary, {
      level: Level.L2_PACKAGED_SMOKE,
      workDir,
      error: e instanceof Error ? e.message : String(e),
    }, 1);
  }
}

function finish(summary, meta, exitHint) {
  const out = {
    schemaVersion: 1,
    level: meta.level,
    result: meta.resultOverride || summary.result,
    stages: summary.stages,
    meta: {
      workDir: meta.workDir,
      binary: meta.binary,
      error: meta.error,
      ciClass: "windows_gui_runner | platform_gated_ci | manual_local",
      network: false,
      credentials: false,
      liveGrok: false,
      claimsL3J: false,
    },
  };
  if (jsonOut) {
    console.log(JSON.stringify(out, null, 2));
  } else {
    console.log("");
    console.log(`L2 result: ${out.result}`);
    for (const s of out.stages) {
      console.log(
        `  [${s.status}] ${s.id}${s.message ? " — " + s.message : ""} (${s.durationMs || 0}ms)`,
      );
    }
  }
  // Write report artifact
  try {
    writeFileSync(
      path.join(meta.workDir || uniqueTempDir("tracer-l2-out"), "l2-report.json"),
      JSON.stringify(out, null, 2),
      "utf8",
    );
  } catch {
    /* ignore */
  }

  if (exitHint != null) {
    process.exitCode = exitHint;
  } else if (
    out.result === ResultClass.PASS ||
    out.result === ResultClass.PARTIAL ||
    out.result === ResultClass.BLOCKED_BY_TOOLING ||
    out.result === ResultClass.BLOCKED_BY_WEBVIEW ||
    out.result === ResultClass.UNSUPPORTED_PLATFORM
  ) {
    // Honest non-pass classifications are not hard fails for CI gating of infra
    process.exitCode = out.result === ResultClass.FAIL ? 1 : 0;
  } else {
    process.exitCode = 1;
  }
  return out;
}

main();
