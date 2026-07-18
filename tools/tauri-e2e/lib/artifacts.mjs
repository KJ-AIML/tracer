/**
 * Failure artifacts + sanitization (W2.3-C).
 * Never write secrets, credentials, or raw user home paths into dumps.
 */

import {
  existsSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  readdirSync,
  statSync,
} from "node:fs";
import path from "node:path";

/**
 * Redact secrets / PII from artifact text before disk write.
 * @param {string|null|undefined} text
 * @returns {string}
 */
export function sanitizeArtifactText(text) {
  if (text == null) return "";
  let s = String(text);
  s = s.replace(
    /(Authorization:\s*)(Bearer\s+)?[^\s"'`]+/gi,
    "$1$2[REDACTED]",
  );
  s = s.replace(
    /(api[_-]?key|access[_-]?token|refresh[_-]?token|token|secret|password|credential|xai[_-]?api[_-]?key|openai[_-]?api[_-]?key|grok[_-]?api[_-]?key)\s*[:=]\s*[^\s"'`,;]+/gi,
    "$1=[REDACTED]",
  );
  s = s.replace(
    /"(api[_-]?key|access[_-]?token|refresh[_-]?token|token|secret|password|credential|authorization)"\s*:\s*"[^"]*"/gi,
    '"$1":"[REDACTED]"',
  );
  s = s.replace(/([A-Za-z]:\\Users\\)[^\\"'`\s]+/g, "$1[USER]");
  s = s.replace(/(\/Users\/|\/home\/)[^\/"'`\s]+/g, "$1[USER]");
  s = s.replace(
    /(TRACER_(?:API|AUTH|SECRET|TOKEN|CREDENTIAL)[A-Z0-9_]*)\s*=\s*[^\r\n]+/gi,
    "$1=[REDACTED]",
  );
  return s;
}

/**
 * @param {string} dir
 * @param {string} name
 * @param {string|object} content
 */
export function writeSanitized(dir, name, content) {
  mkdirSync(dir, { recursive: true });
  const body =
    typeof content === "string"
      ? sanitizeArtifactText(content)
      : sanitizeArtifactText(JSON.stringify(content, null, 2));
  const target = path.join(dir, name);
  writeFileSync(target, body, "utf8");
  return target;
}

/**
 * Capture WebDriver page/probe snapshot into artifactsDir/label.
 * @param {import('./webdriver.mjs').WebDriverClient} client
 * @param {string} artifactsDir
 * @param {string} label
 */
export async function captureFailureArtifacts(client, artifactsDir, label) {
  const dir = path.join(artifactsDir, label);
  mkdirSync(dir, { recursive: true });
  const meta = { label, capturedAt: new Date().toISOString(), files: [] };

  try {
    const src = await client.getPageSource();
    const body = sanitizeArtifactText(src.raw || JSON.stringify(src.body));
    const f = path.join(dir, "page.html");
    writeFileSync(f, body, "utf8");
    meta.files.push("page.html");
  } catch (e) {
    writeSanitized(
      dir,
      "page-error.txt",
      e instanceof Error ? e.message : String(e),
    );
    meta.files.push("page-error.txt");
  }

  try {
    const title = await client.getTitle();
    writeSanitized(dir, "title.json", title.body ?? title);
    meta.files.push("title.json");
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
    writeSanitized(dir, "probe.json", probe.body ?? probe);
    meta.files.push("probe.json");
  } catch {
    /* ignore */
  }

  writeSanitized(dir, "capture-meta.json", meta);
  return { dir, meta };
}

/**
 * Recursively verify no unsanitized secret-like patterns remain in text files.
 * @param {string} rootDir
 * @returns {{ ok: boolean, violations: { file: string, pattern: string }[] }}
 */
export function auditArtifactSanitization(rootDir) {
  const violations = [];
  if (!existsSync(rootDir)) return { ok: true, violations };

  /** @type {RegExp[]} */
  const bad = [
    /Authorization:\s*(Bearer\s+)?(?!\[REDACTED\])[A-Za-z0-9._\-]{8,}/i,
    /(?:api[_-]?key|password|secret)\s*[:=]\s*(?!\[REDACTED\])[^\s"']{6,}/i,
    /[A-Za-z]:\\Users\\(?!\[USER\])[^\\"'`\s]+/i,
  ];

  function walk(dir) {
    let entries = [];
    try {
      entries = readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of entries) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) {
        walk(p);
        continue;
      }
      if (!/\.(html|json|txt|log|md)$/i.test(e.name)) continue;
      let text = "";
      try {
        if (statSync(p).size > 2_000_000) continue;
        text = readFileSync(p, "utf8");
      } catch {
        continue;
      }
      for (const re of bad) {
        if (re.test(text)) {
          violations.push({ file: p, pattern: String(re) });
        }
      }
    }
  }

  walk(rootDir);
  return { ok: violations.length === 0, violations };
}
