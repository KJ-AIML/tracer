#!/usr/bin/env node
/**
 * pnpm release:sign — explicit signing entrypoint.
 * Default mode UNSIGNED (no-op). TRUSTED_AUTHENTICODE requires authorization + cert.
 * SELF_SIGNED_TEST should use pnpm release:sign:test instead.
 */
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { SIGNING_MODES } from "./signing/modes.mjs";
import { resolveSigningMode, signArtifact } from "./signing/sign.mjs";
import { discoverArtifacts } from "./lib/artifacts.mjs";
import { releaseStageDir, REPO_ROOT } from "./lib/paths.mjs";

const args = process.argv.slice(2);
const json = args.includes("--json");
const mode = resolveSigningMode({
  mode: args.includes("--trusted")
    ? SIGNING_MODES.TRUSTED_AUTHENTICODE
    : args.includes("--self-signed")
      ? SIGNING_MODES.SELF_SIGNED_TEST
      : undefined,
});

if (mode === SIGNING_MODES.SELF_SIGNED_TEST) {
  console.error("Use pnpm release:sign:test for SELF_SIGNED_TEST (isolated copies + cleanup)");
  process.exit(2);
}

const found = discoverArtifacts();
const results = [];

if (mode === SIGNING_MODES.UNSIGNED) {
  const summary = {
    ok: true,
    mode,
    note: "default UNSIGNED — no signatures applied; RC remains usable for internal testers with warnings",
    artifacts: found.all.map((p) => path.relative(REPO_ROOT, p).replace(/\\/g, "/")),
  };
  if (json) console.log(JSON.stringify(summary, null, 2));
  else {
    console.log(`mode: ${mode}`);
    console.log(summary.note);
    console.log(`artifacts considered: ${summary.artifacts.length}`);
  }
  process.exit(0);
}

// TRUSTED_AUTHENTICODE
const outDir = path.join(releaseStageDir(), "signed");
mkdirSync(outDir, { recursive: true });
const targets = [
  found.portable ? { src: found.portable, label: "portable" } : null,
  ...found.nsis.map((n, i) => ({ src: n, label: `nsis${i || ""}` })),
].filter(Boolean);

if (targets.length === 0) {
  console.error("BLOCKED_NO_CERTIFICATE/artifacts: no release artifacts to sign — run pnpm release:windows first");
  process.exit(2);
}

for (const t of targets) {
  const dest = path.join(outDir, path.basename(t.src));
  const r = signArtifact(t.src, dest, { mode });
  results.push({ label: t.label, ...r, dest: path.relative(REPO_ROOT, dest).replace(/\\/g, "/") });
}

const summary = { mode, results };
const outPath = path.join(releaseStageDir(), "signing-trusted-attempt.json");
writeFileSync(outPath, JSON.stringify(summary, null, 2) + "\n", "utf8");
if (json) console.log(JSON.stringify(summary, null, 2));
else {
  console.log(`mode: ${mode}`);
  for (const r of results) {
    console.log(`  - ${r.label}: ok=${r.ok} classification=${r.classification || "n/a"}`);
    if (r.error) console.log(`    error: ${r.error}`);
  }
}

const allOk = results.every((r) => r.ok);
process.exit(allOk ? 0 : 2);