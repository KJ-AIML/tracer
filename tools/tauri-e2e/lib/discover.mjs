/**
 * Environment discovery for Tauri E2E infrastructure (W2.2-T).
 * Pure detection by default — no installs, no builds unless caller requests apply.
 */

import { spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  DoctorClass,
  FailureCode,
  ComponentId,
  ComponentStatus,
} from "./classify.mjs";
import {
  detectEdgeVersionWindows,
  evaluateEdgeDriverCompatibility,
  readMsEdgeDriverVersion,
  resolveLocalMsEdgeDriver,
} from "../../tauri-driver/lib/edge.mjs";
import {
  resolveTauriDriverPath,
  readTauriDriverVersion,
  cargoBinDir,
} from "../../tauri-driver/lib/install.mjs";
import { redactPath } from "../../tauri-driver/lib/paths.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(__dirname, "../../..");
export const DESKTOP_DIR = path.join(REPO_ROOT, "apps/desktop");
export const SRC_TAURI = path.join(DESKTOP_DIR, "src-tauri");
export const FRONTEND_DIST = path.join(DESKTOP_DIR, "dist");
export const FAKE_ACP_JS = path.join(
  REPO_ROOT,
  "tools/fake-acp-runtime/bin/fake-acp-runtime.js",
);

const BINARY_NAMES =
  process.platform === "win32"
    ? ["tracer-desktop.exe", "tracer_desktop.exe"]
    : ["tracer-desktop", "tracer_desktop"];

