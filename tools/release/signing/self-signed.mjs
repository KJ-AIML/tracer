/**
 * Isolated self-signed Authenticode mechanics proof (W2.4.2-A Part 9).
 *
 * - Certificate lives ONLY under OS temp (never in repo)
 * - Signs COPIES only
 * - Verifies signature + tamper rejection
 * - Cleans up cert + copies
 * - Never claims trusted / production signing
 */

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  statSync,
} from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { SIGNING_MODES, TEST_ONLY_SUBJECT } from "./modes.mjs";
import {
  createProtectedTempDir,
  secureRemove,
  secureRemoveDir,
} from "./secrets.mjs";
import { detectSigningTools } from "./detect-tools.mjs";
import { signWithPowerShellStore } from "./sign.mjs";
import {
  fileSha256,
  verifySignature,
  tamperFileInPlace,
  inspectAuthenticode,
} from "./verify.mjs";
import { discoverArtifacts } from "../lib/artifacts.mjs";
import { REPO_ROOT } from "../lib/paths.mjs";

function runPs(script, timeout = 120_000) {
  const r = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { encoding: "utf8", windowsHide: true, timeout },
  );
  return {
    ok: r.status === 0 && !r.error,
    status: r.status,
    stdout: (r.stdout || "").trim(),
    stderr: (r.stderr || "").trim(),
    error: r.error ? String(r.error.message || r.error) : null,
  };
}

/**
 * Create a short-lived code-signing cert in CurrentUser\My; return thumbprint.
 * Cert is test-only; also export PFX into protected temp for cleanup tracking.
 */
