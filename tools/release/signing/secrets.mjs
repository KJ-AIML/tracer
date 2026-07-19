/**
 * Secret-handling contract for Authenticode signing (W2.4.2-A Part 6).
 *
 * Rules (must hold for all signing entrypoints):
 *  1. Accept signing material only through explicit operator / secret-store config
 *  2. Never log private-key material
 *  3. Never log certificate passwords
 *  4. Redact sensitive environment variables
 *  5. Avoid storing secrets in repository paths
 *  6. Use protected temporary files only when unavoidable
 *  7. Remove temporary signing material after success and failure
 *  8. Verify cleanup
 *  9. Prevent PR / generic CI builds from accessing signing secrets
 * 10. Require explicit release-signing authorization
 *
 * Allowed env var *names* (values never documented or logged):
 *  - TRACER_SIGNING_AUTHORIZED=1          (explicit trusted-sign gate)
 *  - TRACER_CODE_SIGN_CERTIFICATE_PATH    (PFX/P12 path outside repo)
 *  - TRACER_CODE_SIGN_CERTIFICATE_PASSWORD
 *  - TRACER_CODE_SIGN_THUMBPRINT          (Windows cert store)
 *  - TRACER_CODE_SIGN_SUBJECT             (expected publisher subject)
 *  - TRACER_TIMESTAMP_URL                 (RFC3161 timestamp server)
 *  - TRACER_SIGNING_MODE                  (UNSIGNED|SELF_SIGNED_TEST|TRUSTED_AUTHENTICODE)
 *  - WINDOWS_CERTIFICATE_THUMBPRINT       (Tauri-compatible alias)
 *  - TAURI_SIGNING_PRIVATE_KEY            (Tauri updater key — not Authenticode; redacted)
 */

import {
  existsSync,
  rmSync,
  unlinkSync,
  mkdtempSync,
  writeFileSync,
  readFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";

/** Env keys whose values must never appear in logs or provenance. */
export const SENSITIVE_ENV_KEYS = Object.freeze([
  "TRACER_CODE_SIGN_CERTIFICATE_PASSWORD",
  "TRACER_CODE_SIGN_CERTIFICATE_PATH",
  "TAURI_SIGNING_PRIVATE_KEY",
  "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
  "WINDOWS_CERTIFICATE_PASSWORD",
  "CSC_KEY_PASSWORD",
  "CSC_LINK",
]);

/** Env keys that may be referenced by name but values are still redacted in dumps. */
export const CERT_REF_ENV_KEYS = Object.freeze([
  "TRACER_CODE_SIGN_THUMBPRINT",
  "WINDOWS_CERTIFICATE_THUMBPRINT",
  "TAURI_WINDOWS_CERTIFICATE_THUMBPRINT",
  "TRACER_CODE_SIGN_SUBJECT",
  "TRACER_TIMESTAMP_URL",
  "TRACER_SIGNING_MODE",
  "TRACER_SIGNING_AUTHORIZED",
]);

const REDACTED = "[REDACTED]";

/**
 * Redact a single string that might contain a password or key blob.
 */
export function redactSecretText(text, extraSecrets = []) {
  if (text == null) return text;
  let out = String(text);
  for (const s of extraSecrets) {
    if (s && String(s).length > 0) {
      out = out.split(String(s)).join(REDACTED);
    }
  }
  // PEM / base64-ish private key blobs
  out = out.replace(
    /-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z0-9 ]*PRIVATE KEY-----/g,
    REDACTED,
  );
  return out;
}

/**
 * Safe env snapshot for doctor / provenance (no secret values).
 */
export function safeEnvSnapshot(env = process.env) {
  const snap = {};
  for (const k of [...SENSITIVE_ENV_KEYS, ...CERT_REF_ENV_KEYS]) {
    if (env[k] == null || env[k] === "") {
      snap[k] = { present: false };
    } else if (SENSITIVE_ENV_KEYS.includes(k)) {
      snap[k] = { present: true, value: REDACTED };
    } else if (k === "TRACER_CODE_SIGN_THUMBPRINT" || k.includes("THUMBPRINT")) {
      const v = String(env[k]);
      snap[k] = {
        present: true,
        value: v.length <= 8 ? REDACTED : `${v.slice(0, 4)}…${v.slice(-4)}`,
      };
    } else {
      // subject / url / mode / authorized — non-secret configuration
      snap[k] = { present: true, value: String(env[k]) };
    }
  }
  return snap;
}

