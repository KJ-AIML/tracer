import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));

/**
 * Resolve the tracer repository root (contains tests/specifications and docs/).
 * Walks upward from this package and optional startDir.
 */
export function findRepoRoot(startDir = process.cwd()) {
  const candidates = [startDir, path.resolve(HERE, "../../..")];
  for (const start of candidates) {
    let dir = path.resolve(start);
    for (let i = 0; i < 12; i++) {
      const specs = path.join(dir, "tests", "specifications", "scenarios", "catalog.yaml");
      if (fs.existsSync(specs)) return dir;
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
  }
  throw new Error(
    "Could not locate tracer repo root (missing tests/specifications/scenarios/catalog.yaml)",
  );
}

export function catalogPath(repoRoot = findRepoRoot()) {
  return path.join(repoRoot, "tests", "specifications", "scenarios", "catalog.yaml");
}

export function expectedEventsDir(repoRoot = findRepoRoot()) {
  return path.join(repoRoot, "tests", "specifications", "expected-events");
}

export function expectedEventsPath(scenarioId, repoRoot = findRepoRoot()) {
  return path.join(expectedEventsDir(repoRoot), `${scenarioId}.json`);
}

export function acpFixturesDir(repoRoot = findRepoRoot()) {
  return path.join(repoRoot, "tests", "fixtures", "acp");
}

export function fakeRuntimeBin(repoRoot = findRepoRoot()) {
  return path.join(repoRoot, "tools", "fake-acp-runtime", "bin", "fake-acp-runtime.js");
}
