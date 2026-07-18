/**
 * Minimal WebDriver HTTP client for L3-I infrastructure smoke.
 * No Selenium/WDIO dependency — raw protocol for diagnosability.
 */

import http from "node:http";
import https from "node:https";
import { URL } from "node:url";

/**
 * @param {string} baseUrl e.g. http://127.0.0.1:4444
 * @param {string} method
 * @param {string} pathname
 * @param {object} [body]
 * @param {{ timeoutMs?: number }} [opts]
 */
export function webdriverRequest(baseUrl, method, pathname, body, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 30_000;
  const url = new URL(pathname, baseUrl.endsWith("/") ? baseUrl : baseUrl + "/");
  const payload = body === undefined ? null : JSON.stringify(body);
  const lib = url.protocol === "https:" ? https : http;

  return new Promise((resolve, reject) => {
    const req = lib.request(
      {
        protocol: url.protocol,
        hostname: url.hostname,
        port: url.port,
        path: url.pathname + url.search,
        method,
        headers: {
          "Content-Type": "application/json; charset=utf-8",
          Accept: "application/json",
          ...(payload
            ? { "Content-Length": Buffer.byteLength(payload) }
            : {}),
        },
        timeout: timeoutMs,
      },
      (res) => {
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => {
          const text = Buffer.concat(chunks).toString("utf8");
          let json = null;
          try {
            json = text ? JSON.parse(text) : null;
          } catch {
            json = { raw: text };
          }
          resolve({
            statusCode: res.statusCode || 0,
            headers: res.headers,
            body: json,
            raw: text,
          });
        });
      },
    );
    req.on("error", reject);
    req.on("timeout", () => {
      req.destroy(new Error(`WebDriver request timeout ${timeoutMs}ms ${method} ${pathname}`));
    });
    if (payload) req.write(payload);
    req.end();
  });
}

export class WebDriverClient {
  /**
   * @param {string} baseUrl
   */
  constructor(baseUrl) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.sessionId = null;
  }

  status() {
    return webdriverRequest(this.baseUrl, "GET", "/status");
  }

  /**
   * Create session for Tauri app via tauri-driver.
   * Capability shape follows tauri-driver / WebDriver BiDi-ish alwaysMatch.
   * @param {{ application: string, args?: string[], env?: Record<string,string> }} app
   */
  async newSession(app, opts = {}) {
    // Support both legacy desiredCapabilities and W3C alwaysMatch.
    const tauriOptions = {
      application: app.application,
      ...(app.args ? { args: app.args } : {}),
      ...(app.env ? { env: app.env } : {}),
    };
    const body = {
      capabilities: {
        alwaysMatch: {
          "tauri:options": tauriOptions,
          browserName: "wry",
        },
        firstMatch: [{}],
      },
      // Legacy fallback some driver builds accept
      desiredCapabilities: {
        "tauri:options": tauriOptions,
        browserName: "wry",
      },
    };
    const res = await webdriverRequest(
      this.baseUrl,
      "POST",
      "/session",
      body,
      { timeoutMs: opts.timeoutMs ?? 60_000 },
    );
    const sid =
      res.body?.value?.sessionId ||
      res.body?.sessionId ||
      res.body?.value?.capabilities?.sessionId;
    if (sid) this.sessionId = sid;
    return res;
  }

  async deleteSession(opts = {}) {
    if (!this.sessionId) return { statusCode: 0, body: { skipped: true } };
    const sid = this.sessionId;
    this.sessionId = null;
    return webdriverRequest(
      this.baseUrl,
      "DELETE",
      `/session/${sid}`,
      undefined,
      { timeoutMs: opts.timeoutMs ?? 30_000 },
    );
  }

  async getTitle() {
    this.#needSession();
    return webdriverRequest(
      this.baseUrl,
      "GET",
      `/session/${this.sessionId}/title`,
    );
  }

  async getPageSource() {
    this.#needSession();
    return webdriverRequest(
      this.baseUrl,
      "GET",
      `/session/${this.sessionId}/source`,
    );
  }

  /**
   * Execute sync script in the WebView (if supported by driver).
   * @param {string} script
   * @param {unknown[]} [args]
   */
  async execute(script, args = []) {
    this.#needSession();
    return webdriverRequest(
      this.baseUrl,
      "POST",
      `/session/${this.sessionId}/execute/sync`,
      { script, args },
      { timeoutMs: 30_000 },
    );
  }

  #needSession() {
    if (!this.sessionId) throw new Error("no WebDriver session");
  }
}

/**
 * Poll /status until ready or timeout.
 */
export async function waitDriverReady(baseUrl, opts = {}) {
  const timeoutMs = opts.timeoutMs ?? 20_000;
  const intervalMs = opts.intervalMs ?? 300;
  const start = Date.now();
  let lastErr = null;
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await webdriverRequest(baseUrl, "GET", "/status", undefined, {
        timeoutMs: 3_000,
      });
      if (res.statusCode >= 200 && res.statusCode < 500) {
        return res;
      }
      lastErr = new Error(`status ${res.statusCode}`);
    } catch (e) {
      lastErr = e;
    }
    await new Promise((r) => setTimeout(r, intervalMs));
  }
  throw new Error(
    `tauri-driver not ready at ${baseUrl}: ${lastErr instanceof Error ? lastErr.message : lastErr}`,
  );
}