/**
 * Explicit authorization required for TRUSTED_AUTHENTICODE.
 */
export function isTrustedSigningAuthorized(env = process.env) {
  const v = String(env.TRACER_SIGNING_AUTHORIZED || "").trim();
  return v === "1" || v.toLowerCase() === "true" || v.toLowerCase() === "yes";
}

/**
 * Generic CI / PR isolation: treat CI without explicit release auth as blocked.
 */
export function isGenericCi(env = process.env) {
  const ci =
    env.CI === "true" ||
    env.CI === "1" ||
    env.GITHUB_ACTIONS === "true" ||
    env.GITLAB_CI === "true" ||
    env.TF_BUILD === "True";
  const releaseWorkflow =
    env.TRACER_RELEASE_SIGNING_WORKFLOW === "1" ||
    env.TRACER_RELEASE_SIGNING_WORKFLOW === "true";
  return Boolean(ci && !releaseWorkflow);
}

/**
 * Refuse trusted signing when generic CI or missing authorization.
 */
export function assertTrustedSigningAllowed(env = process.env) {
  if (isGenericCi(env)) {
    return {
      ok: false,
      reason: "generic CI isolation: trusted signing forbidden without TRACER_RELEASE_SIGNING_WORKFLOW=1",
    };
  }
  if (!isTrustedSigningAuthorized(env)) {
    return {
      ok: false,
      reason: "trusted signing requires TRACER_SIGNING_AUTHORIZED=1",
    };
  }
  return { ok: true };
}

/**
 * Create a protected temp dir under OS temp (never under repo).
 */
export function createProtectedTempDir(prefix = "tracer-sign-") {
  const dir = mkdtempSync(path.join(os.tmpdir(), prefix));
  return dir;
}

/**
 * Write bytes to a temp file; caller must cleanup.
 */
export function writeProtectedTempFile(dir, name, contents) {
  const p = path.join(dir, name);
  writeFileSync(p, contents, { mode: 0o600 });
  return p;
}

/**
 * Remove path if it exists; returns whether it is gone.
 */
export function secureRemove(filePath) {
  if (!filePath) return { removed: true, missing: true };
  try {
    if (!existsSync(filePath)) return { removed: true, missing: true };
    try {
      // Best-effort overwrite for small secret files before unlink
      const st = readFileSync(filePath);
      writeFileSync(filePath, Buffer.alloc(st.length, 0));
    } catch {
      /* ignore overwrite failures */
    }
    unlinkSync(filePath);
    return { removed: !existsSync(filePath), missing: false };
  } catch (e) {
    try {
      rmSync(filePath, { force: true });
    } catch {
      /* ignore */
    }
    return {
      removed: !existsSync(filePath),
      missing: false,
      error: e instanceof Error ? e.message : String(e),
    };
  }
}

/**
 * Recursively remove a temp directory and verify absence.
 */
export function secureRemoveDir(dirPath) {
  if (!dirPath) return { removed: true };
  try {
    rmSync(dirPath, { recursive: true, force: true });
  } catch (e) {
    return {
      removed: !existsSync(dirPath),
      error: e instanceof Error ? e.message : String(e),
    };
  }
  return { removed: !existsSync(dirPath) };
}

/**
 * Guard: refuse paths that resolve inside the repo worktree for secret storage.
 */
export function assertOutsideRepo(secretPath, repoRoot) {
  if (!secretPath || !repoRoot) return { ok: true };
  const abs = path.resolve(secretPath);
  const root = path.resolve(repoRoot);
  const rel = path.relative(root, abs);
  if (rel && !rel.startsWith("..") && !path.isAbsolute(rel)) {
    return {
      ok: false,
      reason: "signing secrets must not be stored under the repository tree",
    };
  }
  return { ok: true };
}