function quoteWinArg(a) {
  const s = String(a);
  if (!/[\s"]/u.test(s)) return s;
  return `"${s.replace(/"/g, '\\"')}"`;
}

function tryCmd(cmd, args, opts = {}) {
  try {
    /** @type {import('node:child_process').SpawnSyncReturns<string>} */
    let r;
    if (process.platform === "win32" && opts.direct !== true) {
      const line = [cmd, ...args].map(quoteWinArg).join(" ");
      r = spawnSync(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", line], {
        encoding: "utf8",
        windowsHide: true,
        timeout: opts.timeout ?? 15_000,
        env: process.env,
      });
    } else {
      r = spawnSync(cmd, args, {
        encoding: "utf8",
        windowsHide: true,
        timeout: opts.timeout ?? 15_000,
        env: process.env,
      });
    }
    if (r.error) return { ok: false, error: r.error.message, stdout: "", stderr: "" };
    return {
      ok: r.status === 0,
      status: r.status,
      stdout: (r.stdout || "").trim(),
      stderr: (r.stderr || "").trim(),
    };
  } catch (e) {
    return {
      ok: false,
      error: e instanceof Error ? e.message : String(e),
      stdout: "",
      stderr: "",
    };
  }
}

function which(bin) {
  if (process.platform === "win32") {
    for (const name of [`${bin}.cmd`, `${bin}.exe`, `${bin}.bat`, bin]) {
      const r = tryCmd("where.exe", [name], { timeout: 8_000, direct: true });
      if (!r.ok || !r.stdout) continue;
      const first = r.stdout.split(/\r?\n/).map((s) => s.trim()).find(Boolean);
      if (first) return first;
    }
    return null;
  }
  const r = tryCmd("which", [bin], { timeout: 8_000, direct: true });
  if (!r.ok || !r.stdout) return null;
  return r.stdout.split(/\r?\n/).map((s) => s.trim()).find(Boolean) || null;
}

function parseVersionToken(text) {
  if (!text) return null;
  const m = text.match(/v?(\d+\.\d+\.\d+(?:[-+][\w.]+)?)/);
  return m ? m[1] : text.split(/\s+/)[0] || null;
}

function readWebView2VersionWindows() {
  if (process.platform !== "win32") return null;
  const keys = [
    "HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
  ];
  for (const key of keys) {
    const r = tryCmd("reg", ["query", key, "/v", "pv"], { timeout: 8_000 });
    if (!r.ok) continue;
    const m = r.stdout.match(/pv\s+REG_SZ\s+(\S+)/i);
    if (m) return m[1];
  }
  return null;
}

/**
 * Candidate binary paths for built tracer-desktop.
 * Prefer workspace target/ then src-tauri/target.
 */
export function findAppBinaries({ preferRelease = true } = {}) {
  const profiles = preferRelease
    ? ["release", "debug"]
    : ["debug", "release"];
  const roots = [
    path.join(REPO_ROOT, "target"),
    path.join(SRC_TAURI, "target"),
  ];
  const found = [];
  for (const root of roots) {
    for (const profile of profiles) {
      for (const name of BINARY_NAMES) {
        const p = path.join(root, profile, name);
        if (existsSync(p)) {
          let size = null;
          let mtime = null;
          try {
            const st = statSync(p);
            size = st.size;
            mtime = st.mtime.toISOString();
          } catch {
            /* ignore */
          }
          found.push({ path: p, profile, size, mtime });
        }
      }
    }
  }
  return found;
}

export function resolvePreferredBinary(opts = {}) {
  const bins = findAppBinaries(opts);
  return bins[0] ?? null;
}

function detectTauriCli() {
  const localCandidates = [
    path.join(DESKTOP_DIR, "node_modules", "@tauri-apps", "cli"),
    path.join(REPO_ROOT, "node_modules", "@tauri-apps", "cli"),
  ];
  let localPath = null;
  for (const c of localCandidates) {
    if (existsSync(c)) {
      localPath = c;
      break;
    }
  }

  const cargoTauri = which("cargo-tauri") || which("tauri");
  let version = null;
  if (localPath) {
    const r = tryCmd("pnpm", ["--filter", "@tracer/desktop", "exec", "tauri", "--version"], {
      timeout: 20_000,
    });
    if (r.ok) version = parseVersionToken(r.stdout || r.stderr);
  }
  if (!version && cargoTauri) {
    const r = tryCmd(cargoTauri, ["--version"]);
    if (r.ok) version = parseVersionToken(r.stdout || r.stderr);
  }
  return {
    available: Boolean(localPath || cargoTauri),
    localPath: redactPath(localPath),
    path: redactPath(cargoTauri),
    version,
  };
}

function detectDriverStack() {
  const tauriPath = resolveTauriDriverPath();
  const tauri = readTauriDriverVersion(tauriPath);

  let msedgedriverPath =
    resolveLocalMsEdgeDriver() ||
    process.env.TRACER_NATIVE_DRIVER ||
    which("msedgedriver");
  const msedgedriver = readMsEdgeDriverVersion(msedgedriverPath);

  const chromedriverPath = which("chromedriver");
  const webkitPath = which("WebKitWebDriver");

  const edge =
    process.platform === "win32"
      ? detectEdgeVersionWindows()
      : {
          available: false,
          path: null,
          version: null,
          major: null,
          method: "n/a",
        };

  const compatibility =
    process.platform === "win32"
      ? evaluateEdgeDriverCompatibility(edge, {
          ...msedgedriver,
          path: msedgedriverPath,
        })
      : {
          compatible: Boolean(webkitPath || process.env.TRACER_NATIVE_DRIVER),
          code: webkitPath ? "WEBKIT_PRESENT" : "NATIVE_DRIVER_CHECK",
          message: "non-Windows native driver path",
        };

  return {
    tauriDriver: {
      available: Boolean(tauri.available),
      path: tauri.path,
      pathRedacted: redactPath(tauri.path),
      version: tauri.version,
      source: tauri.path
        ? tauri.path.includes(`${path.sep}tools${path.sep}tauri-driver`)
          ? "project"
          : "path_or_cargo"
        : null,
    },
    edgeBrowser: {
      available: Boolean(edge.available),
      path: redactPath(edge.path),
      version: edge.version,
      major: edge.major,
      method: edge.method,
    },
    nativeDriver: {
      msedgedriver: {
        available: Boolean(msedgedriver.available),
        path: msedgedriver.path,
        pathRedacted: redactPath(msedgedriver.path),
        version: msedgedriver.version,
        major: msedgedriver.major,
        versionVerified: Boolean(msedgedriver.version),
        compatibility,
      },
      chromedriver: chromedriverPath
        ? { available: true, path: redactPath(chromedriverPath) }
        : { available: false, path: null },
      webkitWebDriver: webkitPath
        ? { available: true, path: redactPath(webkitPath) }
        : process.env.TRACER_NATIVE_DRIVER && process.platform === "linux"
          ? {
              available: true,
              path: redactPath(process.env.TRACER_NATIVE_DRIVER),
            }
          : { available: false, path: null },
    },
  };
}

/**
 * Check if a TCP port is free on 127.0.0.1 (async → sync via promise runner).
 * @param {number} port
 */
export function checkPortAvailable(port, host = "127.0.0.1") {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.unref();
    server.once("error", (err) => {
      resolve({
        port,
        host,
        available: false,
        code: err && err.code === "EADDRINUSE" ? FailureCode.PORT_IN_USE : FailureCode.PORT_CHECK_FAILED,
        error: err ? err.code || String(err) : "unknown",
      });
    });
    server.once("listening", () => {
      server.close(() => {
        resolve({ port, host, available: true, code: null, error: null });
      });
    });
    try {
      server.listen(port, host);
    } catch (e) {
      resolve({
        port,
        host,
        available: false,
        code: FailureCode.PORT_CHECK_FAILED,
        error: e instanceof Error ? e.message : String(e),
      });
    }
  });
}

