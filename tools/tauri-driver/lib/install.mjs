/**
 * Provision tauri-driver via cargo install (user cargo bin).
 * Opt-in only — never auto-install without --apply / TRACER_TAURI_E2E_SETUP.
 */

import { spawnSync } from "node:child_process";
import { existsSync, copyFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import os from "node:os";
import {
  BIN_DIR,
  CACHE_DIR,
  projectTauriDriverCandidates,
  redactPath,
} from "./paths.mjs";
import { recordVersions } from "./edge.mjs";

function tryCmd(cmd, args, opts = {}) {
  try {
    let r;
    if (process.platform === "win32" && opts.shell !== false) {
      const line = [cmd, ...args]
        .map((a) => {
          const s = String(a);
          return /[\s"]/u.test(s) ? `"${s.replace(/"/g, '\\"')}"` : s;
        })
        .join(" ");
      r = spawnSync(process.env.ComSpec || "cmd.exe", ["/d", "/s", "/c", line], {
        encoding: "utf8",
        windowsHide: true,
        timeout: opts.timeout ?? 600_000,
        env: process.env,
      });
    } else {
      r = spawnSync(cmd, args, {
        encoding: "utf8",
        windowsHide: true,
        timeout: opts.timeout ?? 600_000,
        env: process.env,
      });
    }
    if (r.error) {
      return { ok: false, error: r.error.message, stdout: "", stderr: "", status: null };
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
      status: null,
    };
  }
}

function which(bin) {
  if (process.platform === "win32") {
    for (const name of [`${bin}.exe`, `${bin}.cmd`, bin]) {
      const r = spawnSync("where.exe", [name], {
        encoding: "utf8",
        windowsHide: true,
        timeout: 8_000,
      });
      if (r.status === 0 && r.stdout) {
        const first = r.stdout
          .split(/\r?\n/)
          .map((s) => s.trim())
          .find(Boolean);
        if (first && existsSync(first)) return first;
      }
    }
    return null;
  }
  const r = spawnSync("which", [bin], {
    encoding: "utf8",
    timeout: 8_000,
  });
  if (r.status !== 0 || !r.stdout) return null;
  return r.stdout.split(/\r?\n/).map((s) => s.trim()).find(Boolean) || null;
}

export function cargoBinDir() {
  if (process.env.CARGO_HOME) {
    return path.join(process.env.CARGO_HOME, "bin");
  }
  return path.join(os.homedir(), ".cargo", "bin");
}

export function resolveTauriDriverPath() {
  for (const c of projectTauriDriverCandidates()) {
    if (c && existsSync(c)) return c;
  }
  const onPath = which("tauri-driver");
  if (onPath) return onPath;
  // cargo bin even if not on PATH
  const cargoName =
    process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver";
  const cargoPath = path.join(cargoBinDir(), cargoName);
  if (existsSync(cargoPath)) return cargoPath;
  return null;
}

export function readTauriDriverVersion(driverPath) {
  if (!driverPath || !existsSync(driverPath)) {
    return { available: false, path: null, version: null, raw: null };
  }
  // tauri-driver may not support --version; try a few forms
  for (const args of [["--version"], ["-V"], ["version"]]) {
    const r = tryCmd(driverPath, args, { timeout: 8_000, shell: false });
    const raw = [r.stdout, r.stderr].filter(Boolean).join("\n");
    if (r.ok || raw) {
      const m = raw.match(/v?(\d+\.\d+\.\d+(?:[-+][\w.]+)?)/);
      return {
        available: true,
        path: driverPath,
        version: m ? m[1] : "present",
        raw: raw || "present",
      };
    }
  }
  return {
    available: true,
    path: driverPath,
    version: "present",
    raw: null,
  };
}

/**
 * cargo install tauri-driver --locked into user cargo bin.
 * Optionally copy into project bin for PATH-independent discovery.
 */
export function installTauriDriver({ copyToProjectBin = true } = {}) {
  const cargo = which("cargo");
  if (!cargo) {
    return {
      ok: false,
      code: "CARGO_NOT_FOUND",
      message: "cargo not on PATH — install Rust toolchain first",
    };
  }

  console.log("[setup] cargo install tauri-driver --locked ...");
  const r = tryCmd("cargo", ["install", "tauri-driver", "--locked"], {
    timeout: 900_000,
  });
  if (!r.ok) {
    return {
      ok: false,
      code: "TAURI_DRIVER_INSTALL_FAILED",
      message: `cargo install tauri-driver failed (exit ${r.status}): ${(r.stderr || r.stdout || r.error || "").slice(0, 1200)}`,
      status: r.status,
    };
  }

  const installed = resolveTauriDriverPath();
  if (!installed) {
    return {
      ok: false,
      code: "TAURI_DRIVER_NOT_FOUND",
      message:
        "cargo install reported success but tauri-driver not found in cargo bin or PATH",
      cargoBin: redactPath(cargoBinDir()),
    };
  }

  let projectCopy = null;
  if (copyToProjectBin) {
    mkdirSync(BIN_DIR, { recursive: true });
    const dest = path.join(
      BIN_DIR,
      process.platform === "win32" ? "tauri-driver.exe" : "tauri-driver",
    );
    try {
      copyFileSync(installed, dest);
      projectCopy = dest;
    } catch (e) {
      // non-fatal — cargo bin is enough if on PATH
      projectCopy = null;
      console.warn(
        "[setup] could not copy tauri-driver to project bin:",
        e instanceof Error ? e.message : e,
      );
    }
  }

  const ver = readTauriDriverVersion(installed);
  recordVersions({
    tauriDriver: {
      version: ver.version,
      path: projectCopy
        ? "tools/tauri-driver/bin/tauri-driver.exe"
        : redactPath(installed),
      cargoBin: redactPath(cargoBinDir()),
      recordedAt: new Date().toISOString(),
    },
  });

  return {
    ok: true,
    code: "TAURI_DRIVER_INSTALLED",
    message: `tauri-driver installed (${ver.version})`,
    path: installed,
    projectCopy,
    version: ver.version,
    redactedPath: redactPath(installed),
    redactedProjectCopy: redactPath(projectCopy),
  };
}