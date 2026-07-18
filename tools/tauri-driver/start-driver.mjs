#!/usr/bin/env node
/**
 * Start tauri-driver with stdout/stderr capture and clean shutdown on signals.
 * Used by operators; L3-I runner also starts its own owned driver instance.
 */

import { spawn, spawnSync } from "node:child_process";
import { createWriteStream, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const port = process.env.TRACER_TAURI_DRIVER_PORT || "4444";
const native = process.env.TRACER_NATIVE_DRIVER;

function resolveDriver() {
  const cmd = process.platform === "win32" ? "where.exe" : "which";
  const r = spawnSync(cmd, ["tauri-driver"], {
    encoding: "utf8",
    shell: process.platform === "win32",
    windowsHide: true,
  });
  if (r.status !== 0) return null;
  return (r.stdout || "").split(/\r?\n/).map((s) => s.trim()).find(Boolean) || null;
}

const driverPath = resolveDriver();
if (!driverPath) {
  console.error("tauri-driver not on PATH. Run: cargo install tauri-driver --locked");
  console.error("See: node tools/tauri-driver/print-setup.mjs");
  process.exit(2);
}

const logDir = path.join(
  process.env.TEMP || process.env.TMPDIR || __dirname,
  `tauri-driver-${process.pid}`,
);
mkdirSync(logDir, { recursive: true });
const outLog = path.join(logDir, "stdout.log");
const errLog = path.join(logDir, "stderr.log");
const out = createWriteStream(outLog, { flags: "a" });
const err = createWriteStream(errLog, { flags: "a" });

const args = ["--port", String(port)];
if (native) args.push("--native-driver", native);

console.log(`starting ${driverPath} ${args.join(" ")}`);
console.log(`logs: ${logDir}`);

const child = spawn(driverPath, args, {
  stdio: ["ignore", "pipe", "pipe"],
  windowsHide: true,
});

child.stdout.pipe(out);
child.stderr.pipe(err);
child.stdout.pipe(process.stdout);
child.stderr.pipe(process.stderr);

function shutdown() {
  if (!child.pid) return;
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
      windowsHide: true,
    });
  } else {
    try {
      process.kill(child.pid, "SIGTERM");
    } catch {
      /* ignore */
    }
  }
}

for (const sig of ["SIGINT", "SIGTERM"]) {
  try {
    process.on(sig, () => {
      shutdown();
      process.exit(130);
    });
  } catch {
    /* signal may be unavailable */
  }
}

child.on("exit", (code) => {
  console.log(`tauri-driver exited ${code}`);
  process.exit(code ?? 0);
});
