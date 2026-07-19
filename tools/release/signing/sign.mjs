/**
 * Authenticode sign wrappers (W2.4.2-A Part 7).
 *
 * Modes:
 *  - UNSIGNED: no-op / refuse to invent signatures
 *  - SELF_SIGNED_TEST: PowerShell or signtool with test-only cert
 *  - TRUSTED_AUTHENTICODE: requires authorization + real material; fails closed
 */

import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync } from "node:fs";
import path from "node:path";
import { SIGNING_MODES } from "./modes.mjs";
import { detectSigningTools } from "./detect-tools.mjs";
import {
  assertTrustedSigningAllowed,
  isGenericCi,
  redactSecretText,
} from "./secrets.mjs";
import { fileSha256, inspectAuthenticode, verifySignature } from "./verify.mjs";

function runPs(script, timeout = 120_000) {
  const r = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { encoding: "utf8", windowsHide: true, timeout },
  );
  return {
    ok: r.status === 0 && !r.error,
    status: r.status,
    stdout: r.stdout || "",
    stderr: r.stderr || "",
    error: r.error ? String(r.error.message || r.error) : null,
  };
}

/**
 * Resolve requested mode from env / opts. Default UNSIGNED.
 */
export function resolveSigningMode(opts = {}, env = process.env) {
  if (opts.mode && Object.values(SIGNING_MODES).includes(opts.mode)) {
    return opts.mode;
  }
  const fromEnv = String(env.TRACER_SIGNING_MODE || "").trim().toUpperCase();
  if (Object.values(SIGNING_MODES).includes(fromEnv)) return fromEnv;
  return SIGNING_MODES.UNSIGNED;
}

/**
 * Sign a single file in-place using PowerShell certificate from store (thumbprint).
 */
export function signWithPowerShellStore(filePath, thumbprint, { timestampUrl = null } = {}) {
  if (!existsSync(filePath)) {
    return { ok: false, error: "file missing", classification: "MISSING_FILE" };
  }
  const ts = timestampUrl
    ? `-TimestampServer ${JSON.stringify(timestampUrl)}`
    : "";
  const ps = `
    $ErrorActionPreference = 'Stop';
    $cert = Get-ChildItem Cert:\\CurrentUser\\My\\${thumbprint} -ErrorAction Stop;
    $r = Set-AuthenticodeSignature -FilePath ${JSON.stringify(filePath)} -Certificate $cert -HashAlgorithm SHA256 ${ts};
    Write-Output ("Status=" + $r.Status);
    Write-Output ("Subject=" + $r.SignerCertificate.Subject);
  `;
  const r = runPs(ps);
  const safeOut = redactSecretText(`${r.stdout}\n${r.stderr}`);
  if (!r.ok) {
    return {
      ok: false,
      error: safeOut || r.error || "Set-AuthenticodeSignature failed",
      classification: "SIGN_FAILED",
      tool: "powershell",
    };
  }
  const insp = inspectAuthenticode(filePath);
  return {
    ok: insp.signaturePresent,
    tool: "powershell",
    toolVersion: null,
    stdout: safeOut,
    inspection: insp,
  };
}

/**
 * Sign with signtool.exe + PFX path (password via env — never logged).
 */
export function signWithSignTool(filePath, { signtoolPath, pfxPath, password, timestampUrl = null, thumbprint = null }) {
  if (!signtoolPath || !existsSync(signtoolPath)) {
    return { ok: false, error: "signtool missing", classification: "BLOCKED_NO_SIGNING_TOOL" };
  }
  const args = ["sign", "/fd", "SHA256", "/td", "SHA256"];
  if (timestampUrl) {
    args.push("/tr", timestampUrl);
  }
  if (pfxPath) {
    args.push("/f", pfxPath);
    if (password) args.push("/p", password);
  } else if (thumbprint) {
    args.push("/sha1", thumbprint);
  } else {
    return { ok: false, error: "no cert material", classification: "BLOCKED_NO_CERTIFICATE" };
  }
  args.push(filePath);
  const r = spawnSync(signtoolPath, args, {
    encoding: "utf8",
    windowsHide: true,
    timeout: 180_000,
  });
  const safeOut = redactSecretText(`${r.stdout || ""}\n${r.stderr || ""}`, [
    password,
  ]);
  if (r.status !== 0 || r.error) {
    return {
      ok: false,
      error: safeOut || String(r.error || `exit ${r.status}`),
      classification: "SIGN_FAILED",
      tool: "signtool",
    };
  }
  return {
    ok: true,
    tool: "signtool",
    toolVersion: null,
    stdout: safeOut,
    inspection: inspectAuthenticode(filePath),
  };
}

/**
 * Copy then sign (never mutate canonical RC unless opts.inPlace).
 */
