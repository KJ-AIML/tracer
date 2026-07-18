#!/usr/bin/env node
/**
 * L3-J — Full WebView GUI product journeys (W2.2-B).
 *
 * Lifecycle:
 *   doctor READY → build → msedgedriver → tauri-driver → launch isolated env
 *   → WebDriver session → app-ready marker → journeys GJ-01…12 → artifacts on fail
 *   → session/app/drivers/fake-runtime cleanup → orphan check → temp cleanup
 *
 * Environment: actual Tauri executable + real WebDriver; fake ACP only;
 * temp file SQLite; unique app-data/work dirs; no live Grok / network / credentials.
 *
 * Classification independent of L0–L3-I.
 */

import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  writeFileSync,
  rmSync,
  readFileSync,
} from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import {
  ResultClass,
  Level,
  FailureCode,
  StageId,
  suiteResultFromJourneys,
} from "./lib/classify.mjs";
import {
  REPO_ROOT,
  FAKE_ACP_JS,
  FRONTEND_DIST,
  discoverEnvironment,
  resolvePreferredBinary,
  resolveDriverSpawnPaths,
} from "./lib/discover.mjs";
import {
  spawnOwned,
  findOrphans,
  reapOrphans,
  uniqueTempDir,
  installExitHooks,
  stopAllOwned,
  processAlive,
} from "./lib/process.mjs";
import { createStageReport } from "./lib/stages.mjs";
import { WebDriverClient, waitDriverReady } from "./lib/webdriver.mjs";
import { filterJourneys } from "./lib/journeys.mjs";
import { waitAppReady, attrTestId } from "./lib/gui.mjs";

const args = process.argv.slice(2);
const argSet = new Set(args);
const jsonOut = argSet.has("--json");
const skipBuild = argSet.has("--skip-build");
const journeyFilter = (() => {
  const i = args.indexOf("--journey");
  if (i >= 0 && args[i + 1]) return args[i + 1];
  const eq = args.find((a) => a.startsWith("--journey="));
  return eq ? eq.split("=")[1] : null;
})();

const port = Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444);
const host = process.env.TRACER_TAURI_DRIVER_HOST || "127.0.0.1";
const baseUrl = `http://${host}:${port}`;
const profile = process.env.TRACER_E2E_PROFILE || "debug";

installExitHooks();

const ORPHAN_NAMES = [
  "tracer-desktop",
  "tracer_desktop",
  "tauri-driver",
  "msedgedriver",
  "WebKitWebDriver",
  "fake-acp-runtime",
  "node", // careful: only reaped via our owned handles; orphan list is name-based
];

const ORPHAN_CHECK_NAMES = [
  "tracer-desktop",
  "tracer_desktop",
  "tauri-driver",
  "msedgedriver",
  "WebKitWebDriver",
];

function runCapture(cmd, cmdArgs, opts = {}) {
  return spawnSync(cmd, cmdArgs, {
    cwd: opts.cwd ?? REPO_ROOT,
    env: { ...process.env, ...(opts.env || {}) },
    encoding: "utf8",
    shell: process.platform === "win32",
    windowsHide: true,
    timeout: opts.timeoutMs ?? 600_000,
  });
}

function ensureFrontend({ build }) {
  const indexHtml = path.join(FRONTEND_DIST, "index.html");
  if (existsSync(indexHtml) && !build) return { path: FRONTEND_DIST, built: false };
  if (build) {
    const r = runCapture("pnpm", ["--filter", "@tracer/desktop", "build"], {
      timeoutMs: 600_000,
    });
    if (r.status !== 0) {
      throw Object.assign(
        new Error(`frontend build failed: ${(r.stderr || r.stdout || "").slice(0, 800)}`),
        { code: FailureCode.FRONTEND_DIST_NOT_FOUND },
      );
    }
  }
  if (!existsSync(indexHtml)) {
    throw Object.assign(new Error("frontend dist missing"), {
      code: FailureCode.FRONTEND_DIST_NOT_FOUND,
    });
  }
  return { path: FRONTEND_DIST, built: Boolean(build) };
}

