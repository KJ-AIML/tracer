/**
 * Discover Windows release artifacts (NSIS, MSI, portable).
 */

import { existsSync, readdirSync, statSync, mkdirSync, copyFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  releaseBinaryCandidates,
  nsisBundleDirs,
  msiBundleDirs,
  releaseStageDir,
  REPO_ROOT,
} from "./paths.mjs";

function listFiles(dir, pred) {
  if (!existsSync(dir)) return [];
  try {
    return readdirSync(dir)
      .map((n) => path.join(dir, n))
      .filter((p) => {
        try {
          return statSync(p).isFile() && (!pred || pred(p));
        } catch {
          return false;
        }
      });
  } catch {
    return [];
  }
}

/**
 * @returns {{
 *   portable: string|null,
 *   nsis: string[],
 *   msi: string[],
 *   primary: string|null,
 *   all: string[]
 * }}
 */
export function discoverArtifacts() {
  let portable = null;
  for (const c of releaseBinaryCandidates()) {
    if (existsSync(c) && statSync(c).isFile()) {
      portable = c;
      break;
    }
  }

  const nsis = [];
  for (const d of nsisBundleDirs()) {
    for (const f of listFiles(d, (p) => /\.exe$/i.test(p))) {
      nsis.push(f);
    }
  }

  const msi = [];
  for (const d of msiBundleDirs()) {
    for (const f of listFiles(d, (p) => /\.msi$/i.test(p))) {
      msi.push(f);
    }
  }

  const primary = nsis[0] || portable || msi[0] || null;
  const all = [...new Set([...(portable ? [portable] : []), ...nsis, ...msi])];

  return { portable, nsis, msi, primary, all };
}

/**
 * Stage copies under target/release-rc/windows with a manifest.
 * Does not commit artifacts.
 */
export function stageArtifacts(extra = {}) {
  const found = discoverArtifacts();
  const stage = releaseStageDir();
  mkdirSync(stage, { recursive: true });

  const staged = [];
  const copyOne = (src, label) => {
    if (!src || !existsSync(src)) return null;
    const dest = path.join(stage, path.basename(src));
    copyFileSync(src, dest);
    const st = statSync(dest);
    const entry = {
      label,
      source: path.relative(REPO_ROOT, src).replace(/\\/g, "/"),
      staged: path.relative(REPO_ROOT, dest).replace(/\\/g, "/"),
      bytes: st.size,
      mtimeMs: st.mtimeMs,
    };
    staged.push(entry);
    return entry;
  };

  if (found.portable) copyOne(found.portable, "portable");
  for (const n of found.nsis) copyOne(n, "nsis");
  for (const m of found.msi) copyOne(m, "msi");

  const manifest = {
    schemaVersion: 1,
    kind: "windows-release-rc",
    createdAt: new Date().toISOString(),
    platform: process.platform,
    arch: process.arch,
    artifacts: staged,
    discovered: {
      portable: found.portable
        ? path.relative(REPO_ROOT, found.portable).replace(/\\/g, "/")
        : null,
      nsis: found.nsis.map((p) =>
        path.relative(REPO_ROOT, p).replace(/\\/g, "/"),
      ),
      msi: found.msi.map((p) =>
        path.relative(REPO_ROOT, p).replace(/\\/g, "/"),
      ),
    },
    ...extra,
  };

  const manifestPath = path.join(stage, "manifest.json");
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2), "utf8");
  return { stage, manifestPath, manifest, found };
}
