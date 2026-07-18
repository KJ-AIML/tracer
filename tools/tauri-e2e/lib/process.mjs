/**
 * Process ownership, capture, timeouts, tree kill, orphan detection.
 * Never leave the Tracer app or drivers running after a harness exit.
 */

import { spawn, spawnSync } from "node:child_process";
import { createWriteStream, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

/**
 * @typedef {object} OwnedProcess
 * @property {import('node:child_process').ChildProcess} child
 * @property {number|null} pid
 * @property {string} label
 * @property {string[]} logPaths
 * @property {() => Promise<void>} stop
 * @property {boolean} stopped
 */

const owned = new Set();

function isWin() {
  return process.platform === "win32";
}

/**
 * Kill a process tree. Windows uses taskkill /T; Unix uses process group.
 * @param {number} pid
 * @param {{ force?: boolean }} [opts]
 */
export function killProcessTree(pid, opts = {}) {
  if (!pid || pid <= 0) return { ok: false, error: "invalid pid" };
  const force = opts.force !== false;
  try {
    if (isWin()) {
      const r = spawnSync(
        "taskkill",
        ["/PID", String(pid), "/T", force ? "/F" : ""].filter(Boolean),
        { encoding: "utf8", windowsHide: true, timeout: 15_000 },
      );
      // 128 / 255 often means already gone
      return { ok: r.status === 0 || r.status === 128, status: r.status, stdout: r.stdout, stderr: r.stderr };
    }
    try {
      process.kill(-pid, force ? "SIGKILL" : "SIGTERM");
    } catch {
      try {
        process.kill(pid, force ? "SIGKILL" : "SIGTERM");
      } catch (e) {
        return { ok: false, error: e instanceof Error ? e.message : String(e) };
      }
    }
    return { ok: true };
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/**
 * List PIDs whose image name matches (Windows) or process name contains (Unix).
 * @param {string} nameFragment e.g. "tracer-desktop"
 */
export function listPidsByName(nameFragment) {
  const pids = [];
  if (isWin()) {
    const r = spawnSync(
      "tasklist",
      ["/FI", `IMAGENAME eq ${nameFragment}.exe`, "/FO", "CSV", "/NH"],
      { encoding: "utf8", windowsHide: true, timeout: 15_000 },
    );
    const out = r.stdout || "";
    for (const line of out.split(/\r?\n/)) {
      // "tracer-desktop.exe","1234","Session Name","0","12,345 K"
      const m = line.match(/"[^"]+","(\d+)"/);
      if (m) pids.push(Number(m[1]));
    }
    // Also try without .exe filter via findstr fallback for partial names
    if (pids.length === 0 && !nameFragment.endsWith(".exe")) {
      const r2 = spawnSync("tasklist", ["/FO", "CSV", "/NH"], {
        encoding: "utf8",
        windowsHide: true,
        timeout: 20_000,
      });
      for (const line of (r2.stdout || "").split(/\r?\n/)) {
        if (line.toLowerCase().includes(nameFragment.toLowerCase())) {
          const m = line.match(/"[^"]+","(\d+)"/);
          if (m) pids.push(Number(m[1]));
        }
      }
    }
  } else {
    const r = spawnSync("pgrep", ["-f", nameFragment], {
      encoding: "utf8",
      timeout: 10_000,
    });
    for (const line of (r.stdout || "").split(/\n/)) {
      const n = Number(line.trim());
      if (n) pids.push(n);
    }
  }
  return [...new Set(pids)];
}

/**
 * Spawn a managed child with stdout/stderr capture.
 * @param {string} command
 * @param {string[]} args
 * @param {object} opts
 * @returns {OwnedProcess}
 */
export function spawnOwned(command, args, opts = {}) {
  const label = opts.label || command;
  const logDir = opts.logDir || null;
  let stdoutPath = null;
  let stderrPath = null;
  let stdoutStream = null;
  let stderrStream = null;

  if (logDir) {
    mkdirSync(logDir, { recursive: true });
    const stamp = Date.now();
    stdoutPath = path.join(logDir, `${label.replace(/\W+/g, "_")}-${stamp}.out.log`);
    stderrPath = path.join(logDir, `${label.replace(/\W+/g, "_")}-${stamp}.err.log`);
    stdoutStream = createWriteStream(stdoutPath, { flags: "a" });
    stderrStream = createWriteStream(stderrPath, { flags: "a" });
  }

  const child = spawn(command, args, {
    cwd: opts.cwd,
    env: { ...process.env, ...(opts.env || {}) },
    stdio: opts.stdio || ["ignore", "pipe", "pipe"],
    shell: opts.shell ?? false,
    windowsHide: opts.windowsHide ?? true,
    detached: opts.detached ?? false,
  });

  if (child.stdout) {
    child.stdout.on("data", (buf) => {
      if (stdoutStream) stdoutStream.write(buf);
      if (opts.onStdout) opts.onStdout(buf);
    });
  }
  if (child.stderr) {
    child.stderr.on("data", (buf) => {
      if (stderrStream) stderrStream.write(buf);
      if (opts.onStderr) opts.onStderr(buf);
    });
  }

  /** @type {OwnedProcess} */
  const handle = {
    child,
    pid: child.pid ?? null,
    label,
    logPaths: [stdoutPath, stderrPath].filter(Boolean),
    stopped: false,
    async stop() {
      if (handle.stopped) return;
      handle.stopped = true;
      const pid = handle.pid || child.pid;
      if (pid) {
        // Graceful then force
        killProcessTree(pid, { force: false });
        await delay(opts.graceMs ?? 800);
        killProcessTree(pid, { force: true });
      }
      try {
        stdoutStream?.end();
        stderrStream?.end();
      } catch {
        /* ignore */
      }
      owned.delete(handle);
    },
  };

  child.on("exit", () => {
    handle.stopped = true;
    try {
      stdoutStream?.end();
      stderrStream?.end();
    } catch {
      /* ignore */
    }
    owned.delete(handle);
  });

  owned.add(handle);
  return handle;
}

/**
 * Wait until predicate true or timeout.
 * @param {() => boolean | Promise<boolean>} pred
 * @param {{ timeoutMs?: number, intervalMs?: number, label?: string }} opts
 */
export async function waitFor(pred, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 30_000;
  const intervalMs = opts.intervalMs ?? 250;
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (await pred()) return true;
    await delay(intervalMs);
  }
  throw new Error(`timeout waiting for ${opts.label || "condition"} after ${timeoutMs}ms`);
}

/**
 * Wait for process exit with timeout; force-kill on timeout.
 * @param {OwnedProcess} handle
 * @param {{ timeoutMs?: number }} opts
 */
export async function waitExitOrKill(handle, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 10_000;
  const child = handle.child;
  if (child.exitCode !== null || child.killed) return child.exitCode;
  return new Promise((resolve) => {
    let done = false;
    const finish = (code) => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      resolve(code);
    };
    const timer = setTimeout(async () => {
      await handle.stop();
      finish(child.exitCode);
    }, timeoutMs);
    child.once("exit", (code) => finish(code));
  });
}

/**
 * Detect orphans matching known harness images, excluding allowlisted PIDs.
 * @param {string[]} names
 * @param {number[]} [allowPids]
 */
export function findOrphans(names, allowPids = []) {
  const allow = new Set(allowPids);
  const orphans = [];
  for (const name of names) {
    for (const pid of listPidsByName(name)) {
      if (!allow.has(pid) && pid !== process.pid) {
        orphans.push({ name, pid });
      }
    }
  }
  return orphans;
}

/**
 * Kill any orphans matching names (post-run safety).
 */
export function reapOrphans(names) {
  const found = findOrphans(names);
  const results = [];
  for (const o of found) {
    results.push({ ...o, kill: killProcessTree(o.pid, { force: true }) });
  }
  return results;
}

/**
 * Stop all owned processes (best-effort). Call from finally / signal handlers.
 */
export async function stopAllOwned() {
  const list = [...owned];
  await Promise.all(list.map((h) => h.stop()));
}

/** Unique temp directory under os.tmpdir. */
export function uniqueTempDir(prefix = "tracer-tauri-e2e") {
  const base = path.join(
    process.env.TEMP || process.env.TMPDIR || process.cwd(),
    `${prefix}-${process.pid}-${Date.now()}`,
  );
  mkdirSync(base, { recursive: true });
  return base;
}

/**
 * Install process signal hooks to never leave children running.
 */
export function installExitHooks() {
  const cleanup = async () => {
    await stopAllOwned();
  };
  for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
    try {
      process.on(sig, () => {
        void cleanup().finally(() => process.exit(130));
      });
    } catch {
      /* signal may not exist on Windows */
    }
  }
  process.on("exit", () => {
    // sync best-effort kill
    for (const h of [...owned]) {
      if (h.pid) killProcessTree(h.pid, { force: true });
    }
  });
}

