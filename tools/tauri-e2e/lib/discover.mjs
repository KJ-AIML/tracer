/**
 * Environment discovery for Tauri E2E infrastructure (W2.2-A).
 * Pure detection — no installs, no builds unless caller requests.
 */

import { spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { DoctorClass } from "./classify.mjs";

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
      // cmd.exe /c runs .CMD shims; avoids spawn EINVAL and shell:true deprecation.
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
    return { ok: false, error: e instanceof Error ? e.message : String(e), stdout: "", stderr: "" };
  }
}

function which(bin) {
  if (process.platform === "win32") {
    // Prefer .cmd/.exe over extensionless POSIX shims that Node cannot spawn.
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
  // Prefer local workspace CLI if present, else cargo install / PATH.
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
    localPath,
    path: cargoTauri,
    version,
  };
}

function detectDriver() {
  const tauriDriver = which("tauri-driver");
  const msedgedriver = which("msedgedriver");
  const chromedriver = which("chromedriver");
  const webkitDriver = which("WebKitWebDriver");

  let tauriDriverVersion = null;
  if (tauriDriver) {
    const r = tryCmd(tauriDriver, ["--version"], { timeout: 8_000 });
    // tauri-driver may not support --version; treat existence as enough.
    tauriDriverVersion = r.ok
      ? parseVersionToken(r.stdout || r.stderr) || "present"
      : "present";
  }

  return {
    tauriDriver: {
      available: Boolean(tauriDriver),
      path: tauriDriver,
      version: tauriDriverVersion,
    },
    nativeDriver: {
      msedgedriver: msedgedriver
        ? { available: true, path: msedgedriver }
        : { available: false, path: null },
      chromedriver: chromedriver
        ? { available: true, path: chromedriver }
        : { available: false, path: null },
      webkitWebDriver: webkitDriver
        ? { available: true, path: webkitDriver }
        : { available: false, path: null },
    },
  };
}

/**
 * Full environment report used by doctor and runners.
 */
export function discoverEnvironment() {
  const platform = process.platform; // win32 | darwin | linux
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

  const webview2 =
    platform === "win32"
      ? { available: Boolean(readWebView2VersionWindows()), version: readWebView2VersionWindows() }
      : platform === "darwin"
        ? { available: true, version: "WKWebView (system)", note: "macOS uses WKWebView" }
        : {
            available: Boolean(which("webkit2gtk-4.1") || which("WebKitWebDriver")),
            version: null,
            note: "Linux WebKitGTK required for Tauri WebView",
          };

  const drivers = detectDriver();
  const tauriCli = detectTauriCli();
  const binaries = findAppBinaries();
  const preferredBinary = binaries[0] ?? null;

  const frontendDistPresent =
    existsSync(path.join(FRONTEND_DIST, "index.html")) ||
    existsSync(FRONTEND_DIST);

  const fakeAcp = existsSync(FAKE_ACP_JS);

  const ports = {
    viteDev: 1420,
    tauriDriverDefault: Number(process.env.TRACER_TAURI_DRIVER_PORT || 4444),
  };

  const buildProfile = preferredBinary?.profile ?? process.env.TRACER_E2E_PROFILE ?? "debug";

  const env = {
    discoveredAt: new Date().toISOString(),
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
    },
    node: {
      version: node.ok ? node.stdout : null,
      available: node.ok,
      path: which("node"),
    },
    pnpm: {
      version: pnpm.ok ? pnpm.stdout : null,
      available: pnpm.ok,
      path: which("pnpm"),
    },
    tauriCli,
    webview: webview2,
    drivers,
    paths: {
      repoRoot: REPO_ROOT,
      desktop: DESKTOP_DIR,
      srcTauri: SRC_TAURI,
      frontendDist: FRONTEND_DIST,
      frontendDistPresent: Boolean(
        existsSync(path.join(FRONTEND_DIST, "index.html")),
      ),
      fakeAcpJs: FAKE_ACP_JS,
      fakeAcpPresent: fakeAcp,
      appBinary: preferredBinary?.path ?? null,
      appBinaries: binaries,
    },
    build: {
      profile: buildProfile,
      binaryFound: Boolean(preferredBinary),
      frontendDistPresent: Boolean(
        existsSync(path.join(FRONTEND_DIST, "index.html")),
      ),
    },
    ports,
    e2eEnvHooks: [
      "TRACER_DATABASE_PATH",
      "TRACER_FAKE_ACP_JS",
      "TRACER_HELI_PROBE_PATH",
      "TRACER_NODE_BIN",
      "TRACER_TAURI_DRIVER_PORT",
      "TRACER_E2E_PROFILE",
      "TRACER_E2E_APP_BINARY",
      "TRACER_NATIVE_DRIVER",
    ],
  };

  return env;
}

/**
 * Derive doctor issues from environment report.
 * @returns {{ class: string, code: string, message: string, setup?: string, fallback?: string }[]}
 */
