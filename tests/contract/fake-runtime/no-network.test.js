import { describe, it } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { findRepoRoot } from "../../../packages/test-fixtures/src/index.js";

const FORBIDDEN_NET_PATTERNS = [
  /\bhttps?:\/\//i,
  /\bfetch\s*\(/,
  /\baxios\b/,
  /\bnode:http\b/,
  /\bnode:https\b/,
  /\bnet\.connect\b/,
  /\bdns\./,
  /\bXAI_API_KEY\b/,
  /\bOPENAI_API_KEY\b/,
  /\bapi\.x\.ai\b/i,
];

// Allowlisted documentation-only mentions inside README strings
const ALLOW_FILES = new Set(["README.md"]);

function walkJsFiles(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, ent.name);
    if (ent.isDirectory()) walkJsFiles(p, out);
    else if (ent.isFile() && ent.name.endsWith(".js")) out.push(p);
  }
  return out;
}

describe("no network / no live credentials in fake runtime", () => {
  const root = findRepoRoot();
  const roots = [
    path.join(root, "tools", "fake-acp-runtime"),
    path.join(root, "packages", "test-fixtures"),
    path.join(root, "tests", "contract", "fake-runtime"),
  ];

  it("source trees contain no network client usage", () => {
    /** @type {string[]} */
    const offenders = [];
    for (const base of roots) {
      for (const file of walkJsFiles(base)) {
        if (ALLOW_FILES.has(path.basename(file))) continue;
        const text = fs.readFileSync(file, "utf8");
        for (const re of FORBIDDEN_NET_PATTERNS) {
          // Allow clearing env vars in harness spawn
          if (re.source.includes("XAI_API_KEY") && text.includes('XAI_API_KEY: ""')) {
            continue;
          }
          if (re.test(text)) {
            // Extra allow: comments about live-only rejection
            if (
              re.source.includes("https") &&
              !text.match(/https?:\/\/(?![^\n]*(example|placeholder))/i)
            ) {
              // still flag real urls
            }
            const lines = text.split(/\n/);
            lines.forEach((line, i) => {
              if (re.test(line) && !line.trim().startsWith("//") && !line.includes("XAI_API_KEY: \"\"")) {
                // skip pure documentation strings about never claiming live
                if (/live-only|never|must not|no network/i.test(line) && !/https?:\/\//.test(line)) {
                  return;
                }
                if (/https?:\/\//.test(line) && !/example\.com|placeholder/.test(line)) {
                  offenders.push(`${file}:${i + 1}: ${line.trim()}`);
                } else if (!/https?:\/\//.test(re.source)) {
                  offenders.push(`${file}:${i + 1}: ${line.trim()}`);
                }
              }
            });
          }
        }
      }
    }
    assert.deepEqual(offenders, [], `network-like usage:\n${offenders.join("\n")}`);
  });

  it("standard tests must not enable TRACER_LIVE_SMOKE", () => {
    for (const base of roots) {
      for (const file of walkJsFiles(base)) {
        const text = fs.readFileSync(file, "utf8");
        assert.equal(
          /TRACER_LIVE_SMOKE\s*=\s*['"]1['"]/.test(text),
          false,
          `${file} enables live smoke`,
        );
      }
    }
  });
});
