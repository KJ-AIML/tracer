/**
 * Load tests/specifications/scenarios/catalog.yaml without external deps.
 * Indentation-based parser covering the W0-D catalog subset.
 */

import fs from "node:fs";
import { catalogPath, findRepoRoot } from "./paths.js";
import { isValidEvidence } from "./provenance.js";

const LIST_KEYS = new Set([
  "scenarios",
  "notes",
  "wireFixtures",
  "acceptanceIds",
  "failureIds",
  "requireFields",
  "forbiddenTypes",
  "forbiddenProductTypeAliases",
  "forbiddenFinalStatuses",
  "forbiddenBehaviors",
  "forbiddenTypesAsSuccess",
  "forbiddenTypesAfterExit",
  "errorClassAnyOf",
  "preconditionEvents",
]);

function stripComment(line) {
  let inSingle = false;
  let inDouble = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === "'" && !inDouble) inSingle = !inSingle;
    else if (c === '"' && !inSingle) inDouble = !inDouble;
    else if (c === "#" && !inSingle && !inDouble) return line.slice(0, i).trimEnd();
  }
  return line;
}

function parseScalar(raw) {
  const v = raw.trim();
  if (v === "" || v === "null" || v === "~") return null;
  if (v === "true") return true;
  if (v === "false") return false;
  if (/^-?\d+$/.test(v)) return Number(v);
  if (/^-?\d+\.\d+$/.test(v)) return Number(v);
  if (
    (v.startsWith('"') && v.endsWith('"')) ||
    (v.startsWith("'") && v.endsWith("'"))
  ) {
    return v.slice(1, -1);
  }
  if (v.startsWith("[") && v.endsWith("]")) {
    const inner = v.slice(1, -1).trim();
    if (!inner) return [];
    return inner.split(",").map((p) => parseScalar(p));
  }
  return v;
}

/**
 * @returns {{ schemaVersion: number, description?: string, defaults?: object, scenarios: object[] }}
 */
export function parseCatalogYaml(text) {
  const rawLines = text.split(/\r?\n/);
  const root = {};
  /** @type {{ indent: number, container: any, kind: 'map'|'list', multilineKey?: string }[]} */
  const stack = [{ indent: -1, container: root, kind: "map" }];

  const top = () => stack[stack.length - 1];

  for (const rawLine of rawLines) {
    // Preserve indent from original (before comment strip of content)
    if (/^\s*#/.test(rawLine) || rawLine.trim() === "") {
      // blank ends multiline folded loosely — keep accumulating only non-empty
      continue;
    }
    const noComment = stripComment(rawLine);
    if (!noComment.trim()) continue;
    const indent = noComment.match(/^\s*/)[0].length;
    const line = noComment.trim();

    // Multilinear scalar continuation (indent > parent key indent)
    if (top().multilineKey && indent > top().indent) {
      const key = top().multilineKey;
      const prev = top().container[key];
      const piece = line.trim();
      top().container[key] = prev ? `${prev} ${piece}` : piece;
      continue;
    }

    while (stack.length > 1 && indent <= top().indent) {
      stack.pop();
    }

    const ctx = top();

    if (line.startsWith("- ")) {
      const rest = line.slice(2).trim();
      if (!Array.isArray(ctx.container)) {
        throw new Error(`YAML list item outside list: ${line}`);
      }

      // `- id: value` or `- plain`
      const m = rest.match(/^([^:]+):\s*(.*)$/);
      if (m && !rest.startsWith("http") /* not url */) {
        const obj = {};
        ctx.container.push(obj);
        const key = m[1].trim();
        const val = m[2];
        if (val === "|" || val === ">") {
          obj[key] = "";
          stack.push({ indent, container: obj, kind: "map", multilineKey: key });
        } else if (val === "") {
          if (LIST_KEYS.has(key)) {
            obj[key] = [];
            stack.push({ indent, container: obj, kind: "map" });
            stack.push({ indent, container: obj[key], kind: "list" });
          } else {
            obj[key] = {};
            stack.push({ indent, container: obj, kind: "map" });
            stack.push({ indent, container: obj[key], kind: "map" });
          }
        } else {
          obj[key] = parseScalar(val);
          stack.push({ indent, container: obj, kind: "map" });
        }
      } else {
        ctx.container.push(parseScalar(rest));
      }
      continue;
    }

    const kv = line.match(/^([^:]+):\s*(.*)$/);
    if (!kv || Array.isArray(ctx.container)) {
      // unexpected plain line
      continue;
    }

    const key = kv[1].trim();
    const val = kv[2];

    // Clear multiline when a new key at this level appears
    if (ctx.multilineKey) delete ctx.multilineKey;

    if (val === "|" || val === ">") {
      ctx.container[key] = "";
      stack.push({ indent, container: ctx.container, kind: "map", multilineKey: key });
      continue;
    }

    if (val === "") {
      if (LIST_KEYS.has(key)) {
        ctx.container[key] = [];
        stack.push({ indent, container: ctx.container[key], kind: "list" });
      } else {
        ctx.container[key] = {};
        stack.push({ indent, container: ctx.container[key], kind: "map" });
      }
      continue;
    }

    ctx.container[key] = parseScalar(val);
  }

  if (!Array.isArray(root.scenarios)) root.scenarios = [];
  return root;
}

export function loadCatalog(repoRoot = findRepoRoot()) {
  const file = catalogPath(repoRoot);
  const text = fs.readFileSync(file, "utf8");
  const catalog = parseCatalogYaml(text);
  if (catalog.schemaVersion !== 1) {
    throw new Error(`Unsupported catalog schemaVersion: ${catalog.schemaVersion}`);
  }
  if (!Array.isArray(catalog.scenarios) || catalog.scenarios.length === 0) {
    throw new Error("catalog.yaml has no scenarios");
  }
  for (const s of catalog.scenarios) {
    if (!s.id) throw new Error("scenario missing id");
    if (!isValidEvidence(s.evidence)) {
      throw new Error(`scenario ${s.id} has invalid evidence: ${s.evidence}`);
    }
  }
  return catalog;
}

export function listStandardCiScenarioIds(catalog) {
  return catalog.scenarios.filter((s) => s.standardCi === true).map((s) => s.id);
}

export function getScenarioEntry(catalog, id) {
  return catalog.scenarios.find((s) => s.id === id) ?? null;
}