export function processAlive(pid) {
  if (!pid) return false;
  try {
    if (isWin()) {
      const r = spawnSync(
        "tasklist",
        ["/FI", `PID eq ${pid}`, "/FO", "CSV", "/NH"],
        { encoding: "utf8", windowsHide: true, timeout: 8_000 },
      );
      return (r.stdout || "").includes(String(pid));
    }
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/**
 * On Windows, check if a process has a main window (GUI started).
 * Best-effort via PowerShell; returns null if indeterminate.
 */
export function windowsProcessHasMainWindow(pid) {
  if (!isWin() || !pid) return null;
  const script = `$p = Get-Process -Id ${pid} -ErrorAction SilentlyContinue; if ($null -eq $p) { 'missing' } elseif ($p.MainWindowHandle -ne 0) { 'yes' } else { 'no' }`;
  const r = spawnSync(
    "powershell",
    ["-NoProfile", "-Command", script],
    { encoding: "utf8", windowsHide: true, timeout: 10_000 },
  );
  const t = (r.stdout || "").trim().toLowerCase();
  if (t === "yes") return true;
  if (t === "no") return false;
  if (t === "missing") return false;
  return null;
}

export function assertPathExists(p, label) {
  if (!existsSync(p)) throw new Error(`${label || "path"} missing: ${p}`);
}