export function provisionTestCertificate(tempDir) {
  const subject = TEST_ONLY_SUBJECT;
  const pfxPath = path.join(tempDir, "tracer-self-signed-test.pfx");
  // Password is ephemeral random; never logged in results
  const password = `t${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;
  const passFile = path.join(tempDir, ".pfx-pass");
  writeFileSync(passFile, password, { mode: 0o600 });

  const ps = `
    $ErrorActionPreference = 'Stop';
    $subject = ${JSON.stringify(subject)};
    $cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject -CertStoreLocation 'Cert:\\CurrentUser\\My' -KeyExportPolicy Exportable -KeySpec Signature -HashAlgorithm SHA256 -NotAfter (Get-Date).AddHours(6);
    $pwd = ConvertTo-SecureString -String ${JSON.stringify(password)} -Force -AsPlainText;
    Export-PfxCertificate -Cert $cert -FilePath ${JSON.stringify(pfxPath)} -Password $pwd | Out-Null;
    Write-Output $cert.Thumbprint;
  `;
  const r = runPs(ps);
  if (!r.ok || !r.stdout) {
    return {
      ok: false,
      error: r.stderr || r.error || "New-SelfSignedCertificate failed",
    };
  }
  const thumbprint = r.stdout.split(/\r?\n/).map((l) => l.trim()).filter(Boolean).pop();
  return {
    ok: true,
    thumbprint,
    subject,
    pfxPath,
    passFile,
    store: "Cert:\\CurrentUser\\My",
  };
}

export function removeTestCertificate(thumbprint) {
  if (!thumbprint) return { ok: true, removed: false };
  const ps = `
    $ErrorActionPreference = 'SilentlyContinue';
    $c = Get-ChildItem Cert:\\CurrentUser\\My\\${thumbprint};
    if ($c) { Remove-Item -Path $c.PSPath -Force; Write-Output 'removed' } else { Write-Output 'missing' }
  `;
  const r = runPs(ps);
  return { ok: r.ok, detail: r.stdout || r.stderr };
}

/**
 * Ensure we have something PE-like to sign. Prefer real RC artifacts; else synthesize tiny PE copy from portable if any.
 * If no artifacts, create a minimal valid-enough PE stub for mechanics (MZ + padding) — Set-AuthenticodeSignature can sign non-PE in some cases but we prefer PE.
 */
export function materializeTestPayloads(tempDir) {
  const found = discoverArtifacts();
  const originals = [];
  const copies = [];

  const consider = (src, label) => {
    if (!src || !existsSync(src)) return;
    const st = statSync(src);
    const sha = fileSha256(src);
    originals.push({
      label,
      path: src,
      relative: path.relative(REPO_ROOT, src).replace(/\\/g, "/"),
      sha256: sha,
      sizeBytes: st.size,
    });
    const dest = path.join(tempDir, `copy-${label}-${path.basename(src)}`);
    copyFileSync(src, dest);
    copies.push({
      label,
      path: dest,
      sourceRelative: path.relative(REPO_ROOT, src).replace(/\\/g, "/"),
      preSignSha256: sha,
    });
  };

  consider(found.portable, "portable");
  for (let i = 0; i < found.nsis.length; i++) {
    consider(found.nsis[i], `nsis${i || ""}`);
  }

  if (copies.length === 0) {
    // Compile a fresh UNSIGNED .NET PE in temp (OS binaries retain catalog signatures).
    const stub = path.join(tempDir, "tracer-signing-unsigned.exe");
    const psCompile =
      "$ErrorActionPreference = 'Stop';\n" +
      "Add-Type -OutputAssembly " +
      JSON.stringify(stub) +
      " -TypeDefinition @\"\n" +
      "public class TracerSignProbe {\n" +
      "  public static void Main() { System.Console.WriteLine(\"tracer-sign-probe\"); }\n" +
      "}\n" +
      "\"@\n";
    const compiled = spawnSync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", psCompile],
      { encoding: "utf8", windowsHide: true, timeout: 120_000 },
    );
    if ((compiled.status !== 0 && compiled.status != null) || compiled.error || !existsSync(stub)) {
      throw new Error(
        "failed to compile unsigned probe PE: " +
          ((compiled.stderr || "") + " " + (compiled.stdout || "") + " " + (compiled.error || "")),
      );
    }
    copies.push({
      label: "probe",
      path: stub,
      sourceRelative: null,
      preSignSha256: fileSha256(stub),
      synthetic: true,
    });
  }

  return { originals, copies, discovered: found };
}

/**
 * Full Part 9 self-signed mechanics proof.
 */
export function runSelfSignedMechanicsProof(opts = {}) {
  const tools = detectSigningTools();
  const report = {
    kind: "tracer-self-signed-mechanics",
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    mode: SIGNING_MODES.SELF_SIGNED_TEST,
    pipelineMechanics: "FAIL",
    selfSignedTestValidation: "FAIL",
    trustedDistributionSigning: "BLOCKED_NO_CERTIFICATE",
    tamperDetection: "FAIL",
    secretCleanup: "FAIL",
    originalsUnchanged: false,
    steps: [],
  };

  if (!tools.windows) {
    report.selfSignedTestValidation = "NOT_RUN";
    report.error = "UNSUPPORTED_PLATFORM";
    return report;
  }
  if (!tools.tools.powershellAuthenticode.available) {
    report.selfSignedTestValidation = "NOT_RUN";
    report.error = "BLOCKED_NO_SIGNING_TOOL";
    report.classification = "BLOCKED_NO_SIGNING_TOOL";
    return report;
  }

  const tempDir = createProtectedTempDir("tracer-sign-test-");
  report.tempDirPresentDuringRun = true;
  let thumbprint = null;

  try {
    const cert = provisionTestCertificate(tempDir);
    report.steps.push({
      step: "provision_test_cert",
      ok: cert.ok,
      subject: cert.ok ? cert.subject : null,
      thumbprintSuffix: cert.ok ? String(cert.thumbprint).slice(-8) : null,
    });
    if (!cert.ok) {
      report.error = cert.error;
      return report;
    }
    thumbprint = cert.thumbprint;

    const payloads = materializeTestPayloads(tempDir);
    report.steps.push({
      step: "materialize_copies",
      ok: payloads.copies.length > 0,
      copyCount: payloads.copies.length,
      originalCount: payloads.originals.length,
      usedSyntheticStub: payloads.copies.some((c) => c.synthetic),
    });

    // Record original hashes for later comparison
    const originalHashes = payloads.originals.map((o) => ({
      relative: o.relative,
      sha256: o.sha256,
    }));

    const signed = [];
    for (const c of payloads.copies) {
      const r = signWithPowerShellStore(c.path, thumbprint, {
        timestampUrl: opts.timestampUrl || null,
      });
      const post = existsSync(c.path) ? fileSha256(c.path) : null;
      const ver = verifySignature(c.path, {
        expectedSubject: TEST_ONLY_SUBJECT,
      });
      signed.push({
        label: c.label,
        signOk: r.ok,
        preSignSha256: c.preSignSha256,
        postSignSha256: post,
        verify: ver,
        hashChanged: c.preSignSha256 !== post,
      });
      report.steps.push({
        step: `sign_verify_${c.label}`,
        ok: r.ok && ver.signaturePresent && (ver.valid || ver.classification === 'PRESENT_SELF_SIGNED_UNTRUSTED_ROOT' || ver.classification === 'PRESENT_NOT_TRUSTED' || ver.classification === 'VALID_SELF_SIGNED_OR_LOCAL'),
        signatureStatus: ver.signatureStatus,
        classification: ver.classification,
        subject: ver.certificateSubject,
      });
    }

    const anySigned = signed.some(
      (s) => s.signOk && s.verify.signaturePresent && (s.verify.valid || String(s.verify.classification || '').includes('SELF_SIGNED') || s.verify.classification === 'PRESENT_NOT_TRUSTED'),
    );
    if (!anySigned) {
      report.pipelineMechanics = "FAIL";
      report.selfSignedTestValidation = "FAIL";
      report.error = "no copy successfully signed/verified";
      report.signed = signed;
      return report;
    }
    report.pipelineMechanics = "PASS";

    // Tamper first successfully signed copy
    const victim = signed.find(
      (s) => s.signOk && s.verify.signaturePresent,
    );
    const tamper = tamperFileInPlace(
      payloads.copies.find((c) => c.label === victim.label).path,
    );
    const afterTamper = verifySignature(
      payloads.copies.find((c) => c.label === victim.label).path,
    );
    const tamperRejected =
      afterTamper.rejected ||
      afterTamper.signatureStatus === "HashMismatch" ||
      afterTamper.classification === "TAMPERED_OR_HASH_MISMATCH" ||
      afterTamper.signatureStatus !== victim.verify.signatureStatus;

    report.steps.push({
      step: "tamper_detection",
      ok: tamperRejected,
      beforeStatus: victim.verify.signatureStatus,
      afterStatus: afterTamper.signatureStatus,
      afterClassification: afterTamper.classification,
      tamperedAtOffset: tamper.tamperedAtOffset,
    });
    report.tamperDetection = tamperRejected ? "PASS" : "FAIL";

    // Originals unchanged
    let originalsOk = true;
    for (const o of originalHashes) {
      const abs = path.join(REPO_ROOT, o.relative);
      if (!existsSync(abs)) continue;
      const now = fileSha256(abs);
      if (now !== o.sha256) originalsOk = false;
    }
    // If only synthetic stub, originals vacuously unchanged
    if (originalHashes.length === 0) originalsOk = true;
    report.originalsUnchanged = originalsOk;
    report.steps.push({ step: "originals_unchanged", ok: originalsOk });

    report.selfSignedTestValidation =
      anySigned && tamperRejected && originalsOk ? "PASS" : "FAIL";
    report.signed = signed.map((s) => ({
      label: s.label,
      preSignSha256: s.preSignSha256,
      postSignSha256: s.postSignSha256,
      signatureStatus: s.verify.signatureStatus,
      classification: s.verify.classification,
      subject: s.verify.certificateSubject,
    }));
    report.signingTool = {
      kind: "powershell",
      path: tools.tools.powershellAuthenticode.path,
      version: tools.tools.powershellAuthenticode.version,
    };
    report.note =
      "SELF_SIGNED_TEST proves pipeline mechanics only — not trusted Authenticode or SmartScreen readiness";
  } finally {
    // Cleanup cert store + temp secrets
    const rmCert = removeTestCertificate(thumbprint);
    const rmDir = secureRemoveDir(tempDir);
    report.secretCleanup =
      rmCert.ok && rmDir.removed ? "PASS" : "FAIL";
    report.steps.push({
      step: "cleanup",
      certRemoved: rmCert,
      tempRemoved: rmDir,
      tempStillExists: existsSync(tempDir),
    });
    report.tempDirPresentDuringRun = false;
    report.tempDirRemains = existsSync(tempDir);
  }

  return report;
}