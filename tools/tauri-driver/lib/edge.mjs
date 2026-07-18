/**
 * Edge browser + msedgedriver detection, version compatibility, opt-in download.
 *
 * Compatibility rule (explicit):
 *   major(msedgedriver) MUST equal major(installed Microsoft Edge).
 *   Prefer exact full-version match when available.
 *   Major mismatch => INCOMPATIBLE_VERSION / EDGE_DRIVER_VERSION_MISMATCH.
 *   File existence alone is NOT sufficient — version must be verified.
 */

import { spawnSync } from "node:child_process";
import {
  createWriteStream,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
  chmodSync,
  copyFileSync,
} from "node:fs";
import { pipeline } from "node:stream/promises";
import { Readable } from "node:stream";
import path from "node:path";
import https from "node:https";
import http from "node:http";
import {
  CACHE_DIR,
  MSEDGEDRIVER_CACHE_DIR,
  VERSIONS_FILE,
  redactPath,
  projectDriverCandidates,
} from "./paths.mjs";

function tryCmd(cmd, args, opts = {}) {
  try {
    const r = spawnSync(cmd, args, {
      encoding: "utf8",
      windowsHide: true,
      timeout: opts.timeout ?? 15_000,
      env: process.env,
      shell: opts.shell ?? false,
    });
    if (r.error) {
      return { ok: false, error: r.error.message, stdout: "", stderr: "" };
    }
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

/**
 * Parse dotted version; return { raw, major, minor, patch, build, full }.
 * @param {string | null | undefined} text
 */
export function parseSemverish(text) {
  if (!text) return null;
  const m = String(text).match(/(\d+)\.(\d+)\.(\d+)(?:\.(\d+))?/);
  if (!m) return null;
  return {
    raw: m[0],
    major: Number(m[1]),
    minor: Number(m[2]),
    patch: Number(m[3]),
    build: m[4] != null ? Number(m[4]) : null,
    full: m[0],
  };
}

/**
 * Locate msedge.exe on Windows without launching the browser UI.
 */
export function findEdgeBinaryWindows() {
  if (process.platform !== "win32") return null;
  const candidates = [
    process.env.TRACER_EDGE_BINARY,
    path.join(
      process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)",
      "Microsoft",
      "Edge",
      "Application",
      "msedge.exe",
    ),
    path.join(
      process.env.ProgramFiles || "C:\\Program Files",
      "Microsoft",
      "Edge",
      "Application",
      "msedge.exe",
    ),
  ].filter(Boolean);

  for (const c of candidates) {
    if (existsSync(c)) return c;
  }

  const r = tryCmd(
    "reg",
    [
      "query",
      "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\msedge.exe",
      "/ve",
    ],
    { timeout: 8_000 },
  );
  if (r.ok) {
    const m = r.stdout.match(/REG_SZ\s+(.+\.exe)/i);
    if (m && existsSync(m[1].trim())) return m[1].trim();
  }
  return null;
}

/**
 * Read Edge version from Application\<ver>\ folder or FileVersionInfo.
 * Avoids launching msedge.exe (which may open a browser window).
 */
export function detectEdgeVersionWindows() {
  const binary = findEdgeBinaryWindows();
  if (!binary) {
    return {
      available: false,
      path: null,
      version: null,
      major: null,
      method: null,
    };
  }
  const appDir = path.dirname(binary);
  try {
    const entries = readdirSync(appDir, { withFileTypes: true });
    const vers = [];
    for (const e of entries) {
      if (!e.isDirectory()) continue;
      const pv = parseSemverish(e.name);
      if (pv) vers.push({ ...pv, dir: path.join(appDir, e.name) });
    }
    vers.sort((a, b) => {
      if (a.major !== b.major) return b.major - a.major;
      if (a.minor !== b.minor) return b.minor - a.minor;
      if (a.patch !== b.patch) return b.patch - a.patch;
      return (b.build ?? 0) - (a.build ?? 0);
    });
    if (vers[0]) {
      return {
        available: true,
        path: binary,
        version: vers[0].full,
        major: vers[0].major,
        method: "application_version_dir",
      };
    }
  } catch {
    /* fall through */
  }

  const ps = tryCmd(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      `(Get-Item -LiteralPath '${binary.replace(/'/g, "''")}').VersionInfo.ProductVersion`,
    ],
    { timeout: 10_000 },
  );
  if (ps.ok && ps.stdout) {
    const pv = parseSemverish(ps.stdout);
    if (pv) {
      return {
        available: true,
        path: binary,
        version: pv.full,
        major: pv.major,
        method: "file_version_info",
      };
    }
  }

  return {
    available: true,
    path: binary,
    version: null,
    major: null,
    method: "binary_only",
  };
}

/**
 * Run msedgedriver --version and parse.
 * @param {string} driverPath
 */
export function readMsEdgeDriverVersion(driverPath) {
  if (!driverPath || !existsSync(driverPath)) {
    return { available: false, path: null, version: null, major: null, raw: null };
  }
  const r = tryCmd(driverPath, ["--version"], { timeout: 10_000 });
  const raw = [r.stdout, r.stderr].filter(Boolean).join("\n");
  const pv = parseSemverish(raw);
  return {
    available: true,
    path: driverPath,
    version: pv?.full ?? null,
    major: pv?.major ?? null,
    raw: raw || null,
    versionOk: r.ok || Boolean(pv),
  };
}