function ensureBinary({ build }) {
  if (process.env.TRACER_E2E_APP_BINARY && existsSync(process.env.TRACER_E2E_APP_BINARY)) {
    return process.env.TRACER_E2E_APP_BINARY;
  }
  let bin = resolvePreferredBinary({ preferRelease: profile === "release" });
  if (bin?.path && !build) return bin.path;
  if (build || !bin?.path) {
    const r = runCapture(
      "cargo",
      ["build", "-p", "tracer-desktop", ...(profile === "release" ? ["--release"] : [])],
      { timeoutMs: 900_000 },
    );
    if (r.status !== 0) {
      throw Object.assign(
        new Error(`cargo build failed: ${(r.stderr || r.stdout || "").slice(0, 800)}`),
        { code: FailureCode.APP_BINARY_NOT_FOUND },
      );
    }
    bin = resolvePreferredBinary({ preferRelease: profile === "release" });
  }
  if (!bin?.path || !existsSync(bin.path)) {
    throw Object.assign(new Error("app binary missing after build"), {
      code: FailureCode.APP_BINARY_NOT_FOUND,
    });
  }
  return bin.path;
}

function runId() {
  const d = new Date();
  const stamp = d.toISOString().replace(/[:.]/g, "-");
  return `l3j-${stamp}-${process.pid}`;
}