export function checkPortAvailableSync(port, host = "127.0.0.1") {
  // deasync-free: use spawn to run a tiny node check would be heavy;
  // use net with spin via Atomics wait is complex. Prefer spawnSync node -e.
  const script = `
    const net=require('net');
    const s=net.createServer();
    s.once('error',e=>{console.log(JSON.stringify({available:false,error:e.code||String(e)}));process.exit(0)});
    s.once('listening',()=>{s.close(()=>{console.log(JSON.stringify({available:true}));});});
    s.listen(${Number(port)}, ${JSON.stringify(host)});
  `;
  const r = spawnSync(process.execPath, ["-e", script], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 5_000,
  });
  try {
    const j = JSON.parse((r.stdout || "").trim() || "{}");
    return {
      port,
      host,
      available: Boolean(j.available),
      code: j.available
        ? null
        : j.error === "EADDRINUSE"
          ? FailureCode.PORT_IN_USE
          : FailureCode.PORT_CHECK_FAILED,
      error: j.error || null,
    };
  } catch {
    return {
      port,
      host,
      available: false,
      code: FailureCode.PORT_CHECK_FAILED,
      error: r.stderr || "parse_failed",
    };
  }
}

function detectProcessCleanupCapability() {
  if (process.platform === "win32") {
    const taskkill = which("taskkill") || existsSync(
      path.join(process.env.SystemRoot || "C:\\Windows", "System32", "taskkill.exe"),
    );
    const tasklist = which("tasklist") || existsSync(
      path.join(process.env.SystemRoot || "C:\\Windows", "System32", "tasklist.exe"),
    );
    const ok = Boolean(taskkill && tasklist);
    return {
      available: ok,
      method: "taskkill /T + tasklist",
      tools: {
        taskkill: Boolean(taskkill),
        tasklist: Boolean(tasklist),
      },
    };
  }
  const pgrep = Boolean(which("pgrep"));
  const kill = Boolean(which("kill"));
  return {
    available: kill,
    method: "process.kill / pgrep -f",
    tools: { pgrep, kill },
  };
}

/**
 * Full environment report used by doctor and runners.
 */
