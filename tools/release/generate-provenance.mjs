#!/usr/bin/env node
/**
 * Generate machine-readable release provenance for portable + NSIS.
 *
 *   node tools/release/generate-provenance.mjs
 *   node tools/release/generate-provenance.mjs --out target/release-rc/windows/provenance.json
 */

import { writeFileSync } from "node:fs";
import path from "node:path";
import {
  generateProvenance,
  writeProvenance,
} from "./lib/provenance.mjs";
import { releaseStageDir, REPO_ROOT } from "./lib/paths.mjs";

const args = process.argv.slice(2);
const json = args.includes("--json");
const outIdx = args.indexOf("--out");
const outPath =
  outIdx >= 0
    ? path.resolve(args[outIdx + 1])
    : path.join(releaseStageDir(), "provenance.json");

try {
  const { manifest, signing } = generateProvenance();
  const dest = writeProvenance(manifest, outPath);
  // Also write a relative-path copy note under fixtures metadata (no binaries).
  const metaDir = path.join(REPO_ROOT, "tests/fixtures/releases");
  try {
    writeFileSync(
      path.join(metaDir, "provenance.schema.json"),
      JSON.stringify(
        {
          schemaVersion: 1,
          kind: "tracer-release-provenance",
          required: [
            "product",
            "version",
            "schemaLogicalVersion",
            "sourceSha",
            "buildSourceSha",
            "gateTipSha",
            "platform",
            "identifier",
            "artifacts",
            "signing",
            "buildToolchain",
          ],
          artifactFields: [
            "artifactType",
            "filename",
            "relativePath",
            "sizeBytes",
            "sha256",
          ],
          note: "Binaries are never committed. sourceSha aliases buildSourceSha; gateTipSha may advance after report-only commits.",
        },
        null,
        2,
      ) + "\n",
      "utf8",
    );
  } catch {
    /* fixtures dir optional at generate time */
  }

  if (json) {
    console.log(JSON.stringify({ ok: true, path: path.relative(REPO_ROOT, dest).replace(/\\/g, "/"), signingClass: signing.class, manifest }, null, 2));
  } else {
    console.log(`wrote ${path.relative(REPO_ROOT, dest).replace(/\\/g, "/")}`);
    console.log(`signing: ${signing.class}`);
    console.log(`artifacts: ${manifest.artifacts.length}`);
    for (const a of manifest.artifacts) {
      console.log(`  - ${a.artifactType} ${a.filename} sha256=${a.sha256.slice(0, 12)}…`);
    }
  }
  process.exit(0);
} catch (e) {
  console.error(e instanceof Error ? e.message : e);
  process.exit(1);
}