export function signArtifact(srcPath, destPath, opts = {}) {
  const mode = resolveSigningMode(opts);
  const tools = detectSigningTools();
  const preSignSha256 = existsSync(srcPath) ? fileSha256(srcPath) : null;

  if (mode === SIGNING_MODES.UNSIGNED) {
    return {
      ok: true,
      skipped: true,
      mode,
      preSignSha256,
      postSignSha256: preSignSha256,
      classification: "UNSIGNED",
      note: "default unsigned posture — no signature applied",
    };
  }

  if (!tools.windows) {
    return {
      ok: false,
      mode,
      classification: "UNSUPPORTED_PLATFORM",
      error: "Authenticode signing requires Windows",
    };
  }

  if (!tools.anyAvailable) {
    return {
      ok: false,
      mode,
      classification: "BLOCKED_NO_SIGNING_TOOL",
      error: "no Authenticode signing tool detected",
    };
  }

  if (mode === SIGNING_MODES.TRUSTED_AUTHENTICODE) {
    if (isGenericCi()) {
      return {
        ok: false,
        mode,
        classification: "BLOCKED_CI_ISOLATION",
        error: "trusted signing blocked in generic CI",
      };
    }
    const auth = assertTrustedSigningAllowed();
    if (!auth.ok) {
      return {
        ok: false,
        mode,
        classification: "BLOCKED_NO_AUTHORIZATION",
        error: auth.reason,
      };
    }
    const thumb =
      opts.thumbprint ||
      process.env.TRACER_CODE_SIGN_THUMBPRINT ||
      process.env.WINDOWS_CERTIFICATE_THUMBPRINT;
    const pfx = opts.pfxPath || process.env.TRACER_CODE_SIGN_CERTIFICATE_PATH;
    const password = opts.password || process.env.TRACER_CODE_SIGN_CERTIFICATE_PASSWORD;
    if (!thumb && !pfx) {
      return {
        ok: false,
        mode,
        classification: "BLOCKED_NO_CERTIFICATE",
        error: "no trusted certificate thumbprint or PFX configured",
      };
    }
    if (!opts.inPlace) {
      copyFileSync(srcPath, destPath);
    }
    const target = opts.inPlace ? srcPath : destPath;
    let result;
    if (tools.tools.signtool.available && (pfx || thumb)) {
      result = signWithSignTool(target, {
        signtoolPath: tools.tools.signtool.path,
        pfxPath: pfx,
        password,
        timestampUrl: opts.timestampUrl || process.env.TRACER_TIMESTAMP_URL || null,
        thumbprint: thumb,
      });
    } else if (thumb && tools.tools.powershellAuthenticode.available) {
      result = signWithPowerShellStore(target, thumb, {
        timestampUrl: opts.timestampUrl || process.env.TRACER_TIMESTAMP_URL || null,
      });
    } else {
      return {
        ok: false,
        mode,
        classification: "BLOCKED_NO_SIGNING_TOOL",
        error: "trusted signing tool/cert combination unavailable",
      };
    }
    const postSignSha256 = existsSync(target) ? fileSha256(target) : null;
    return {
      ...result,
      mode,
      preSignSha256,
      postSignSha256,
      dest: target,
    };
  }

  // SELF_SIGNED_TEST
  if (!opts.thumbprint) {
    return {
      ok: false,
      mode,
      classification: "BLOCKED_NO_CERTIFICATE",
      error: "SELF_SIGNED_TEST requires opts.thumbprint from test cert provisioning",
    };
  }
  if (!opts.inPlace) {
    copyFileSync(srcPath, destPath);
  }
  const target = opts.inPlace ? srcPath : destPath;
  const result = signWithPowerShellStore(target, opts.thumbprint, {
    timestampUrl: opts.timestampUrl || null,
  });
  const postSignSha256 = existsSync(target) ? fileSha256(target) : null;
  return {
    ...result,
    mode,
    preSignSha256,
    postSignSha256,
    dest: target,
    signingTool: "powershell",
    signingToolVersion: tools.tools.powershellAuthenticode.version,
  };
}

/**
 * Sign portable + NSIS paths into an output directory (copies).
 */
export function signReleaseArtifacts({ portable, nsis = [], outDir, mode, thumbprint }) {
  const results = [];
  const signOne = (src, label) => {
    if (!src || !existsSync(src)) {
      results.push({ label, ok: false, classification: "MISSING_ARTIFACT", src });
      return;
    }
    const dest = path.join(outDir, path.basename(src));
    const r = signArtifact(src, dest, { mode, thumbprint });
    results.push({ label, src, dest, ...r });
  };
  signOne(portable, "portable");
  for (const n of nsis) signOne(n, "nsis");
  return results;
}

export function verifyReleaseSignatures(paths, opts = {}) {
  return (paths || []).filter(Boolean).map((p) => verifySignature(p, opts));
}