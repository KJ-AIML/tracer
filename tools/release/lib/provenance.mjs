/**
 * Release provenance manifest helpers (W2.4.1-A).
 *
 * Distinguishes:
 * - provenance: who/what/when built the artifact (source SHA, version, toolchain)
 * - integrity: sha256 sizeBytes of the bytes on disk
 * - signing: Authenticode class (UNSIGNED_DEVELOPMENT_RC | SIGNED | BLOCKED)
 * - test evidence: separate RC / upgrade JSON results (not embedded here)
 */

import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { REPO_ROOT, releaseStageDir } from "./paths.mjs";
import { checkIdentity } from "./identity.mjs";
import { classifyReleaseSigning } from "./signing.mjs";
import { discoverArtifacts } from "./artifacts.mjs";

export const PROVENANCE_SCHEMA_VERSION = 1;
export const PROVENANCE_KIND = "tracer-release-provenance";

export function sha256File(filePath) {
  const hash = createHash("sha256");
  hash.update(readFileSync(filePath));
  return hash.digest("hex");
}

function git(args) {
  const r = spawnSync("git", args, {
    cwd: REPO_ROOT,
    encoding: "utf8",
    windowsHide: true,
  });
  if (r.status !== 0) return null;
  return (r.stdout || "").trim() || null;
}

function toolchain() {
  const rustc = spawnSync("rustc", ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  });
  const cargo = spawnSync("cargo", ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  });
  const node = spawnSync("node", ["--version"], {
    encoding: "utf8",
    windowsHide: true,
  });
  const pnpm = spawnSync(
    process.platform === "win32" ? "pnpm.cmd" : "pnpm",
    ["--version"],
    { encoding: "utf8", windowsHide: true, shell: process.platform === "win32" },
  );
  return {
    os: `${process.platform}/${process.arch}`,
    rustc: (rustc.stdout || "").trim() || null,
    cargo: (cargo.stdout || "").trim() || null,
    node: (node.stdout || "").trim() || null,
    pnpm: (pnpm.stdout || "").trim() || null,
    tauriCli: "@tauri-apps/cli@2",
  };
}

/**
 * Build a single artifact provenance entry (relative paths only).
 */
export function artifactEntry(absPath, artifactType) {
  if (!absPath || !existsSync(absPath)) {
    return null;
  }
  const st = statSync(absPath);
  const rel = path.relative(REPO_ROOT, absPath).replace(/\\/g, "/");
  // Refuse absolute-dev-path leakage in filename field.
  const filename = path.basename(absPath);
  return {
    artifactType,
    filename,
    relativePath: rel.startsWith("..") ? filename : rel,
    sizeBytes: st.size,
    sha256: sha256File(absPath),
  };
}

/**
 * Generate machine-readable provenance for portable + NSIS.
 * @param {{ signingClass?: string, extra?: object }} [opts]
 */
export function generateProvenance(opts = {}) {
  const identity = checkIdentity();
  const found = discoverArtifacts();
  const signing = classifyReleaseSigning(found.all);
  const sourceSha = git(["rev-parse", "HEAD"]);
  const tag = git(["describe", "--tags", "--exact-match", "HEAD"]) || null;
  const product = identity.identity?.productName || "Tracer";
  const version = identity.version || identity.identity?.resolved?.versions?.tauri;

  const artifacts = [];
  const portable = artifactEntry(found.portable, "portable");
  if (portable) artifacts.push(portable);
  for (const n of found.nsis) {
    const e = artifactEntry(n, "nsis");
    if (e) artifacts.push(e);
  }

  const manifest = {
    schemaVersion: PROVENANCE_SCHEMA_VERSION,
    kind: PROVENANCE_KIND,
    generatedAt: new Date().toISOString(),
    product,
    version,
    sourceSha,
    tag,
    platform: "windows-x64",
    identifier: identity.identity?.identifier || "dev.tracer.desktop",
    signing: {
      class: opts.signingClass || signing.class,
      note: "provenance records signing class; it does not prove Authenticode",
    },
    buildToolchain: toolchain(),
    artifacts,
    distinctions: {
      provenance: "build identity + source SHA + toolchain",
      integrity: "sizeBytes + sha256 per artifact",
      signing: "Authenticode classification only",
      testEvidence: "RC / upgrade result JSON files are separate",
    },
    ...(opts.extra || {}),
  };

  // Guard: no absolute Windows user paths in serialized JSON.
  const serialized = JSON.stringify(manifest);
  if (/[A-Za-z]:\\Users\\/i.test(serialized) || /\/Users\//.test(serialized)) {
    throw new Error(
      "provenance manifest must not embed absolute developer home paths",
    );
  }

  return { manifest, found, identity, signing };
}

