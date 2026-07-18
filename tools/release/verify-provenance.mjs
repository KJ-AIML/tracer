#!/usr/bin/env node
/**
 * Verify a release provenance manifest against on-disk artifacts.
 *
 *   node tools/release/verify-provenance.mjs
 *   node tools/release/verify-provenance.mjs --manifest target/release-rc/windows/provenance.json
 */

import path from "node:path";
import { verifyProvenance } from "./lib/provenance.mjs";
import { releaseStageDir, REPO_ROOT } from "./lib/paths.mjs";

const args = process.argv.slice(2);
const json = args.includes("--json");
const mIdx = args.indexOf("--manifest");
const manifestPath =
  mIdx >= 0
    ? path.resolve(args[mIdx + 1])
    : path.join(releaseStageDir(), "provenance.json");

const result = verifyProvenance(manifestPath);

if (json) {
  console.log(
    JSON.stringify(
      {
        ok: result.ok,
        manifest: path.relative(REPO_ROOT, manifestPath).replace(/\\/g, "/"),
        errors: result.errors,
        checks: result.checks,
      },
      null,
      2,
    ),
  );
} else {
  console.log("=== Release provenance verify ===");
  console.log(
    `manifest: ${path.relative(REPO_ROOT, manifestPath).replace(/\\/g, "/")}`,
  );
  for (const c of result.checks || []) {
    console.log(`  [${c.status}] ${c.filename}`);
  }
  if (result.errors?.length) {
    console.log("errors:");
    for (const e of result.errors) console.log(`  - ${e}`);
  }
  console.log(result.ok ? "RESULT: PASS" : "RESULT: FAIL");
}

process.exit(result.ok ? 0 : 1);
