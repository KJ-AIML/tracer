/**
 * Authenticode verification wrappers (W2.4.2-A Part 7).
 */

import { spawnSync } from "node:child_process";
import {
  existsSync,
  openSync,
  readSync,
  closeSync,
  statSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { SIGNING_MODES } from "./modes.mjs";

export function fileSha256(filePath) {
  return createHash("sha256").update(readFileSync(filePath)).digest("hex");
}

/** Heuristic: PE MZ header present (Windows EXE/DLL). */
export function looksLikePe(filePath) {
  if (!existsSync(filePath)) return false;
  const fd = openSync(filePath, "r");
  try {
    const buf = Buffer.alloc(2);
    readSync(fd, buf, 0, 2, 0);
    return buf[0] === 0x4d && buf[1] === 0x5a;
  } finally {
    closeSync(fd);
  }
}

/**
 * Inspect Authenticode via PowerShell Get-AuthenticodeSignature.
 */
export function inspectAuthenticode(filePath) {
  if (!filePath || !existsSync(filePath)) {
    return {
      ok: false,
      signaturePresent: false,
      signatureStatus: "MISSING_FILE",
      error: "artifact missing",
    };
  }
  if (process.platform !== "win32") {
    return {
      ok: false,
      signaturePresent: false,
      signatureStatus: "UNSUPPORTED_PLATFORM",
      error: "Authenticode inspection requires Windows",
    };
  }

  const ps = `
    $ErrorActionPreference = 'Stop';
    $s = Get-AuthenticodeSignature -FilePath ${JSON.stringify(filePath)};
    $o = [ordered]@{
      Status = [string]$s.Status
      StatusMessage = [string]$s.StatusMessage
      SignatureType = [string]$s.SignatureType
      IsOSBinary = [bool]$s.IsOSBinary
    };
    if ($s.SignerCertificate) {
      $c = $s.SignerCertificate;
      $o.Subject = [string]$c.Subject;
      $o.Issuer = [string]$c.Issuer;
      $o.Thumbprint = [string]$c.Thumbprint;
      $o.NotBefore = $c.NotBefore.ToUniversalTime().ToString('o');
      $o.NotAfter = $c.NotAfter.ToUniversalTime().ToString('o');
    }
    if ($s.TimeStamperCertificate) {
      $o.TimestampSubject = [string]$s.TimeStamperCertificate.Subject;
      $o.TimestampPresent = $true;
    } else {
      $o.TimestampPresent = $false;
    }
    $o | ConvertTo-Json -Compress
  `;
  const r = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", ps],
    { encoding: "utf8", windowsHide: true, timeout: 45_000 },
  );
  const out = (r.stdout || "").trim();
  if (r.error || r.status !== 0 || !out) {
    return {
      ok: false,
      signaturePresent: false,
      signatureStatus: "INSPECT_FAILED",
      error: r.stderr || String(r.error || `exit ${r.status}`),
      raw: out,
    };
  }
  let parsed;
  try {
    parsed = JSON.parse(out);
  } catch (e) {
    return {
      ok: false,
      signaturePresent: false,
      signatureStatus: "INSPECT_PARSE_FAILED",
      error: e instanceof Error ? e.message : String(e),
      raw: out,
    };
  }

  const status = parsed.Status || "Unknown";
  const signaturePresent = status !== "NotSigned";
  return {
    ok: true,
    signaturePresent,
    signatureStatus: status,
    statusMessage: parsed.StatusMessage || null,
    signatureType: parsed.SignatureType || null,
    certificateSubject: parsed.Subject || null,
    certificateIssuer: parsed.Issuer || null,
    certificateThumbprint: parsed.Thumbprint || null,
    certificateValidity:
      parsed.NotBefore && parsed.NotAfter
        ? { notBefore: parsed.NotBefore, notAfter: parsed.NotAfter }
        : null,
    timestampPresent: Boolean(parsed.TimestampPresent),
    timestampAuthority: parsed.TimestampSubject || null,
    raw: parsed,
  };
}

