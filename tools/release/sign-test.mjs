#!/usr/bin/env node
/**
 * pnpm release:sign:test — isolated self-signed mechanics proof (copies only).
 */
import { writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { runSelfSignedMechanicsProof } from "./signing/self-signed.mjs";
import { releaseStageDir, REPO_ROOT } from "./lib/paths.mjs";

const json = process.argv.includes("--json");
const report = runSelfSignedMechanicsProof();
const outDir = releaseStageDir();
mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, "signing-self-signed-test.json");
writeFileSync(outPath, JSON.stringify(report, null, 2) + "\n", "utf8");

if (json) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log("Tracer self-signed signing test (W2.4.2-A)");
  console.log(`pipelineMechanics: ${report.pipelineMechanics}`);
  console.log(`selfSignedTestValidation: ${report.selfSignedTestValidation}`);
  console.log(`tamperDetection: ${report.tamperDetection}`);
  console.log(`secretCleanup: ${report.secretCleanup}`);
  console.log(`trustedDistributionSigning: ${report.trustedDistributionSigning}`);
  console.log(`originalsUnchanged: ${report.originalsUnchanged}`);
  if (report.error) console.log(`error: ${report.error}`);
  console.log(`wrote ${path.relative(REPO_ROOT, outPath).replace(/\\/g, "/")}`);
}

const ok =
  report.pipelineMechanics === "PASS" &&
  report.selfSignedTestValidation === "PASS" &&
  report.tamperDetection === "PASS" &&
  report.secretCleanup === "PASS";
process.exit(ok ? 0 : 1);