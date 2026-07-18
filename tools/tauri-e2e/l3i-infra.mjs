#!/usr/bin/env node
/**
 * L3-I — WebView driver infrastructure interaction (NOT product journey L3-J).
 *
 * Proves: tauri-driver startup → WebDriver session → app launch via driver →
 *         basic WebView interaction (title / root / optional Tauri detect) →
 *         session delete → driver shutdown → no orphans.
 *
 * Does NOT: session create/prompt/approval product flow through the GUI.
 *
 * If driver stack missing → BLOCKED_BY_TOOLING (not false PASS, not FAIL).
 */

import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { ResultClass, StageId, Level } from "./lib/classify.mjs";
import {
  REPO_ROOT,
  FAKE_ACP_JS,
  discoverEnvironment,
  resolvePreferredBinary,
} from "./lib/discover.mjs";
import {
  spawnOwned,
  processAlive,
  findOrphans,
  reapOrphans,
  uniqueTempDir,
  installExitHooks,
  stopAllOwned,
} from "./lib/process.mjs";
import { createStageReport } from "./lib/stages.mjs";
import { WebDriverClient, waitDriverReady } from "./lib/webdriver.mjs";

const args = new Set(process.argv.slice(2));
const jsonOut = args.has("--json");
const port = Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444);
const host = process.env.TRACER_TAURI_DRIVER_HOST || "127.0.0.1";
const baseUrl = `http://${host}:${port}`;

installExitHooks();

const ORPHAN_NAMES = [
  "tracer-desktop",
  "tracer_desktop",
  "tauri-driver",
  "msedgedriver",
  "WebKitWebDriver",
];

