#!/usr/bin/env node
/**
 * CLI: classify Windows RC signing class.
 * Usage: node classify-signing.mjs [artifact paths...]
 */

import { classifyReleaseSigning } from "./lib/signing.mjs";
import { discoverArtifacts } from "./lib/artifacts.mjs";

const args = process.argv.slice(2).filter((a) => a !== "--json");
const json = process.argv.includes("--json");

const discovered = discoverArtifacts();
const paths = args.length > 0 ? args : discovered.all;
const report = classifyReleaseSigning(paths);

if (json) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log("Windows RC signing classification (W2.3-A)");
  console.log(`class: ${report.class}`);
  if (report.note) console.log(`note:  ${report.note}`);
  console.log(`env.hasCertEnv: ${report.env.hasCertEnv}`);
  for (const a of report.artifacts || []) {
    console.log(`  - ${a.path}`);
    console.log(`    class=${a.class} status=${a.status}`);
    if (a.error) console.log(`    error=${a.error}`);
  }
  if ((report.artifacts || []).length === 0) {
    console.log("  (no artifacts inspected)");
  }
}

// Exit 0 for SIGNED or UNSIGNED_DEVELOPMENT_RC; 2 for BLOCKED
if (report.class === "BLOCKED") process.exit(2);
process.exit(0);