/**
 * Resolve first existing candidate path for msedgedriver.
 */
export function resolveLocalMsEdgeDriver() {
  for (const c of projectDriverCandidates("win32")) {
    if (c && existsSync(c)) return c;
  }
  return null;
}

/**
 * Explicit compatibility evaluation.
 * @param {{ major: number|null, version?: string|null }} edge
 * @param {{ major: number|null, version?: string|null, available?: boolean }} driver
 */
export function evaluateEdgeDriverCompatibility(edge, driver) {
  if (!driver?.available && !driver?.path) {
    return {
      compatible: false,
      code: "EDGE_DRIVER_NOT_FOUND",
      rule: "major(msedgedriver) == major(Edge)",
      message: "msedgedriver not found (PATH, TRACER_NATIVE_DRIVER, or project cache)",
      edgeMajor: edge?.major ?? null,
      driverMajor: null,
    };
  }
  if (edge?.major == null) {
    return {
      compatible: false,
      code: "EDGE_BROWSER_VERSION_UNKNOWN",
      rule: "major(msedgedriver) == major(Edge)",
      message: "Edge browser major version could not be determined; cannot verify driver compatibility",
      edgeMajor: null,
      driverMajor: driver?.major ?? null,
    };
  }
  if (driver?.major == null) {
    return {
      compatible: false,
      code: "EDGE_DRIVER_VERSION_UNVERIFIED",
      rule: "major(msedgedriver) == major(Edge)",
      message:
        "msedgedriver file present but --version did not yield a parseable version (existence alone is insufficient)",
      edgeMajor: edge.major,
      driverMajor: null,
    };
  }
  if (driver.major !== edge.major) {
    return {
      compatible: false,
      code: "EDGE_DRIVER_VERSION_MISMATCH",
      rule: "major(msedgedriver) == major(Edge)",
      message: `msedgedriver major ${driver.major} != Edge major ${edge.major} (driver=${driver.version}, edge=${edge.version})`,
      edgeMajor: edge.major,
      driverMajor: driver.major,
    };
  }
  return {
    compatible: true,
    code: "EDGE_DRIVER_COMPATIBLE",
    rule: "major(msedgedriver) == major(Edge)",
    message:
      driver.version && edge.version && driver.version === edge.version
        ? `exact match ${driver.version}`
        : `major match ${driver.major} (driver=${driver.version}, edge=${edge.version})`,
    edgeMajor: edge.major,
    driverMajor: driver.major,
    exact: Boolean(driver.version && edge.version && driver.version === edge.version),
  };
}

function httpGetBuffer(url, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > 8) {
      reject(new Error(`too many redirects: ${url}`));
      return;
    }
    const lib = url.startsWith("https") ? https : http;
    const req = lib.get(
      url,
      {
        headers: { "User-Agent": "tracer-tauri-e2e-setup/1.0" },
        timeout: 120_000,
      },
      (res) => {
        if (
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location
        ) {
          const next = new URL(res.headers.location, url).toString();
          res.resume();
          resolve(httpGetBuffer(next, redirects + 1));
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`HTTP ${res.statusCode} for ${url}`));
          return;
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      },
    );
    req.on("error", reject);
    req.on("timeout", () => {
      req.destroy(new Error(`timeout fetching ${url}`));
    });
  });
}

function httpGetText(url) {
  return httpGetBuffer(url).then((b) => b.toString("utf8").trim());
}

/**
 * Resolve a downloadable msedgedriver version string matching Edge major.
 * Tries exact Edge full version first, then latest for major.
 * @param {{ version: string|null, major: number|null }} edge
 */
export async function resolveDownloadableDriverVersion(edge) {
  const tried = [];
  if (edge.version) {
    tried.push(edge.version);
  }
  if (edge.major != null) {
    // Microsoft endpoint for latest release of a major on Windows
    try {
      const latest = await httpGetText(
        `https://msedgedriver.microsoft.com/LATEST_RELEASE_${edge.major}_WINDOWS`,
      );
      const pv = parseSemverish(latest);
      if (pv) tried.push(pv.full);
    } catch {
      /* ignore */
    }
    try {
      const latest2 = await httpGetText(
        `https://msedgedriver.azureedge.net/LATEST_RELEASE_${edge.major}_WINDOWS`,
      );
      const pv = parseSemverish(latest2);
      if (pv) tried.push(pv.full);
    } catch {
      /* ignore */
    }
  }
  // unique preserve order
  return [...new Set(tried)];
}

/**
 * Download and extract msedgedriver into project cache (gitignored).
 * Opt-in only — caller must gate on --apply / TRACER_TAURI_E2E_SETUP.
 * @param {{ version: string|null, major: number|null }} edge
 */
