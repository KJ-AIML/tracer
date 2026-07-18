#!/usr/bin/env node
/**
 * W2.3-C reliability self-test (no GUI / no live provider).
 * Validates: port allocation, artifact sanitize, edge probe, inject parse, temp cleanup.
 *
 * Usage:
 *   node tools/tauri-e2e/reliability-selftest.mjs
 *   node tools/tauri-e2e/reliability-selftest.mjs --json
 */

import net from "node:net";
import {
  existsSync,
  mkdirSync,
  writeFileSync,
  rmSync,
  readFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { allocateDriverPort, probePort } from "./lib/ports.mjs";
import {
  sanitizeArtifactText,
  writeSanitized,
  auditArtifactSanitization,
} from "./lib/artifacts.mjs";
import {
  parseInjectMode,
  edgeUpdateResilienceProbe,
  cleanupTempDir,
  countProductAssertionFailures,
  injectClassification,
  WAIT_POLICY,
} from "./lib/reliability.mjs";
import { uniqueTempDir } from "./lib/process.mjs";
import { FailureCode } from "./lib/classify.mjs";

const jsonOut = process.argv.includes("--json");
const results = [];

function check(name, ok, detail = {}) {
  results.push({ name, ok: Boolean(ok), ...detail });
  if (!jsonOut) {
    console.log(`${ok ? "PASS" : "FAIL"}  ${name}${detail.message ? " — " + detail.message : ""}`);
  }
}

async function holdPort(port, host = "127.0.0.1") {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.unref();
    server.once("error", reject);
    server.listen(port, host, () => resolve(server));
  });
}

async function main() {
  // --- sanitize ---
  const dirty =
    'Authorization: Bearer sk-secret-abc123\n' +
    'api_key=supersecretvalue\n' +
    '"password":"hunter2"\n' +
    'C:\\Users\\alice\\AppData\\Local\\Tracer\n' +
    '/Users/bob/project\n' +
    'TRACER_API_KEY=should-hide\n';
  const clean = sanitizeArtifactText(dirty);
  check(
    "sanitize_redacts_bearer",
    /\[REDACTED\]/.test(clean) && !/sk-secret-abc123/.test(clean),
    { sample: clean.slice(0, 120) },
  );
  check(
    "sanitize_redacts_user_path",
    /Users\\\[USER\]/.test(clean) || /Users\/\[USER\]/.test(clean),
  );
  check("sanitize_redacts_password_json", !/hunter2/.test(clean));

  const artDir = uniqueTempDir("tracer-rel-art");
  writeSanitized(artDir, "leak.html", dirty);
  const audit = auditArtifactSanitization(artDir);
  check("artifact_audit_clean_after_sanitize", audit.ok, {
    violations: audit.violations,
  });

  // Write intentionally dirty and ensure audit catches it
  const dirtyDir = path.join(artDir, "dirty");
  mkdirSync(dirtyDir, { recursive: true });
  writeFileSync(
    path.join(dirtyDir, "raw.txt"),
    "Authorization: Bearer still-leaked-token-value\n",
    "utf8",
  );
  const auditBad = auditArtifactSanitization(dirtyDir);
  check("artifact_audit_detects_leak", !auditBad.ok, {
    violations: auditBad.violations.length,
  });

  // --- ports ---
  const alloc1 = await allocateDriverPort({ preferred: 4444 });
  check("port_allocate_primary", alloc1.port > 0, {
    port: alloc1.port,
    strategy: alloc1.strategy,
  });

  // Hold a preferred port and ensure scan/ephemeral avoids collision
  let holder = null;
  try {
    const prefer = 18765;
    holder = await holdPort(prefer);
    const p = await probePort(prefer);
    check("port_probe_detects_in_use", !p.available, { code: p.code });
    const alloc2 = await allocateDriverPort({ preferred: prefer });
    check(
      "port_allocate_avoids_collision",
      alloc2.port !== prefer && alloc2.port > 0,
      { port: alloc2.port, strategy: alloc2.strategy },
    );
  } finally {
    if (holder) await new Promise((r) => holder.close(() => r()));
  }

  // --- inject parse ---
  check("inject_none", parseInjectMode("none").mode === "none");
  check("inject_orphan_leak", parseInjectMode("orphan_leak").mode === "orphan_leak");
  check("inject_invalid_falls_safe", parseInjectMode("boom").mode === "none" && parseInjectMode("boom").invalid);
  check(
    "inject_app_launch_failure",
    parseInjectMode("app_launch_failure").mode === "app_launch_failure",
  );
  check(
    "inject_classification_app_launch",
    injectClassification("app_launch_failure").failureCode ===
      FailureCode.APP_LAUNCH_FAILED &&
      injectClassification("app_launch_failure").retries === 0,
  );
  check(
    "inject_classification_stale_edge",
    injectClassification("stale_edge_driver").failureCode ===
      FailureCode.EDGE_DRIVER_VERSION_MISMATCH,
  );
  check(
    "wait_policy_documented",
    WAIT_POLICY.appReady?.timeoutMs > 0 &&
      WAIT_POLICY.driverReady?.mechanism?.includes("wait"),
  );

  // --- edge probe ---
  const edge = edgeUpdateResilienceProbe();
  check("edge_probe_runs", edge && typeof edge.compatible === "boolean", {
    code: edge.code,
    message: edge.message,
    compatible: edge.compatible,
  });

  // --- temp cleanup ---
  const tmp = uniqueTempDir("tracer-rel-tmp");
  writeFileSync(path.join(tmp, "x.txt"), "x", "utf8");
  const cleaned = cleanupTempDir(tmp);
  check("temp_cleanup_removes_dir", cleaned.cleaned && !existsSync(tmp), cleaned);

  // --- product assert counter ---
  const n = countProductAssertionFailures([
    { result: "PASS" },
    { result: "FAIL" },
    { result: "BLOCKED_BY_TOOLING" },
    { result: "BLOCKED_BY_PRODUCT_GAP" },
  ]);
  check("count_product_assertion_failures", n === 2, { n });

  // cleanup art dir
  try {
    rmSync(artDir, { recursive: true, force: true });
  } catch {
    /* ignore */
  }

  const failed = results.filter((r) => !r.ok);
  const out = {
    schemaVersion: 1,
    module: "W2.3-C",
    suite: "reliability-selftest",
    result: failed.length === 0 ? "PASS" : "FAIL",
    passed: results.filter((r) => r.ok).length,
    failed: failed.length,
    total: results.length,
    checks: results,
    edge,
  };

  if (jsonOut) console.log(JSON.stringify(out, null, 2));
  else {
    console.log("");
    console.log(`reliability-selftest: ${out.result} (${out.passed}/${out.total})`);
  }
  process.exitCode = failed.length ? 1 : 0;
}

main().catch((e) => {
  console.error("[reliability-selftest] FAILED:", e);
  process.exitCode = 1;
});