async function main() {
  const env = discoverEnvironment();
  const report = createStageReport();
  const workDir = uniqueTempDir("tracer-l3i");
  const logDir = path.join(workDir, "logs");
  mkdirSync(logDir, { recursive: true });
  const dbPath = path.join(workDir, "tracer-e2e.sqlite");

  /** @type {import('./lib/process.mjs').OwnedProcess | null} */
  let driver = null;
  const client = new WebDriverClient(baseUrl);

  console.log("=== L3-I WebView driver infrastructure (W2.2-A) ===");
  console.log(`workDir: ${workDir}`);
  console.log(`driver: ${baseUrl}`);
  console.log("NOTE: L3-J full product GUI journey is DEFERRED — not claimed.");

  try {
    // Platform
    if (!env.os.supportedForL3I_externalDriver) {
      for (const id of Object.values(StageId)) {
        report.skip(
          id,
          `external tauri-driver unsupported on ${env.os.platform}`,
          ResultClass.UNSUPPORTED_PLATFORM,
        );
      }
      return finish(report.summary(), {
        workDir,
        resultOverride: ResultClass.UNSUPPORTED_PLATFORM,
      });
    }

    // Resolve binary (do not auto-build here — keep L3-I focused; caller/L2 builds)
    let binary =
      process.env.TRACER_E2E_APP_BINARY ||
      resolvePreferredBinary()?.path ||
      null;

    await report.run(StageId.FRONTEND_BUILD, async () => {
      // L3-I assumes prior build; report status only
      const distOk = existsSync(path.join(REPO_ROOT, "apps/desktop/dist/index.html"));
      if (!distOk) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message: "frontend dist missing — run L2 build first",
        };
      }
      return { status: "pass", message: "frontend dist present" };
    });

    await report.run(StageId.BACKEND_BUILD, async () => {
      if (!binary || !existsSync(binary)) {
        return {
          status: "blocked_tooling",
          classification: ResultClass.BLOCKED_BY_TOOLING,
          message: "app binary missing — run L2 / cargo build -p tracer-desktop first",
        };
      }
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

    // Driver preflight
    const hasDriver = env.drivers.tauriDriver.available;
    const hasNative =
      env.os.platform === "win32"
        ? env.drivers.nativeDriver.msedgedriver.available
        : env.drivers.nativeDriver.webkitWebDriver.available;

    if (!hasDriver || !hasNative) {
      const msg = !hasDriver
        ? "tauri-driver not on PATH (cargo install tauri-driver --locked)"
        : env.os.platform === "win32"
          ? "msedgedriver not on PATH"
          : "WebKitWebDriver not on PATH";
      report.skip(StageId.DRIVER_STARTUP, msg, ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.APP_LAUNCH, "blocked: no driver", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.READINESS, "blocked: no driver", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.SMOKE, "blocked: no driver", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.APP_SHUTDOWN, "blocked: no driver", ResultClass.BLOCKED_BY_TOOLING);
      report.skip(StageId.DRIVER_SHUTDOWN, "blocked: no driver", ResultClass.BLOCKED_BY_TOOLING);
      await report.run(StageId.ORPHAN_VERIFY, async () => ({
        status: "pass",
        message: "no driver processes started",
      }));
      return finish(report.summary(), {
        workDir,
        resultOverride: ResultClass.BLOCKED_BY_TOOLING,
        setup: msg,
      });
    }

    if (!binary || !existsSync(binary)) {
      report.skip(
        StageId.DRIVER_STARTUP,
        "binary missing — not starting driver",
        ResultClass.BLOCKED_BY_TOOLING,
      );
      return finish(report.summary(), {
        workDir,
        resultOverride: ResultClass.BLOCKED_BY_TOOLING,
      });
    }

    // 4) driver startup
    await report.run(StageId.DRIVER_STARTUP, async () => {
      const driverArgs = ["--port", String(port)];
      if (process.env.TRACER_NATIVE_DRIVER) {
        driverArgs.push("--native-driver", process.env.TRACER_NATIVE_DRIVER);
      }
      driver = spawnOwned(env.drivers.tauriDriver.path, driverArgs, {
        label: "tauri-driver",
        logDir,
        windowsHide: true,
      });
      if (!driver.pid) throw new Error("failed to spawn tauri-driver");
      const status = await waitDriverReady(baseUrl, { timeoutMs: 25_000 });
      return {
        status: "pass",
        message: `tauri-driver ready pid=${driver.pid}`,
        detail: { pid: driver.pid, statusCode: status.statusCode, body: status.body },
      };
    });

    // 5) app launch via WebDriver session
    await report.run(StageId.APP_LAUNCH, async () => {
      const appEnv = {
        TRACER_DATABASE_PATH: dbPath,
        TRACER_FAKE_ACP_JS: FAKE_ACP_JS,
        TRACER_HELI_PROBE_PATH: path.join(workDir, "heli-empty"),
        TRACER_NODE_BIN: process.env.TRACER_NODE_BIN || "node",
      };
      mkdirSync(appEnv.TRACER_HELI_PROBE_PATH, { recursive: true });
      const res = await client.newSession(
        {
          application: binary,
          env: appEnv,
        },
        { timeoutMs: 90_000 },
      );
      if (!client.sessionId) {
        return {
          status: "fail",
          classification: ResultClass.FAIL,
          message: `WebDriver new session failed: HTTP ${res.statusCode} ${JSON.stringify(res.body).slice(0, 500)}`,
          detail: res,
        };
      }
      return {
        status: "pass",
        message: `session ${client.sessionId}`,
        detail: { sessionId: client.sessionId, statusCode: res.statusCode },
      };
    });

    // 6) readiness
    let title = null;
    let rootOk = null;
    let tauriDetect = null;
    await report.run(StageId.READINESS, async () => {
      await delay(2_000);
      try {
        const t = await client.getTitle();
        title = t.body?.value ?? t.body;
      } catch (e) {
        title = { error: e instanceof Error ? e.message : String(e) };
      }
      return {
        status: "pass",
        message: `WebDriver session responsive; title=${JSON.stringify(title)}`,
        detail: { title },
      };
    });

    // 7) infrastructure smoke (NOT product journey)
    await report.run(StageId.SMOKE, async () => {
      const checks = {
        driverSession: Boolean(client.sessionId),
        title: title,
        frontendRoot: null,
        tauriApiDetect: null,
        pageSourceSnippet: null,
      };

      // Try to find #root
      try {
        const execRoot = await client.execute(
          `return !!(document.getElementById('root') || document.body);`,
        );
        rootOk = execRoot.body?.value;
        checks.frontendRoot = rootOk;
      } catch (e) {
        checks.frontendRoot = {
          error: e instanceof Error ? e.message : String(e),
        };
      }

      // Tauri API detect
      try {
        const execTauri = await client.execute(
          `return !!(globalThis.__TAURI__ && globalThis.__TAURI__.core && globalThis.__TAURI__.core.invoke);`,
        );
        tauriDetect = execTauri.body?.value;
        checks.tauriApiDetect = tauriDetect;
      } catch (e) {
        checks.tauriApiDetect = {
          error: e instanceof Error ? e.message : String(e),
        };
      }

      // Optional app info via invoke — infrastructure, not journey
      try {
        const info = await client.execute(
          `return globalThis.__TAURI__?.core?.invoke ? globalThis.__TAURI__.core.invoke('tracer_app_info').then(v => JSON.stringify(v)).catch(e => String(e)) : 'no-tauri';`,
        );
        checks.appInfo = info.body?.value;
      } catch (e) {
        checks.appInfo = { error: e instanceof Error ? e.message : String(e) };
      }

      const hardFail = !checks.driverSession;
      if (hardFail) throw new Error("no driver session");

      const partial =
        checks.frontendRoot !== true || checks.tauriApiDetect !== true;

      return {
        status: partial ? "partial" : "pass",
        classification: partial ? ResultClass.PARTIAL : ResultClass.PASS,
        message: partial
          ? "driver session OK; some WebView probes incomplete (PARTIAL — still not L3-J)"
          : "driver + WebView infrastructure probes OK (L3-I only)",
        detail: { checks, claimsL3J: false },
      };
    });

    // 8) app shutdown via session delete
    await report.run(StageId.APP_SHUTDOWN, async () => {
      const res = await client.deleteSession({ timeoutMs: 30_000 });
      await delay(800);
      return {
        status: "pass",
        message: `session deleted HTTP ${res.statusCode}`,
        detail: { statusCode: res.statusCode },
      };
    });

    // 9) driver shutdown
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

    // 10) orphans
    await report.run(StageId.ORPHAN_VERIFY, async () => {
      await delay(500);
      let orphans = findOrphans(ORPHAN_NAMES);
      if (orphans.length) {
        const reaped = reapOrphans(ORPHAN_NAMES);
        orphans = findOrphans(ORPHAN_NAMES);
        if (orphans.length) {
          throw new Error(`orphans remain: ${JSON.stringify(orphans)}`);
        }
        return {
          status: "partial",
          classification: ResultClass.PARTIAL,
          message: "orphans reaped",
          detail: { reaped },
        };
      }
      return { status: "pass", message: "no orphans" };
    });

    return finish(report.summary(), { workDir, binary });
  } catch (e) {
    console.error("[l3i-infra] FAILED:", e instanceof Error ? e.message : e);
    try {
      await client.deleteSession().catch(() => {});
      await stopAllOwned();
      reapOrphans(ORPHAN_NAMES);
    } catch {
      /* ignore */
    }
    const summary = report.summary();
    if (summary.result === ResultClass.PASS) summary.result = ResultClass.FAIL;
    return finish(summary, {
      workDir,
      error: e instanceof Error ? e.message : String(e),
    }, 1);
  }
}