export function discoverEnvironment() {
  const platform = process.platform;
  const arch = process.arch;
  const osRelease = os.release();
  const osType = os.type();

  const rustc = tryCmd("rustc", ["--version"]);
  const cargo = tryCmd("cargo", ["--version"]);
  const node = tryCmd("node", ["--version"]);
  const pnpmPath = which("pnpm");
  const pnpm = pnpmPath
    ? tryCmd(pnpmPath, ["--version"])
    : tryCmd("pnpm", ["--version"]);

  const webview2Version = platform === "win32" ? readWebView2VersionWindows() : null;
  const webview2 =
    platform === "win32"
      ? { available: Boolean(webview2Version), version: webview2Version }
      : platform === "darwin"
        ? { available: true, version: "WKWebView (system)", note: "macOS uses WKWebView" }
        : {
            available: Boolean(which("webkit2gtk-4.1") || which("WebKitWebDriver")),
            version: null,
            note: "Linux WebKitGTK required for Tauri WebView",
          };

  const drivers = detectDriverStack();
  const tauriCli = detectTauriCli();
  const binaries = findAppBinaries();
  const preferredBinary = binaries[0] ?? null;

  const frontendDistPresent = existsSync(path.join(FRONTEND_DIST, "index.html"));
  const fakeAcp = existsSync(FAKE_ACP_JS);

  const tauriDriverPort = Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444);
  const ports = {
    viteDev: 1420,
    tauriDriverDefault: tauriDriverPort,
    tauriDriver: checkPortAvailableSync(tauriDriverPort),
  };

  const processCleanup = detectProcessCleanupCapability();
  const buildProfile =
    preferredBinary?.profile ?? process.env.TRACER_E2E_PROFILE ?? "debug";

  const env = {
    discoveredAt: new Date().toISOString(),
    module: "W2.2-T",
    os: {
      platform,
      type: osType,
      release: osRelease,
      arch,
      supportedForL2: platform === "win32" || platform === "linux" || platform === "darwin",
      supportedForL3I_externalDriver: platform === "win32" || platform === "linux",
      note:
        platform === "darwin"
          ? "L3-I external tauri-driver unsupported on macOS; use embedded WDIO path (future)"
          : null,
    },
    rust: {
      rustc: rustc.ok ? rustc.stdout : null,
      cargo: cargo.ok ? cargo.stdout : null,
      available: rustc.ok && cargo.ok,
      cargoBin: redactPath(cargoBinDir()),
    },
    node: {
      version: node.ok ? node.stdout : null,
      available: node.ok,
      path: redactPath(which("node")),
    },
    pnpm: {
      version: pnpm.ok ? pnpm.stdout : null,
      available: pnpm.ok,
      path: redactPath(which("pnpm")),
    },
    tauriCli,
    webview: webview2,
    drivers,
    processCleanup,
    paths: {
      repoRoot: REPO_ROOT,
      desktop: DESKTOP_DIR,
      srcTauri: SRC_TAURI,
      frontendDist: FRONTEND_DIST,
      frontendDistPresent,
      fakeAcpJs: FAKE_ACP_JS,
      fakeAcpPresent: fakeAcp,
      appBinary: preferredBinary?.path ?? null,
      appBinaryRedacted: redactPath(preferredBinary?.path ?? null),
      appBinaries: binaries.map((b) => ({
        ...b,
        path: b.path,
        pathRedacted: redactPath(b.path),
      })),
    },
    build: {
      profile: buildProfile,
      binaryFound: Boolean(preferredBinary),
      frontendDistPresent,
    },
    ports,
    e2eEnvHooks: [
      "TRACER_DATABASE_PATH",
      "TRACER_FAKE_ACP_JS",
      "TRACER_HELI_PROBE_PATH",
      "TRACER_NODE_BIN",
      "TRACER_TAURI_DRIVER_PORT",
      "TRACER_TAURI_DRIVER",
      "TRACER_E2E_PROFILE",
      "TRACER_E2E_APP_BINARY",
      "TRACER_NATIVE_DRIVER",
      "TRACER_TAURI_E2E_SETUP",
      "TRACER_EDGE_BINARY",
    ],
  };

  return env;
}

/**
 * Component matrix for Gate 2.2.2 doctor.
 */
