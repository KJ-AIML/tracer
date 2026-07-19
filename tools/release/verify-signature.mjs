#!/usr/bin/env node
/**
 * pnpm release:verify-signature — inspect Authenticode on RC artifacts.
 */
import { writeFileSync, mkdirSync, existsSync } from "node:fs";
import path from "node:path";
import { verifySignature, signingFieldsFromVerify, unsignedSigningFields } from "./signing/verify.mjs";
import { discoverArtifacts } from "./lib/artifacts.mjs";
import { releaseStageDir, REPO_ROOT } from "./lib/paths.mjs";

const args = process.argv.slice(2).filter((a) => a !== "--json");
const json = process.argv.includes("--json");
const found = discoverArtifacts();
const paths = args.length > 0 ? args : found.all;

const artifacts = paths.map((p) => {
  const abs = path.isAbsolute(p) ? p : path.join(REPO_ROOT, p);
  if (!existsSync(abs)) {
    return {
      path: p,
      ...unsignedSigningFields(),
      signatureStatus: "MISSING_FILE",
      rejected: true,
    };
  }
  const v = verifySignature(abs);
  return {
    relative: path.relative(REPO_ROOT, abs).replace(/\\/g, "/"),
    ...signingFieldsFromVerify(v, {
      artifactSha256: v.artifactSha256,
      preSignSha256: null,
      postSignSha256: v.signaturePresent ? v.artifactSha256 : null,
    }),
    rejected: v.rejected,
    classification: v.classification,
  };
});

const report = {
  kind: "tracer-signature-verify",
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  artifactCount: artifacts.length,
  artifacts,
};

const outDir = releaseStageDir();
mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, "signature-verify.json");
writeFileSync(outPath, JSON.stringify(report, null, 2) + "\n", "utf8");

if (json) console.log(JSON.stringify(report, null, 2));
else {
  console.log("Tracer signature verify (W2.4.2-A)");
  if (artifacts.length === 0) console.log("(no artifacts)");
  for (const a of artifacts) {
    console.log(`  - ${a.relative || a.path}`);
    console.log(`    signaturePresent=${a.signaturePresent} status=${a.signatureStatus} mode=${a.signingMode}`);
  }
  console.log(`wrote ${path.relative(REPO_ROOT, outPath).replace(/\\/g, "/")}`);
}

// Unsigned artifacts are OK (exit 0); missing tools/platform failures exit 2
const hardFail = artifacts.some((a) =>
  ["MISSING_FILE", "INSPECT_FAILED", "UNSUPPORTED_PLATFORM"].includes(a.signatureStatus),
);
process.exit(hardFail && artifacts.length > 0 ? 2 : 0);