export async function downloadMsEdgeDriver(edge) {
  if (process.platform !== "win32") {
    return {
      ok: false,
      code: "UNSUPPORTED_PLATFORM",
      message: "msedgedriver auto-download is Windows-only",
    };
  }
  if (edge.major == null) {
    return {
      ok: false,
      code: "EDGE_BROWSER_VERSION_UNKNOWN",
      message: "cannot download driver without Edge major version",
    };
  }

  mkdirSync(MSEDGEDRIVER_CACHE_DIR, { recursive: true });
  mkdirSync(CACHE_DIR, { recursive: true });

  const candidates = await resolveDownloadableDriverVersion(edge);
  if (!candidates.length) {
    return {
      ok: false,
      code: "EDGE_DRIVER_DOWNLOAD_VERSION_UNKNOWN",
      message: "could not resolve a downloadable msedgedriver version",
    };
  }

  const bases = [
    "https://msedgedriver.microsoft.com",
    "https://msedgedriver.azureedge.net",
  ];
  const zipName = "edgedriver_win64.zip";
  let lastErr = null;
  let usedVersion = null;
  let zipBuf = null;

  outer: for (const ver of candidates) {
    for (const base of bases) {
      const url = `${base}/${ver}/${zipName}`;
      try {
        zipBuf = await httpGetBuffer(url);
        usedVersion = ver;
        break outer;
      } catch (e) {
        lastErr = e instanceof Error ? e.message : String(e);
      }
    }
  }

  if (!zipBuf || !usedVersion) {
    return {
      ok: false,
      code: "EDGE_DRIVER_DOWNLOAD_FAILED",
      message: `failed to download msedgedriver for candidates ${candidates.join(", ")}: ${lastErr}`,
      candidates,
    };
  }

  const zipPath = path.join(MSEDGEDRIVER_CACHE_DIR, `edgedriver_${usedVersion}.zip`);
  writeFileSync(zipPath, zipBuf);

  const extractDir = path.join(MSEDGEDRIVER_CACHE_DIR, usedVersion);
  mkdirSync(extractDir, { recursive: true });

  // Expand-Archive via PowerShell (no extra deps)
  const expand = tryCmd(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      `Expand-Archive -LiteralPath '${zipPath.replace(/'/g, "''")}' -DestinationPath '${extractDir.replace(/'/g, "''")}' -Force`,
    ],
    { timeout: 120_000 },
  );
  if (!expand.ok) {
    return {
      ok: false,
      code: "EDGE_DRIVER_EXTRACT_FAILED",
      message: `Expand-Archive failed: ${expand.stderr || expand.stdout || expand.error}`,
      zipPath: redactPath(zipPath),
    };
  }

  const extracted = path.join(extractDir, "msedgedriver.exe");
  if (!existsSync(extracted)) {
    return {
      ok: false,
      code: "EDGE_DRIVER_EXTRACT_FAILED",
      message: `msedgedriver.exe missing after extract in ${redactPath(extractDir)}`,
    };
  }

  const currentDir = path.join(MSEDGEDRIVER_CACHE_DIR, "current");
  mkdirSync(currentDir, { recursive: true });
  const currentExe = path.join(currentDir, "msedgedriver.exe");
  copyFileSync(extracted, currentExe);

  // Also place a top-level convenience copy
  const topExe = path.join(MSEDGEDRIVER_CACHE_DIR, "msedgedriver.exe");
  copyFileSync(extracted, topExe);

  const verified = readMsEdgeDriverVersion(currentExe);
  const compat = evaluateEdgeDriverCompatibility(edge, verified);

  recordVersions({
    msedgedriver: {
      version: verified.version,
      major: verified.major,
      path: "tools/tauri-driver/.cache/msedgedriver/current/msedgedriver.exe",
      downloadedVersion: usedVersion,
      edgeVersion: edge.version,
      edgeMajor: edge.major,
      compatible: compat.compatible,
      recordedAt: new Date().toISOString(),
    },
  });

  return {
    ok: compat.compatible,
    code: compat.compatible ? "EDGE_DRIVER_INSTALLED" : compat.code,
    message: compat.compatible
      ? `msedgedriver ${verified.version} installed to project cache`
      : compat.message,
    path: currentExe,
    version: verified.version,
    major: verified.major,
    downloadedVersion: usedVersion,
    compatibility: compat,
    redactedPath: redactPath(currentExe),
  };
}

/**
 * Merge-record into versions.local.json (gitignored).
 * @param {object} partial
 */
export function recordVersions(partial) {
  mkdirSync(CACHE_DIR, { recursive: true });
  let prev = {};
  if (existsSync(VERSIONS_FILE)) {
    try {
      prev = JSON.parse(readFileSync(VERSIONS_FILE, "utf8"));
    } catch {
      prev = {};
    }
  }
  const next = {
    ...prev,
    ...partial,
    updatedAt: new Date().toISOString(),
  };
  writeFileSync(VERSIONS_FILE, JSON.stringify(next, null, 2), "utf8");
  return next;
}

export function readRecordedVersions() {
  if (!existsSync(VERSIONS_FILE)) return null;
  try {
    return JSON.parse(readFileSync(VERSIONS_FILE, "utf8"));
  } catch {
    return null;
  }
}