export function componentStatus(env) {
  const platform = env.os.platform;
  const comps = [];

  // TAURI_DRIVER
  comps.push({
    id: ComponentId.TAURI_DRIVER,
    status: env.drivers.tauriDriver.available
      ? ComponentStatus.OK
      : ComponentStatus.MISSING,
    version: env.drivers.tauriDriver.version,
    path: env.drivers.tauriDriver.pathRedacted,
    code: env.drivers.tauriDriver.available
      ? null
      : FailureCode.TAURI_DRIVER_NOT_FOUND,
  });

  // EDGE_BROWSER
  if (platform === "win32") {
    let st = ComponentStatus.MISSING;
    let code = FailureCode.EDGE_BROWSER_NOT_FOUND;
    if (env.drivers.edgeBrowser.available && env.drivers.edgeBrowser.major != null) {
      st = ComponentStatus.OK;
      code = null;
    } else if (env.drivers.edgeBrowser.available) {
      st = ComponentStatus.UNVERIFIED;
      code = FailureCode.EDGE_BROWSER_VERSION_UNKNOWN;
    }
    comps.push({
      id: ComponentId.EDGE_BROWSER,
      status: st,
      version: env.drivers.edgeBrowser.version,
      path: env.drivers.edgeBrowser.path,
      code,
    });
  } else {
    comps.push({
      id: ComponentId.EDGE_BROWSER,
      status: ComponentStatus.NA,
      version: null,
      path: null,
      code: null,
      note: "Windows-only component",
    });
  }

  // WEBVIEW2_RUNTIME
  if (platform === "win32") {
    comps.push({
      id: ComponentId.WEBVIEW2_RUNTIME,
      status: env.webview.available ? ComponentStatus.OK : ComponentStatus.MISSING,
      version: env.webview.version,
      path: null,
      code: env.webview.available ? null : FailureCode.WEBVIEW2_NOT_FOUND,
    });
  } else {
    comps.push({
      id: ComponentId.WEBVIEW2_RUNTIME,
      status: env.webview.available ? ComponentStatus.OK : ComponentStatus.MISSING,
      version: env.webview.version,
      path: null,
      code: null,
      note: env.webview.note || "platform webview",
    });
  }

  // EDGE_DRIVER
  if (platform === "win32") {
    const c = env.drivers.nativeDriver.msedgedriver.compatibility;
    let st = ComponentStatus.MISSING;
    let code = FailureCode.EDGE_DRIVER_NOT_FOUND;
    if (c?.compatible) {
      st = ComponentStatus.OK;
      code = null;
    } else if (c?.code === "EDGE_DRIVER_VERSION_MISMATCH") {
      st = ComponentStatus.MISMATCH;
      code = FailureCode.EDGE_DRIVER_VERSION_MISMATCH;
    } else if (c?.code === "EDGE_DRIVER_VERSION_UNVERIFIED") {
      st = ComponentStatus.UNVERIFIED;
      code = FailureCode.EDGE_DRIVER_VERSION_UNVERIFIED;
    } else if (env.drivers.nativeDriver.msedgedriver.available) {
      st = ComponentStatus.UNVERIFIED;
      code = c?.code || FailureCode.EDGE_DRIVER_VERSION_UNVERIFIED;
    }
    comps.push({
      id: ComponentId.EDGE_DRIVER,
      status: st,
      version: env.drivers.nativeDriver.msedgedriver.version,
      path: env.drivers.nativeDriver.msedgedriver.pathRedacted,
      code,
      compatibility: c,
    });
  } else if (platform === "linux") {
    const ok = env.drivers.nativeDriver.webkitWebDriver.available;
    comps.push({
      id: ComponentId.EDGE_DRIVER,
      status: ok ? ComponentStatus.OK : ComponentStatus.MISSING,
      version: null,
      path: env.drivers.nativeDriver.webkitWebDriver.path,
      code: ok ? null : FailureCode.EDGE_DRIVER_NOT_FOUND,
      note: "Linux uses WebKitWebDriver (mapped to EDGE_DRIVER slot)",
    });
  } else {
    comps.push({
      id: ComponentId.EDGE_DRIVER,
      status: ComponentStatus.NA,
      version: null,
      path: null,
      code: FailureCode.UNSUPPORTED_PLATFORM,
    });
  }

  // APPLICATION_BINARY
  comps.push({
    id: ComponentId.APPLICATION_BINARY,
    status: env.build.binaryFound ? ComponentStatus.OK : ComponentStatus.MISSING,
    version: env.build.profile,
    path: env.paths.appBinaryRedacted,
    code: env.build.binaryFound ? null : FailureCode.APP_BINARY_NOT_FOUND,
  });

  // FRONTEND_DIST
  comps.push({
    id: ComponentId.FRONTEND_DIST,
    status: env.build.frontendDistPresent
      ? ComponentStatus.OK
      : ComponentStatus.MISSING,
    version: null,
    path: "apps/desktop/dist",
    code: env.build.frontendDistPresent
      ? null
      : FailureCode.FRONTEND_DIST_NOT_FOUND,
  });

  // PORT_AVAILABILITY
  const port = env.ports.tauriDriver;
  comps.push({
    id: ComponentId.PORT_AVAILABILITY,
    status: port.available ? ComponentStatus.OK : ComponentStatus.IN_USE,
    version: null,
    path: `${port.host}:${port.port}`,
    code: port.available ? null : port.code || FailureCode.PORT_IN_USE,
  });

  // PROCESS_CLEANUP_CAPABILITY
  comps.push({
    id: ComponentId.PROCESS_CLEANUP_CAPABILITY,
    status: env.processCleanup.available
      ? ComponentStatus.OK
      : ComponentStatus.MISSING,
    version: null,
    path: null,
    code: env.processCleanup.available
      ? null
      : FailureCode.PROCESS_CLEANUP_UNAVAILABLE,
    method: env.processCleanup.method,
  });

  return comps;
}