export function classifyInspection(
  insp,
  { expectedSubject = null, requireTimestamp = false } = {},
) {
  if (!insp || !insp.ok) {
    return {
      valid: false,
      classification: insp?.signatureStatus || "INSPECT_FAILED",
      signingMode: SIGNING_MODES.UNSIGNED,
    };
  }
  if (!insp.signaturePresent || insp.signatureStatus === "NotSigned") {
    return {
      valid: false,
      classification: "UNSIGNED",
      signingMode: SIGNING_MODES.UNSIGNED,
    };
  }

  const now = Date.now();
  const nb = insp.certificateValidity?.notBefore
    ? Date.parse(insp.certificateValidity.notBefore)
    : null;
  const na = insp.certificateValidity?.notAfter
    ? Date.parse(insp.certificateValidity.notAfter)
    : null;
  if (nb != null && Number.isFinite(nb) && now < nb) {
    return {
      valid: false,
      classification: "CERTIFICATE_NOT_YET_VALID",
      signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
    };
  }
  if (na != null && Number.isFinite(na) && now > na) {
    return {
      valid: false,
      classification: "CERTIFICATE_EXPIRED",
      signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
    };
  }
  if (expectedSubject && insp.certificateSubject) {
    const norm = (s) => String(s).replace(/\s+/g, " ").trim().toLowerCase();
    const cn = expectedSubject.match(/CN=([^,]+)/i)?.[1];
    const subjectOk =
      norm(insp.certificateSubject) === norm(expectedSubject) ||
      (cn && norm(insp.certificateSubject).includes(norm(cn)));
    if (!subjectOk) {
      return {
        valid: false,
        classification: "WRONG_CERTIFICATE_SUBJECT",
        signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
      };
    }
  }
  if (requireTimestamp && !insp.timestampPresent) {
    return {
      valid: false,
      classification: "MISSING_TIMESTAMP",
      signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
    };
  }

  const isTestSubject = /Self-Signed Test Only|Tracer Test/i.test(
    insp.certificateSubject || "",
  );
  const selfIssued =
    insp.certificateSubject &&
    insp.certificateIssuer &&
    insp.certificateSubject === insp.certificateIssuer;

  if (insp.signatureStatus === "HashMismatch") {
    return {
      valid: false,
      classification: "TAMPERED_OR_HASH_MISMATCH",
      signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
    };
  }

  if (insp.signatureStatus === "Valid" && !isTestSubject && !selfIssued) {
    return {
      valid: true,
      classification: "VALID_TRUSTED",
      signingMode: SIGNING_MODES.TRUSTED_AUTHENTICODE,
    };
  }

  if (
    insp.signatureStatus === "Valid" ||
    insp.signatureStatus === "NotTrusted" ||
    insp.signatureStatus === "UnknownError"
  ) {
    // Self-signed roots commonly surface as UnknownError ("root not trusted").
    // Cryptographic presence + subject is enough for SELF_SIGNED_TEST mechanics.
    const selfSignedOk =
      isTestSubject ||
      selfIssued ||
      insp.signatureStatus === "UnknownError" ||
      insp.signatureStatus === "NotTrusted";
    return {
      valid:
        insp.signatureStatus === "Valid" ||
        insp.signatureStatus === "NotTrusted" ||
        (insp.signatureStatus === "UnknownError" && selfSignedOk),
      classification:
        insp.signatureStatus === "Valid"
          ? isTestSubject || selfIssued
            ? "VALID_SELF_SIGNED_OR_LOCAL"
            : "VALID_TRUSTED_OR_LOCAL"
          : insp.signatureStatus === "NotTrusted"
            ? "PRESENT_NOT_TRUSTED"
            : "PRESENT_SELF_SIGNED_UNTRUSTED_ROOT",
      signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
    };
  }

  return {
    valid: false,
    classification: `STATUS_${insp.signatureStatus}`,
    signingMode: SIGNING_MODES.UNSIGNED,
  };
}

export function verifySignature(filePath, opts = {}) {
  const size = existsSync(filePath) ? statSync(filePath).size : 0;
  const hash = existsSync(filePath) ? fileSha256(filePath) : null;
  const insp = inspectAuthenticode(filePath);
  const cls = classifyInspection(insp, opts);
  return {
    path: filePath,
    artifactSha256: hash,
    sizeBytes: size,
    ...insp,
    ...cls,
    rejected: !cls.valid,
  };
}

/** Flip a byte past the PE header to invalidate Authenticode digests. */
export function tamperFileInPlace(filePath) {
  const buf = readFileSync(filePath);
  if (buf.length < 256) throw new Error("file too small to tamper safely");
  const idx = Math.min(200, buf.length - 1);
  buf[idx] = buf[idx] ^ 0xff;
  writeFileSync(filePath, buf);
  return { path: filePath, tamperedAtOffset: idx, postTamperSha256: fileSha256(filePath) };
}

/**
 * Build explicit unsigned provenance signing fields (Part 10).
 */
export function unsignedSigningFields() {
  return {
    artifactSha256: null,
    preSignSha256: null,
    postSignSha256: null,
    signaturePresent: false,
    signatureStatus: "UNSIGNED",
    certificateSubject: null,
    certificateThumbprint: null,
    certificateIssuer: null,
    certificateValidity: null,
    timestampPresent: false,
    timestampAuthority: null,
    signingMode: SIGNING_MODES.UNSIGNED,
    signingTool: null,
    signingToolVersion: null,
  };
}

export function signingFieldsFromVerify(verifyResult, meta = {}) {
  const base = unsignedSigningFields();
  if (!verifyResult) return { ...base, ...meta };
  return {
    artifactSha256: verifyResult.artifactSha256 || meta.artifactSha256 || null,
    preSignSha256: meta.preSignSha256 ?? null,
    postSignSha256:
      meta.postSignSha256 ?? verifyResult.artifactSha256 ?? null,
    signaturePresent: Boolean(verifyResult.signaturePresent),
    signatureStatus:
      verifyResult.signatureStatus ||
      verifyResult.classification ||
      "UNSIGNED",
    certificateSubject: verifyResult.certificateSubject || null,
    certificateThumbprint: verifyResult.certificateThumbprint || null,
    certificateIssuer: verifyResult.certificateIssuer || null,
    certificateValidity: verifyResult.certificateValidity || null,
    timestampPresent: Boolean(verifyResult.timestampPresent),
    timestampAuthority: verifyResult.timestampAuthority || null,
    signingMode: verifyResult.signingMode || SIGNING_MODES.UNSIGNED,
    signingTool: meta.signingTool || null,
    signingToolVersion: meta.signingToolVersion || null,
  };
}