#!/usr/bin/env node
/**
 * CLI: product identity consistency check for Windows packaging.
 * Exit 0 on pass, 1 on fail.
 */

import { checkIdentity } from "./lib/identity.mjs";

const report = checkIdentity();
const mode = process.argv.includes("--json") ? "json" : "text";

if (mode === "json") {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log("Tracer Windows identity check (W2.3-A)");
  console.log(`version: ${report.version}`);
  console.log(`product: ${report.identity.productName}`);
  console.log(`id:      ${report.identity.identifier}`);
  console.log(`exe:     ${report.identity.exeWindows}`);
  console.log(`appData: ${report.identity.appDataWindowsTemplate}`);
  console.log("");
  for (const c of report.checks) {
    const mark =
      c.status === "pass" ? "PASS" : c.status === "warn" ? "WARN" : "FAIL";
    const detail =
      c.actual !== undefined
        ? ` actual=${JSON.stringify(c.actual)} expected=${JSON.stringify(c.expected)}`
        : c.path
          ? ` path=${c.path}`
          : "";
    console.log(`  [${mark}] ${c.id}${detail}`);
    if (c.note) console.log(`         note: ${c.note}`);
  }
  console.log("");
  console.log(report.ok ? "RESULT: PASS" : "RESULT: FAIL");
}

process.exit(report.ok ? 0 : 1);