/**
 * Derive doctor issues from environment report.
 */
export function doctorIssues(env) {
  const issues = [];

  if (!env.os.supportedForL2) {
    issues.push({
      class: DoctorClass.UNSUPPORTED_PLATFORM,
      code: FailureCode.UNSUPPORTED_PLATFORM,
      message: `OS platform ${env.os.platform} is not supported for Tauri desktop E2E`,
      setup: "Use Windows, Linux, or macOS host with GUI session",
      fallback: "Run L0 invoke policy + L1 desktop boundary on any platform",
    });
  }

  if (!env.rust.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: FailureCode.RUST_NOT_FOUND,
      message: "rustc/cargo not available on PATH",
      setup: "Install Rust toolchain: https://rustup.rs",
      fallback: "L0 frontend policy only (vitest)",
    });
  }

  if (!env.node.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: FailureCode.NODE_NOT_FOUND,
      message: "node not available on PATH (required for harness + fake ACP)",
      setup: "Install Node.js >= 20",
      fallback: "None for full harness",
    });
  } else {
    const ver = parseVersionToken(env.node.version || "");
    const major = ver ? Number(ver.split(".")[0]) : 0;
    if (major && major < 20) {
      issues.push({
        class: DoctorClass.INCOMPATIBLE_VERSION,
        code: FailureCode.NODE_VERSION_INCOMPATIBLE,
        message: `Node ${env.node.version} < 20`,
        setup: "Upgrade to Node.js >= 20",
        fallback: "May still run with older node; unsupported",
      });
    }
  }

  if (!env.pnpm.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: FailureCode.PNPM_NOT_FOUND,
      message: "pnpm not available on PATH",
      setup: "corepack enable && corepack prepare pnpm@9.15.0 --activate",
      fallback:
        "node tools/tauri-e2e/*.mjs still works for doctor/L2 if cargo+node present",
    });
  }

  if (!env.paths.fakeAcpPresent) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: FailureCode.FAKE_ACP_NOT_FOUND,
      message: `fake ACP runtime missing: ${env.paths.fakeAcpJs}`,
      setup: "Ensure tools/fake-acp-runtime is present in worktree",
      fallback: "L2 process smoke without ACP still possible",
    });
  }

  if (env.os.platform === "win32" && !env.webview.available) {
    issues.push({
      class: DoctorClass.WEBVIEW_UNAVAILABLE,
      code: FailureCode.WEBVIEW2_NOT_FOUND,
      message: "WebView2 Runtime not detected in registry",
      setup:
        "Install Evergreen WebView2 Runtime: https://developer.microsoft.com/microsoft-edge/webview2/",
      fallback: "L0+L1 only; L2 launch will fail or crash",
    });
  }

  if (!env.build.binaryFound) {
    issues.push({
      class: DoctorClass.BUILD_REQUIRED,
      code: FailureCode.APP_BINARY_NOT_FOUND,
      message: "tracer-desktop binary not found under target/{debug,release}",
      setup:
        "From repo root: pnpm --filter @tracer/desktop build ; cargo build -p tracer-desktop",
      fallback: "L0+L1 do not require packaged binary",
    });
  }

  if (!env.build.frontendDistPresent) {
    issues.push({
      class: DoctorClass.BUILD_REQUIRED,
      code: FailureCode.FRONTEND_DIST_NOT_FOUND,
      message:
        "apps/desktop/dist/index.html missing (required for real app frontend)",
      setup: "pnpm --filter @tracer/desktop build",
      fallback:
        "L1 cargo tests can use dist stub; L2 real smoke needs Vite build",
    });
  }

  // Driver stack for L3-I
  if (!env.os.supportedForL3I_externalDriver) {
    issues.push({
      class: DoctorClass.UNSUPPORTED_PLATFORM,
      code: FailureCode.UNSUPPORTED_PLATFORM,
      message: `External tauri-driver path unsupported on ${env.os.platform}`,
      setup:
        "Use Windows/Linux for L3-I external driver, or future embedded WDIO path on macOS",
      fallback: "L0+L1+L2 process smoke only",
    });
  } else if (!env.drivers.tauriDriver.available) {
    issues.push({
      class: DoctorClass.DRIVER_UNAVAILABLE,
      code: FailureCode.TAURI_DRIVER_NOT_FOUND,
      message: "tauri-driver not found (PATH, cargo bin, or tools/tauri-driver/bin)",
      setup:
        "cargo install tauri-driver --locked  OR  node tools/tauri-driver/setup.mjs --apply",
      fallback: "L0+L1+L2 without WebDriver interaction",
    });
  }

  if (env.os.platform === "win32" && env.os.supportedForL3I_externalDriver) {
    if (!env.drivers.edgeBrowser.available) {
      issues.push({
        class: DoctorClass.MISSING_TOOL,
        code: FailureCode.EDGE_BROWSER_NOT_FOUND,
        message: "Microsoft Edge browser not found",
        setup: "Install Microsoft Edge",
        fallback: "L0+L1+L2 may still work with WebView2 alone",
      });
    }
    const compat = env.drivers.nativeDriver.msedgedriver.compatibility;
    if (!compat?.compatible) {
      const isMismatch = compat?.code === "EDGE_DRIVER_VERSION_MISMATCH";
      issues.push({
        class: isMismatch
          ? DoctorClass.INCOMPATIBLE_VERSION
          : DoctorClass.DRIVER_UNAVAILABLE,
        code: compat?.code || FailureCode.EDGE_DRIVER_NOT_FOUND,
        message:
          compat?.message ||
          "msedgedriver missing or incompatible with installed Edge major",
        setup:
          "node tools/tauri-driver/setup.mjs --apply   # downloads matching driver to project cache",
        fallback: "L0+L1+L2 process smoke only",
        rule: compat?.rule,
      });
    }
  } else if (
    env.os.platform === "linux" &&
    env.os.supportedForL3I_externalDriver &&
    !env.drivers.nativeDriver.webkitWebDriver.available
  ) {
    issues.push({
      class: DoctorClass.DRIVER_UNAVAILABLE,
      code: FailureCode.EDGE_DRIVER_NOT_FOUND,
      message: "WebKitWebDriver not on PATH",
      setup: "Install webkit2gtk-driver (Debian) or distribution equivalent",
      fallback: "L0+L1+L2 process smoke only",
    });
  }

  if (env.ports.tauriDriver && !env.ports.tauriDriver.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: FailureCode.PORT_IN_USE,
      message: `tauri-driver port ${env.ports.tauriDriver.port} is not available (${env.ports.tauriDriver.error || "in use"})`,
      setup: `Free port or set TRACER_TAURI_DRIVER_PORT to a free port`,
      fallback: "L0+L1+L2 without driver",
    });
  }

  if (!env.processCleanup.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: FailureCode.PROCESS_CLEANUP_UNAVAILABLE,
      message: "process tree cleanup tools unavailable",
      setup: "Ensure taskkill/tasklist (Windows) or kill (Unix) are available",
      fallback: "Harness may leave orphans on failure",
    });
  }

  // Tauri CLI optional
  if (!env.tauriCli.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: "tauri_cli",
      message:
        "@tauri-apps/cli / cargo-tauri not detected (optional for cargo-only builds)",
      setup:
        "pnpm --filter @tracer/desktop add -D @tauri-apps/cli  OR  cargo install tauri-cli --locked",
      fallback: "cargo build -p tracer-desktop still works for L2 binary",
    });
  }

  return issues;
}