export function doctorIssues(env) {
  const issues = [];

  if (!env.os.supportedForL2) {
    issues.push({
      class: DoctorClass.UNSUPPORTED_PLATFORM,
      code: "platform",
      message: `OS platform ${env.os.platform} is not supported for Tauri desktop E2E`,
      setup: "Use Windows, Linux, or macOS host with GUI session",
      fallback: "Run L0 invoke policy + L1 desktop boundary on any platform",
    });
  }

  if (!env.rust.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: "rust",
      message: "rustc/cargo not available on PATH",
      setup: "Install Rust toolchain: https://rustup.rs",
      fallback: "L0 frontend policy only (vitest)",
    });
  }

  if (!env.node.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: "node",
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
        code: "node_version",
        message: `Node ${env.node.version} < 20`,
        setup: "Upgrade to Node.js >= 20",
        fallback: "May still run with older node; unsupported",
      });
    }
  }

  if (!env.pnpm.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: "pnpm",
      message: "pnpm not available on PATH",
      setup: "corepack enable && corepack prepare pnpm@9.15.0 --activate",
      fallback: "node tools/tauri-e2e/*.mjs still works for doctor/L2 if cargo+node present",
    });
  }

  if (!env.paths.fakeAcpPresent) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: "fake_acp",
      message: `fake ACP runtime missing: ${env.paths.fakeAcpJs}`,
      setup: "Ensure tools/fake-acp-runtime is present in worktree",
      fallback: "L2 process smoke without ACP still possible",
    });
  }

  // WebView
  if (env.os.platform === "win32" && !env.webview.available) {
    issues.push({
      class: DoctorClass.WEBVIEW_UNAVAILABLE,
      code: "webview2",
      message: "WebView2 Runtime not detected in registry",
      setup:
        "Install Evergreen WebView2 Runtime: https://developer.microsoft.com/microsoft-edge/webview2/",
      fallback: "L0+L1 only; L2 launch will fail or crash",
    });
  }

  // Build artifacts
  if (!env.build.binaryFound) {
    issues.push({
      class: DoctorClass.BUILD_REQUIRED,
      code: "app_binary",
      message: "tracer-desktop binary not found under target/{debug,release}",
      setup:
        "From repo root: pnpm --filter @tracer/desktop build ; cargo build -p tracer-desktop",
      fallback: "L0+L1 do not require packaged binary",
    });
  }

  if (!env.build.frontendDistPresent) {
    issues.push({
      class: DoctorClass.BUILD_REQUIRED,
      code: "frontend_dist",
      message: "apps/desktop/dist/index.html missing (required for real app frontend)",
      setup: "pnpm --filter @tracer/desktop build",
      fallback: "L1 cargo tests can use dist stub; L2 real smoke needs Vite build",
    });
  }

  // Driver stack for L3-I
  const driverReady =
    env.drivers.tauriDriver.available &&
    (env.os.platform === "win32"
      ? env.drivers.nativeDriver.msedgedriver.available
      : env.os.platform === "linux"
        ? env.drivers.nativeDriver.webkitWebDriver.available
        : false);

  if (!env.os.supportedForL3I_externalDriver) {
    issues.push({
      class: DoctorClass.UNSUPPORTED_PLATFORM,
      code: "l3i_platform",
      message: `External tauri-driver path unsupported on ${env.os.platform}`,
      setup: "Use Windows/Linux for L3-I external driver, or future embedded WDIO path on macOS",
      fallback: "L0+L1+L2 process smoke only",
    });
  } else if (!env.drivers.tauriDriver.available) {
    issues.push({
      class: DoctorClass.DRIVER_UNAVAILABLE,
      code: "tauri_driver",
      message: "tauri-driver not on PATH",
      setup: "cargo install tauri-driver --locked",
      fallback: "L0+L1+L2 without WebDriver interaction",
    });
  } else if (env.os.platform === "win32" && !env.drivers.nativeDriver.msedgedriver.available) {
    issues.push({
      class: DoctorClass.DRIVER_UNAVAILABLE,
      code: "msedgedriver",
      message: "msedgedriver not on PATH (required by tauri-driver on Windows)",
      setup:
        "Download Edge WebDriver matching Edge version; place msedgedriver.exe on PATH, or set TRACER_NATIVE_DRIVER. Or: cargo install --git https://github.com/chippers/msedgedriver-tool && msedgedriver-tool",
      fallback: "L0+L1+L2 process smoke only",
    });
  } else if (
    env.os.platform === "linux" &&
    !env.drivers.nativeDriver.webkitWebDriver.available
  ) {
    issues.push({
      class: DoctorClass.DRIVER_UNAVAILABLE,
      code: "webkit_webdriver",
      message: "WebKitWebDriver not on PATH",
      setup: "Install webkit2gtk-driver (Debian) or distribution equivalent",
      fallback: "L0+L1+L2 process smoke only",
    });
  }

  // Tauri CLI is helpful but not strictly required for cargo build -p
  if (!env.tauriCli.available) {
    issues.push({
      class: DoctorClass.MISSING_TOOL,
      code: "tauri_cli",
      message: "@tauri-apps/cli / cargo-tauri not detected (optional for cargo-only builds)",
      setup: "pnpm --filter @tracer/desktop add -D @tauri-apps/cli  OR  cargo install tauri-cli --locked",
      fallback: "cargo build -p tracer-desktop still works for L2 binary",
    });
  }

  // If only driver-related / optional issues remain and binary exists, READY for L2
  void driverReady;
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
  const l3i =
    l2 &&
    env.os.supportedForL3I_externalDriver &&
    env.drivers.tauriDriver.available &&
    (env.os.platform === "win32"
      ? env.drivers.nativeDriver.msedgedriver.available
      : env.drivers.nativeDriver.webkitWebDriver.available);

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
        ? "attemptable (driver stack present)"
        : "blocked until tauri-driver + native WebDriver installed",
    },
    "L3-J": {
      attemptable: false,
      claim: "DEFERRED — owned by future W2.2-B product journey; not claimed by W2.2-A",
    },
    blockers: [...codes],
  };
}

/** Convenience one-shot for doctor. */
export function runDiscovery() {
  const env = discoverEnvironment();
  const issues = doctorIssues(env);
  const capabilities = capabilityMatrix(env, issues);
  return { env, issues, capabilities };
}