function finish(summary, meta, exitHint) {
  const out = {
    schemaVersion: 1,
    level: Level.L3I_WEBVIEW_INFRA,
    result: meta.resultOverride || summary.result,
    stages: summary.stages,
    meta: {
      workDir: meta.workDir,
      binary: meta.binary,
      error: meta.error,
      setup: meta.setup,
      claimsL3J: false,
      ciClass: "windows_gui_runner | platform_gated_ci | manual_local",
      network: false,
      credentials: false,
      liveGrok: false,
    },
  };
  if (jsonOut) {
    console.log(JSON.stringify(out, null, 2));
  } else {
    console.log("");
    console.log(`L3-I result: ${out.result}`);
    for (const s of out.stages) {
      console.log(
        `  [${s.status}] ${s.id}${s.message ? " — " + s.message : ""}`,
      );
    }
    if (meta.setup) console.log(`setup: ${meta.setup}`);
  }
  try {
    writeFileSync(
      path.join(meta.workDir, "l3i-report.json"),
      JSON.stringify(out, null, 2),
      "utf8",
    );
  } catch {
    /* ignore */
  }
  if (exitHint != null) process.exitCode = exitHint;
  else process.exitCode = out.result === ResultClass.FAIL ? 1 : 0;
  return out;
}

main();