/**
 * Levels this environment can attempt.
 */
export function capabilityMatrix(env, issues) {
  const classes = new Set(issues.map((i) => i.class));
  const codes = new Set(issues.map((i) => i.code));

  const l0 = env.node.available && env.pnpm.available;
  const l1 = l0 && env.rust.available && env.paths.fakeAcpPresent;
  const l2 =
    env.os.supportedForL2 &&
    env.rust.available &&
    env.node.available &&
    !classes.has(DoctorClass.UNSUPPORTED_PLATFORM) &&
    !(env.os.platform === "win32" && !env.webview.available);

  const edgeOk =
    env.os.platform !== "win32" ||
    env.drivers.nativeDriver.msedgedriver.compatibility?.compatible === true;
  const nativeOk =
    env.os.platform === "win32"
      ? edgeOk
      : env.drivers.nativeDriver.webkitWebDriver.available;

  const l3i =
    l2 &&
    env.os.supportedForL3I_externalDriver &&
    env.drivers.tauriDriver.available &&
    nativeOk &&
    env.build.binaryFound &&
    env.build.frontendDistPresent &&
    env.ports.tauriDriver.available;

  return {
    L0: { attemptable: l0, claim: "executable when pnpm/vitest available" },
    L1: { attemptable: l1, claim: "executable when cargo + fake ACP available" },
    L2: {
      attemptable: l2,
      claim: l2
        ? env.build.binaryFound
          ? "attemptable (binary present)"
          : "attemptable after build"
        : "blocked",
      needsBuild: !env.build.binaryFound || !env.build.frontendDistPresent,
    },
    "L3-I": {
      attemptable: l3i,
      claim: l3i
        ? "attemptable (driver stack + build present)"
        : "blocked until tauri-driver + compatible native WebDriver + build",
    },
    "L3-J": {
      attemptable: false,
      claim:
        "NOT_STARTED — owned by future W2.2-B product journey; not claimed by W2.2-T",
    },
    blockers: [...codes],
  };
}

/** Convenience one-shot for doctor. */
export function runDiscovery() {
  const env = discoverEnvironment();
  const issues = doctorIssues(env);
  const capabilities = capabilityMatrix(env, issues);
  const components = componentStatus(env);
  return { env, issues, capabilities, components };
}

/**
 * Resolve absolute paths for L3-I process spawn (not redacted).
 */
export function resolveDriverSpawnPaths() {
  const tauriPath = resolveTauriDriverPath();
  const nativePath =
    resolveLocalMsEdgeDriver() ||
    process.env.TRACER_NATIVE_DRIVER ||
    which("msedgedriver") ||
    which("WebKitWebDriver");
  return { tauriPath, nativePath };
}