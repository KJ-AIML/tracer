/**
 * Free TCP port allocation for tauri-driver (W2.3-C).
 * Avoids fixed-port collisions when multiple runners or leftovers exist.
 */

import net from "node:net";
import { FailureCode } from "./classify.mjs";

const DEFAULT_HOST = "127.0.0.1";
/** Prefer this base when free; scan upward on collision. */
export const DEFAULT_TAURI_DRIVER_PORT = 4444;
const MAX_SCAN = 64;

/**
 * Probe whether a TCP port is free for bind on host.
 * @param {number} port
 * @param {string} [host]
 * @returns {Promise<{ port: number, host: string, available: boolean, code: string|null, error: string|null }>}
 */
export function probePort(port, host = DEFAULT_HOST) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.unref();
    server.once("error", (err) => {
      resolve({
        port,
        host,
        available: false,
        code:
          err && err.code === "EADDRINUSE"
            ? FailureCode.PORT_IN_USE
            : FailureCode.PORT_CHECK_FAILED,
        error: err ? err.code || String(err) : "unknown",
      });
    });
    server.once("listening", () => {
      server.close(() => {
        resolve({
          port,
          host,
          available: true,
          code: null,
          error: null,
        });
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

/**
 * Bind ephemeral port (OS-assigned) then release — true free port.
 * @param {string} [host]
 * @returns {Promise<number>}
 */
export function allocateEphemeralPort(host = DEFAULT_HOST) {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.once("error", reject);
    server.listen(0, host, () => {
      const addr = server.address();
      const port = typeof addr === "object" && addr ? addr.port : 0;
      server.close((err) => {
        if (err) reject(err);
        else if (!port) reject(new Error("ephemeral port allocation failed"));
        else resolve(port);
      });
    });
  });
}

/**
 * Resolve a free tauri-driver port.
 *
 * Policy:
 * 1. If TRACER_TAURI_DRIVER_PORT is set and free → use it
 * 2. If fixed preference (default 4444) free → use it
 * 3. Scan [base..base+MAX_SCAN)
 * 4. Fall back to OS ephemeral
 *
 * @param {{ preferred?: number, host?: string, forceEphemeral?: boolean }} [opts]
 * @returns {Promise<{ port: number, host: string, strategy: string, probed: object[] }>}
 */
export async function allocateDriverPort(opts = {}) {
  const host = opts.host || process.env.TRACER_TAURI_DRIVER_HOST || DEFAULT_HOST;
  const preferred =
    opts.preferred != null
      ? Number(opts.preferred)
      : Number(process.env.TRACER_TAURI_DRIVER_PORT || DEFAULT_TAURI_DRIVER_PORT);
  const probed = [];

  if (opts.forceEphemeral) {
    const port = await allocateEphemeralPort(host);
    return { port, host, strategy: "ephemeral_forced", probed };
  }

  if (Number.isFinite(preferred) && preferred > 0) {
    const p = await probePort(preferred, host);
    probed.push(p);
    if (p.available) {
      return {
        port: preferred,
        host,
        strategy:
          process.env.TRACER_TAURI_DRIVER_PORT != null
            ? "env_preferred"
            : "default_preferred",
        probed,
      };
    }
  }

  const base =
    Number.isFinite(preferred) && preferred > 0
      ? preferred
      : DEFAULT_TAURI_DRIVER_PORT;
  for (let i = 1; i < MAX_SCAN; i++) {
    const candidate = base + i;
    const p = await probePort(candidate, host);
    probed.push(p);
    if (p.available) {
      return { port: candidate, host, strategy: "scan_up", probed };
    }
  }

  const port = await allocateEphemeralPort(host);
  return { port, host, strategy: "ephemeral", probed };
}

/**
 * Assert port free or throw with PORT_IN_USE (no silent reuse).
 * @param {number} port
 * @param {string} [host]
 */
export async function assertPortFree(port, host = DEFAULT_HOST) {
  const p = await probePort(port, host);
  if (!p.available) {
    const err = new Error(
      `port ${host}:${port} not free (${p.error || p.code})`,
    );
    err.code = p.code || FailureCode.PORT_IN_USE;
    throw err;
  }
  return p;
}
