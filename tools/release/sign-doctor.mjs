#!/usr/bin/env node
/**
 * pnpm release:sign:doctor — non-destructive Authenticode readiness report.
 */
import { writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { runSigningDoctor } from "./signing/doctor.mjs";
import { releaseStageDir, REPO_ROOT } from "./lib/paths.mjs";

const json = process.argv.includes("--json");
const report = runSigningDoctor();
const outDir = releaseStageDir();
mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, "signing-doctor.json");
writeFileSync(outPath, JSON.stringify(report, null, 2) + "\n", "utf8");

if (json) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log("Tracer signing doctor (W2.4.2-A)");
  console.log(`classification: ${report.classification}`);
  console.log(`trustedReadiness: ${report.trustedReadiness}`);
  console.log(`platform: ${report.platform}`);
  console.log(
    `tool: ${report.signingTool ? `${report.signingTool.kind} ${report.signingTool.version || ""}`.trim() : "(none)"}`,
  );
  console.log(`certificate: ${report.certificate.category}`);
  console.log(`publisher: ${report.publisher.status}`);
  console.log(`timestamp: ${report.timestamp.status}`);
  console.log(`artifacts: ${report.artifacts.length}`);
  for (const a of report.artifacts) console.log(`  - ${a.type} ${a.relative}`);
  console.log("remediation:");
  for (const r of report.remediation) console.log(`  - ${r}`);
  console.log(`wrote ${path.relative(REPO_ROOT, outPath).replace(/\\/g, "/")}`);
}

// Exit 0 for READY_* ; 2 for BLOCKED_/UNSUPPORTED/FAIL
if (String(report.classification).startsWith("READY_")) process.exit(0);
process.exit(2);