async function main() {
  const env = discoverEnvironment();
  const report = createStageReport();
  const rid = runId();
  const workDir = uniqueTempDir("tracer-l3j");
  const artifactsDir = path.join(REPO_ROOT, "artifacts", "tauri-e2e", rid);
  mkdirSync(artifactsDir, { recursive: true });
  const logDir = path.join(workDir, "logs");
  mkdirSync(logDir, { recursive: true });
  const dbPath = path.join(workDir, "tracer-l3j.sqlite");
  const projectRoot = path.join(workDir, "sample-project");
  mkdirSync(projectRoot, { recursive: true });
  writeFileSync(path.join(projectRoot, "README.md"), "# L3-J sample project\n", "utf8");
  const heliEmpty = path.join(workDir, "heli-empty");
  mkdirSync(heliEmpty, { recursive: true });
  const readyMarker = path.join(workDir, "app-ready.marker");

  /** @type {import('./lib/process.mjs').OwnedProcess | null} */
  let driver = null;
  const client = new WebDriverClient(baseUrl);
  const spawnPaths = resolveDriverSpawnPaths();
  let binary = null;
  /** Resolved app entry (wrapper script when env injection required). */
  let applicationEntry = null;
  /** @type {Record<string,string>} */
  let appEnv = {};

  console.log("=== L3-J Full WebView GUI Product Journeys (W2.2-B) ===");
  console.log(`runId: ${rid}`);
  console.log(`workDir: ${workDir}`);
  console.log(`artifacts: ${artifactsDir}`);
  console.log(`driver: ${baseUrl}`);
  console.log(`journeys: ${journeyFilter || "ALL GJ-01..GJ-12"}`);

  const journeysToRun = filterJourneys(journeyFilter);
  /** @type {import('./lib/journeys.mjs').JOURNEY_RUNNERS[0] extends never ? any : any[]} */
  const journeyResults = [];

  async function captureArtifact(label) {
    const dir = path.join(artifactsDir, label);
    mkdirSync(dir, { recursive: true });
    try {
      const src = await client.getPageSource();
      writeFileSync(path.join(dir, "page.html"), src.raw || JSON.stringify(src.body), "utf8");
    } catch (e) {
      writeFileSync(
        path.join(dir, "page-error.txt"),
        e instanceof Error ? e.message : String(e),
        "utf8",
      );
    }
    try {
      const title = await client.getTitle();
      writeFileSync(path.join(dir, "title.json"), JSON.stringify(title.body, null, 2), "utf8");
    } catch {
      /* ignore */
    }
    try {
      const probe = await client.execute(`
        return {
          ready: !!document.querySelector('[data-testid="tracer-app-ready"]'),
          backend: document.querySelector('[data-testid="tracer-app-root"]')?.getAttribute('data-tracer-backend'),
          route: document.querySelector('[data-testid="tracer-app-root"]')?.getAttribute('data-tracer-route'),
          status: document.querySelector('[data-testid="tracer-session-workspace"]')?.getAttribute('data-session-status'),
          title: document.title
        };
      `);
      writeFileSync(path.join(dir, "probe.json"), JSON.stringify(probe.body, null, 2), "utf8");
    } catch {
      /* ignore */
    }
    return { dir };
  }

  /**
   * Write dotenv-style E2E env file consumed by desktop `--tracer-e2e-env=`.
   * Preferred over tauri:options.env which is not reliable on all Windows hosts.
   */
  function writeE2eEnvFile(envMap) {
    const envFile = path.join(workDir, "tracer-e2e.env");
    const body = Object.entries(envMap)
      .map(([k, v]) => `${k}=${v}`)
      .join("\n");
    writeFileSync(envFile, body + "\n", "utf8");
    return envFile;
  }

  async function startDriverAndApp() {
    if (driver) {
      try {
        await client.deleteSession().catch(() => {});
      } catch {
        /* ignore */
      }
      await driver.stop().catch(() => {});
      driver = null;
    }
    const driverArgs = ["--port", String(port)];
    const native =
      process.env.TRACER_NATIVE_DRIVER || spawnPaths.nativePath || null;
    if (native) driverArgs.push("--native-driver", native);
    driver = spawnOwned(spawnPaths.tauriPath, driverArgs, {
      label: "tauri-driver",
      logDir,
      windowsHide: true,
    });
    await waitDriverReady(baseUrl, { timeoutMs: 30_000 });

    appEnv = {
      TRACER_DATABASE_PATH: dbPath,
      TRACER_FAKE_ACP_JS: FAKE_ACP_JS,
      TRACER_HELI_PROBE_PATH: heliEmpty,
      TRACER_NODE_BIN: process.env.TRACER_NODE_BIN || "node",
      TRACER_E2E_READY_MARKER: readyMarker,
      TRACER_E2E_PROFILE: profile,
    };
    const envFile = writeE2eEnvFile(appEnv);
    applicationEntry = binary;
    console.log(`[l3j] e2e env file: ${envFile}`);
    const res = await client.newSession(
      {
        application: binary,
        args: [`--tracer-e2e-env=${envFile}`],
        // Best-effort; primary injection is --tracer-e2e-env file.
        env: appEnv,
      },
      { timeoutMs: 120_000 },
    );
    if (!client.sessionId) {
      throw Object.assign(
        new Error(
          `WebDriver new session failed: HTTP ${res.statusCode} ${JSON.stringify(res.body).slice(0, 500)}`,
        ),
        { code: FailureCode.SESSION_CREATE_FAILED },
      );
    }
    await waitAppReady(client, { timeoutMs: 60_000 });
    // Mark automation mode so GUI skips blocking window.confirm on leave.
    await client
      .execute(`globalThis.__TRACER_E2E__ = true; return true;`)
      .catch(() => {});
  }

  async function relaunchApp() {
    // Delete session (kills app) then new session with same env/db.
    // Wait for process exit so SQLite lock is released — otherwise the next
    // build_control_plane may fail open and fall back to empty in-memory store.
    try {
      await client.deleteSession({ timeoutMs: 30_000 });
    } catch {
      /* ignore */
    }
    await delay(1500);
    for (let i = 0; i < 20; i++) {
      const orphans = findOrphans(["tracer-desktop", "tracer_desktop"]);
      if (!orphans.length) break;
      reapOrphans(["tracer-desktop", "tracer_desktop"]);
      await delay(500);
    }
    await delay(500);
    const envFile = writeE2eEnvFile(appEnv);
    const res = await client.newSession(
      {
        application: binary,
        args: [`--tracer-e2e-env=${envFile}`],
        env: { ...appEnv },
      },
      { timeoutMs: 120_000 },
    );
    if (!client.sessionId) {
      throw new Error(
        `relaunch session failed: HTTP ${res.statusCode} ${JSON.stringify(res.body).slice(0, 400)}`,
      );
    }
    await waitAppReady(client, { timeoutMs: 60_000 });
    await client
      .execute(`globalThis.__TRACER_E2E__ = true; return true;`)
      .catch(() => {});
  }

  try {
    if (!env.os.supportedForL3I_externalDriver) {
      for (const j of journeysToRun) {
        journeyResults.push({
          id: j.id,
          name: j.name,
          result: ResultClass.UNSUPPORTED_PLATFORM,
          message: `unsupported on ${env.os.platform}`,
        });
      }
      return finish(report.summary(), {
        workDir,
        artifactsDir,
        rid,
        journeyResults,
        resultOverride: ResultClass.UNSUPPORTED_PLATFORM,
      });
    }

    await report.run(StageId.FRONTEND_BUILD, async () => {
      const r = ensureFrontend({ build: !skipBuild });
      return { status: "pass", message: `frontend dist ${r.path}`, detail: r };
    });

    await report.run(StageId.BACKEND_BUILD, async () => {
      binary = ensureBinary({ build: !skipBuild });
      return { status: "pass", message: `binary ${binary}`, detail: { binary } };
    });

    await report.run(StageId.PACKAGING, async () => {
      if (!binary || !existsSync(binary)) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message: "no test binary",
        };
      }
      return { status: "pass", message: "test binary ready" };
    });

    const hasDriver = Boolean(spawnPaths.tauriPath);
    const nativePath =
      spawnPaths.nativePath || process.env.TRACER_NATIVE_DRIVER || null;
    const hasNative =
      env.os.platform === "win32"
        ? Boolean(
            nativePath &&
              env.drivers.nativeDriver.msedgedriver.compatibility?.compatible,
          )
        : Boolean(env.drivers.nativeDriver.webkitWebDriver.available || nativePath);

    if (!hasDriver || !hasNative) {
      const msg = !hasDriver
        ? "tauri-driver not found"
        : "native WebDriver missing or incompatible";
      for (const j of journeysToRun) {
        journeyResults.push({
          id: j.id,
          name: j.name,
          result: ResultClass.BLOCKED_BY_TOOLING,
          message: msg,
        });
      }
      report.skip(StageId.DRIVER_STARTUP, msg, ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.APP_LAUNCH, "blocked", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.READINESS, "blocked", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.SMOKE, "blocked", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.APP_SHUTDOWN, "blocked", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.DRIVER_SHUTDOWN, "blocked", ResultClass.BLOCKED_BY_TOOLING);
      await report.run(StageId.ORPHAN_VERIFY, async () => ({
        status: "pass",
        message: "no processes started",
      }));
      return finish(report.summary(), {
        workDir,
        artifactsDir,
        rid,
        journeyResults,
        resultOverride: ResultClass.BLOCKED_BY_TOOLING,
        failureCode: FailureCode.EDGE_DRIVER_NOT_FOUND,
      });
    }

    await report.run(StageId.DRIVER_STARTUP, async () => {
      // started inside startDriverAndApp
      return { status: "pass", message: "driver start deferred to app launch bundle" };
    });

    await report.run(StageId.APP_LAUNCH, async () => {
      await startDriverAndApp();
      return {
        status: "pass",
        message: `session ${client.sessionId}`,
        detail: { sessionId: client.sessionId, binary, dbPath },
      };
    });

    await report.run(StageId.READINESS, async () => {
      const backend = await attrTestId(client, "tracer-app-root", "data-tracer-backend");
      const markerExists = existsSync(readyMarker);
      return {
        status: "pass",
        message: `app ready backend=${backend} fileMarker=${markerExists}`,
        detail: { backend, readyMarker: markerExists },
      };
    });

    // Product journeys (serial)
    await report.run(StageId.SMOKE, async () => {
      const ctx = {
        client,
        workDir,
        projectRoot,
        dbPath,
        artifactsDir,
        captureArtifact,
        relaunchApp,
      };
      for (const j of journeysToRun) {
        console.log(`\n--- ${j.id} ${j.name} ---`);
        const started = Date.now();
        let result;
        try {
          result = await j.run(ctx);
        } catch (e) {
          await captureArtifact(`${j.id}-throw`).catch(() => {});
          result = {
            id: j.id,
            result: ResultClass.FAIL,
            message: e instanceof Error ? e.message : String(e),
          };
        }
        result.durationMs = Date.now() - started;
        result.name = j.name;
        journeyResults.push(result);
        console.log(`[${result.result}] ${j.id}: ${result.message || ""}`);
        if (result.result === ResultClass.FAIL) {
          await captureArtifact(`${j.id}-fail`).catch(() => {});
        }
      }
      const overall = suiteResultFromJourneys(journeyResults);
      return {
        status:
          overall === ResultClass.PASS
            ? "pass"
            : overall === ResultClass.PARTIAL
              ? "partial"
              : overall === ResultClass.BLOCKED_BY_TOOLING
                ? "blocked_tooling"
                : overall === ResultClass.BLOCKED_BY_PRODUCT_GAP
                  ? "blocked_product_gap"
                  : "fail",
        classification: overall,
        message: `journeys ${journeyResults.filter((r) => r.result === ResultClass.PASS).length}/${journeyResults.length} PASS`,
        detail: { journeys: journeyResults },
      };
    });

    await report.run(StageId.APP_SHUTDOWN, async () => {
      const res = await client.deleteSession({ timeoutMs: 30_000 });
      await delay(800);
      return {
        status: "pass",
        message: `session deleted HTTP ${res.statusCode}`,
      };
    });

    await report.run(StageId.DRIVER_SHUTDOWN, async () => {
      if (driver) {
        const pid = driver.pid;
        await driver.stop();
        await delay(400);
        if (processAlive(pid)) {
          throw new Error(`tauri-driver pid ${pid} still alive`);
        }
        return { status: "pass", message: `driver stopped pid=${pid}` };
      }
      return { status: "pass", message: "no driver handle" };
    });

    await report.run(StageId.ORPHAN_VERIFY, async () => {
      await delay(600);
      let orphans = findOrphans(ORPHAN_CHECK_NAMES);
      if (orphans.length) {
        const reaped = reapOrphans(ORPHAN_CHECK_NAMES);
        orphans = findOrphans(ORPHAN_CHECK_NAMES);
        if (orphans.length) {
          throw Object.assign(
            new Error(`orphans remain: ${JSON.stringify(orphans)}`),
            { code: FailureCode.ORPHAN_PROCESS },
          );
        }
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message: "orphans reaped",
          detail: { reaped },
        };
      }
      // Patch GJ-12 if orphan clean
      const gj12 = journeyResults.find((j) => j.id === "GJ-12");
      if (gj12 && gj12.result === ResultClass.PASS) {
        gj12.message = "clean shutdown; no orphans after teardown";
        gj12.detail = { ...(gj12.detail || {}), orphans: [] };
      }
      return { status: "pass", message: "no orphans" };
    });

    // Temp cleanup (best effort)
    try {
      // keep artifacts; remove workDir logs only if clean
      if (process.env.TRACER_E2E_KEEP_TEMP !== "1") {
        // leave workDir for diagnosis on fail
        const overall = suiteResultFromJourneys(journeyResults);
        if (overall === ResultClass.PASS) {
          rmSync(workDir, { recursive: true, force: true });
        }
      }
    } catch {
      /* ignore */
    }

    return finish(report.summary(), {
      workDir,
      artifactsDir,
      rid,
      binary,
      journeyResults,
    });
  } catch (e) {
    console.error("[l3j-gui] FAILED:", e instanceof Error ? e.message : e);
    try {
      await captureArtifact("harness-fail");
    } catch {
      /* ignore */
    }
    try {
      await client.deleteSession().catch(() => {});
      await stopAllOwned();
      reapOrphans(ORPHAN_CHECK_NAMES);
    } catch {
      /* ignore */
    }
    const summary = report.summary();
    if (summary.result === ResultClass.PASS) summary.result = ResultClass.FAIL;
    return finish(
      summary,
      {
        workDir,
        artifactsDir,
        rid,
        journeyResults,
        error: e instanceof Error ? e.message : String(e),
        failureCode: e?.code || FailureCode.DRIVER_STARTUP_FAILED,
      },
      1,
    );
  }
}