export function writeProvenance(manifest, outPath) {
  const dest =
    outPath ||
    path.join(releaseStageDir(), "provenance.json");
  mkdirSync(path.dirname(dest), { recursive: true });
  writeFileSync(dest, JSON.stringify(manifest, null, 2) + "\n", "utf8");
  return dest;
}

/**
 * Verify a provenance manifest against files on disk.
 * @returns {{ ok: boolean, errors: string[], checks: object[] }}
 */
export function verifyProvenance(manifestPath) {
  const errors = [];
  const checks = [];
  if (!existsSync(manifestPath)) {
    return {
      ok: false,
      errors: [`manifest missing: ${manifestPath}`],
      checks,
    };
  }

  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (e) {
    return {
      ok: false,
      errors: [`invalid JSON: ${e instanceof Error ? e.message : String(e)}`],
      checks,
    };
  }

  if (manifest.kind !== PROVENANCE_KIND) {
    errors.push(`unexpected kind: ${manifest.kind}`);
  }
  if (manifest.schemaVersion !== PROVENANCE_SCHEMA_VERSION) {
    errors.push(`unexpected schemaVersion: ${manifest.schemaVersion}`);
  }
  if (!manifest.sourceSha || !/^[0-9a-f]{40}$/i.test(manifest.sourceSha)) {
    errors.push("sourceSha missing or not a 40-char hex SHA");
  }
  if (!manifest.version) errors.push("version missing");
  if (!manifest.product) errors.push("product missing");
  if (!Array.isArray(manifest.artifacts) || manifest.artifacts.length === 0) {
    errors.push("artifacts[] empty");
  }

  const serialized = JSON.stringify(manifest);
  if (/[A-Za-z]:\\Users\\/i.test(serialized) || /\/home\//.test(serialized)) {
    errors.push("manifest embeds absolute developer home path");
  }

  for (const art of manifest.artifacts || []) {
    const candidates = [];
    if (art.relativePath) {
      candidates.push(path.join(REPO_ROOT, art.relativePath));
    }
    // staged copy under release-rc
    candidates.push(path.join(releaseStageDir(), art.filename));
    candidates.push(
      path.join(REPO_ROOT, "target/release", art.filename),
    );
    candidates.push(
      path.join(REPO_ROOT, "target/release/bundle/nsis", art.filename),
    );

    let resolved = null;
    for (const c of candidates) {
      if (existsSync(c)) {
        resolved = c;
        break;
      }
    }

    if (!resolved) {
      errors.push(`artifact file not found for ${art.filename}`);
      checks.push({ filename: art.filename, status: "missing" });
      continue;
    }

    const st = statSync(resolved);
    const hash = sha256File(resolved);
    const sizeOk = st.size === art.sizeBytes;
    const hashOk = hash === art.sha256;
    if (!sizeOk) {
      errors.push(
        `${art.filename} size mismatch: disk=${st.size} manifest=${art.sizeBytes}`,
      );
    }
    if (!hashOk) {
      errors.push(`${art.filename} sha256 mismatch`);
    }
    checks.push({
      filename: art.filename,
      status: sizeOk && hashOk ? "pass" : "fail",
      sizeOk,
      hashOk,
      relativeResolved: path.relative(REPO_ROOT, resolved).replace(/\\/g, "/"),
    });
  }

  return { ok: errors.length === 0, errors, checks, manifest };
}
