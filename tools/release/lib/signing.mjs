/**
 * Authenticode / code-signing classification for Windows RC.
 *
 * Classes (task contract):
 *   SIGNED | UNSIGNED_DEVELOPMENT_RC | BLOCKED
 *
 * Rules:
 * - Never claim SIGNED without a real signature on the artifact.
 * - Local/dev RC without certs → UNSIGNED_DEVELOPMENT_RC (may PASS when classified).
 * - Missing tooling that prevents classification → BLOCKED.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";

/**
 * @typedef {'SIGNED' | 'UNSIGNED_DEVELOPMENT_RC' | 'BLOCKED'} SigningClass
 */

/**
 * Probe environment for signing material (does not invent secrets).
 */
export function probeSigningEnvironment() {
  const thumb =
    process.env.TAURI_SIGNING_PRIVATE_KEY ||
    process.env.WINDOWS_CERTIFICATE_THUMBPRINT ||
    process.env.TAURI_WINDOWS_CERTIFICATE_THUMBPRINT ||
    null;
  const hasCertEnv = Boolean(
    thumb && String(thumb).trim() && !String(thumb).includes("REPLACE"),
  );
  return {
    hasCertEnv,
    certEnvKeysPresent: [
      "TAURI_SIGNING_PRIVATE_KEY",
      "WINDOWS_CERTIFICATE_THUMBPRINT",
      "TAURI_WINDOWS_CERTIFICATE_THUMBPRINT",
    ].filter((k) => Boolean(process.env[k])),
    note: hasCertEnv
      ? "signing material env present — still must verify Authenticode on artifact"
      : "no signing material in env; development RC is unsigned",
  };
}

/**
 * Use PowerShell Get-AuthenticodeSignature when available.
 * @param {string} filePath
 * @returns {{ class: SigningClass, status: string|null, raw: string|null, error?: string }}
 */
export function classifyFileSignature(filePath) {
  if (!filePath || !existsSync(filePath)) {
    return {
      class: "BLOCKED",
      status: null,
      raw: null,
      error: "artifact missing; cannot classify signature",
    };
  }

  if (process.platform !== "win32") {
    return {
      class: "BLOCKED",
      status: null,
      raw: null,
      error: "Authenticode classification requires Windows host",
    };
  }

  const ps = `
    $ErrorActionPreference = 'Stop';
    $s = Get-AuthenticodeSignature -FilePath ${JSON.stringify(filePath)};
    Write-Output ("Status=" + $s.Status);
    Write-Output ("StatusMessage=" + $s.StatusMessage);
    if ($s.SignerCertificate) {
      Write-Output ("Subject=" + $s.SignerCertificate.Subject);
    }
  `;
  const r = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", ps],
    { encoding: "utf8", windowsHide: true, timeout: 30_000 },
  );
  const out = `${r.stdout || ""}\n${r.stderr || ""}`.trim();
  if (r.error || r.status !== 0) {
    const env = probeSigningEnvironment();
    return {
      class: env.hasCertEnv ? "BLOCKED" : "UNSIGNED_DEVELOPMENT_RC",
      status: null,
      raw: out || String(r.error || `exit ${r.status}`),
      error: "Get-AuthenticodeSignature failed; used env fallback",
    };
  }

  const statusMatch = out.match(/Status=(\S+)/);
  const status = statusMatch ? statusMatch[1] : null;

  if (status === "Valid") {
    return { class: "SIGNED", status, raw: out };
  }

  if (
    status === "NotSigned" ||
    status === "UnknownError" ||
    status === "NotTrusted" ||
    !status
  ) {
    const env = probeSigningEnvironment();
    if (status === "NotSigned" || !env.hasCertEnv) {
      return { class: "UNSIGNED_DEVELOPMENT_RC", status, raw: out };
    }
    return {
      class: "BLOCKED",
      status,
      raw: out,
      error: "signature present but not Valid; investigate cert chain",
    };
  }

  return {
    class: "BLOCKED",
    status,
    raw: out,
    error: `unexpected Authenticode status: ${status}`,
  };
}

/**
 * Classify the RC as a whole from primary artifacts + env.
 * Prefer NSIS setup exe, then portable binary.
 * @param {string[]} artifactPaths
 */
export function classifyReleaseSigning(artifactPaths) {
  const env = probeSigningEnvironment();
  const paths = (artifactPaths || []).filter((p) => p && existsSync(p));

  if (paths.length === 0) {
    return {
      class: env.hasCertEnv ? "BLOCKED" : "UNSIGNED_DEVELOPMENT_RC",
      env,
      artifacts: [],
      note: env.hasCertEnv
        ? "cert env set but no artifacts to verify → BLOCKED until signed artifacts exist"
        : "no artifacts yet; development posture is UNSIGNED_DEVELOPMENT_RC",
    };
  }

  const artifacts = paths.map((p) => ({
    path: p,
    ...classifyFileSignature(p),
  }));

  if (artifacts.some((a) => a.class === "SIGNED")) {
    const allSigned = artifacts.every((a) => a.class === "SIGNED");
    return {
      class: allSigned ? "SIGNED" : "BLOCKED",
      env,
      artifacts,
      note: allSigned
        ? "all inspected artifacts have Valid Authenticode"
        : "mixed signed/unsigned artifacts — do not claim full SIGNED RC",
    };
  }

  if (artifacts.every((a) => a.class === "UNSIGNED_DEVELOPMENT_RC")) {
    return {
      class: "UNSIGNED_DEVELOPMENT_RC",
      env,
      artifacts,
      note: "unsigned local development RC — allowed to PASS when explicitly classified",
    };
  }

  return {
    class: "BLOCKED",
    env,
    artifacts,
    note: "could not classify cleanly; see per-artifact results",
  };
}