function finish(summary, meta, exitHint) {
  const journeyOverall =
    meta.resultOverride ||
    (meta.journeyResults?.length
      ? suiteResultFromJourneys(meta.journeyResults)
      : summary.result);

  const out = {
    schemaVersion: 1,
    module: "W2.2-B",
    level: Level.L3J_PRODUCT_JOURNEY,
    result: journeyOverall,
    failureCode: meta.failureCode || null,
    stages: summary.stages,
    journeys: meta.journeyResults || [],
    meta: {
      runId: meta.rid,
      workDir: meta.workDir,
      artifactsDir: meta.artifactsDir,
      binary: meta.binary,
      error: meta.error,
      claimsL3J: true,
      ciClass: "windows_gui_runner | platform_gated_ci | manual_local",
      network: false,
      credentials: false,
      liveGrok: false,
      provider: false,
      fakeAcp: true,
      isolation:
        "NOT part of pnpm -r test or cargo test --workspace; explicit pnpm test:tauri-e2e:gui only",
    },
  };

  const reportPath = path.join(
    meta.artifactsDir || meta.workDir || process.cwd(),
    "l3j-report.json",
  );
  try {
    mkdirSync(path.dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, JSON.stringify(out, null, 2), "utf8");
  } catch {
    /* ignore */
  }

  if (jsonOut) {
    console.log(JSON.stringify(out, null, 2));
  } else {
    console.log("");
    console.log(`L3-J result: ${out.result}`);
    for (const j of out.journeys) {
      console.log(`  [${j.result}] ${j.id} ${j.name || ""} — ${j.message || ""}`);
    }
    for (const s of out.stages) {
      console.log(
        `  stage[${s.status}] ${s.id}${s.message ? " — " + s.message : ""}`,
      );
    }
    if (meta.failureCode) console.log(`failureCode: ${meta.failureCode}`);
    console.log(`report: ${reportPath}`);
  }

  if (exitHint != null) process.exitCode = exitHint;
  else {
    process.exitCode =
      out.result === ResultClass.FAIL ||
      out.result === ResultClass.BLOCKED_BY_PRODUCT_GAP
        ? 1
        : 0;
    // Tooling blocked is exit 0 for CI class separation (honest non-fail)
    if (out.result === ResultClass.BLOCKED_BY_TOOLING) process.exitCode = 0;
  }
  return out;
}

main();
