/**
 * Non-destructive signing doctor (W2.4.2-A Part 8).
 * Never signs or modifies artifacts.
 */

import { existsSync } from "node:fs";
import path from "node:path";
import { DOCTOR_CLASSES, SIGNING_MODES, TEST_ONLY_SUBJECT } from "./modes.mjs";
import { detectSigningTools } from "./detect-tools.mjs";
import {
  safeEnvSnapshot,
  isTrustedSigningAuthorized,
  isGenericCi,
} from "./secrets.mjs";
import { discoverArtifacts } from "../lib/artifacts.mjs";
import { REPO_ROOT } from "../lib/paths.mjs";

function publisherIdentityStatus(env = process.env) {
  const subject =
    env.TRACER_CODE_SIGN_SUBJECT ||
    env.TRACER_PUBLISHER_SUBJECT ||
    null;
  const configured = Boolean(subject && String(subject).trim());
  // Tauri publisher field is product metadata, not Authenticode publisher identity.
  if (!configured) {
    return {
      status: "UNPROVEN",
      expectedSubject: null,
      note: "no TRACER_CODE_SIGN_SUBJECT configured; Authenticode publisher identity unproven",
    };
  }
  return {
    status: "CONFIGURED_UNVERIFIED",
    expectedSubject: String(subject).trim(),
    note: "subject configured but not cryptographically proven without trusted cert",
  };
}

function certificateAvailability(env = process.env) {
  const thumb =
    env.TRACER_CODE_SIGN_THUMBPRINT ||
    env.WINDOWS_CERTIFICATE_THUMBPRINT ||
    env.TAURI_WINDOWS_CERTIFICATE_THUMBPRINT ||
    null;
  const pfx = env.TRACER_CODE_SIGN_CERTIFICATE_PATH || null;
  const hasThumb = Boolean(thumb && String(thumb).trim() && !String(thumb).includes("REPLACE"));
  const hasPfx = Boolean(pfx && existsSync(pfx));
  if (hasThumb || hasPfx) {
    return {
      category: "TRUSTED_MATERIAL_REFERENCED",
      thumbprintPresent: hasThumb,
      pfxPathPresent: hasPfx,
      // Never echo path under repo or password
      note: "certificate reference present in env — validity not proven by doctor",
    };
  }
  return {
    category: "NONE",
    thumbprintPresent: false,
    pfxPathPresent: false,
    note: "no trusted certificate thumbprint or PFX configured",
  };
}

function timestampStatus(env = process.env) {
  const url = env.TRACER_TIMESTAMP_URL || null;
  if (!url) {
    return {
      status: "UNPROVEN",
      url: null,
      note: "TRACER_TIMESTAMP_URL unset; timestamping optional for self-signed test, required policy for production",
    };
  }
  // Doctor does not contact the TSA (non-destructive / no network by default).
  return {
    status: "CONFIGURED_UNVERIFIED",
    url: String(url),
    note: "timestamp URL configured; live TSA reachability not probed by doctor",
  };
}

/**
 * Run signing doctor. Pure inspection.
 */
export function runSigningDoctor(opts = {}) {
  const env = opts.env || process.env;
  const tools = detectSigningTools();
  const artifacts = discoverArtifacts();
  const cert = certificateAvailability(env);
  const publisher = publisherIdentityStatus(env);
  const timestamp = timestampStatus(env);
  const authorized = isTrustedSigningAuthorized(env);
  const genericCi = isGenericCi(env);

  /** @type {string} */
  let classification;
  /** @type {string[]} */
  const remediation = [];

  if (!tools.windows) {
    classification = DOCTOR_CLASSES.UNSUPPORTED_PLATFORM;
    remediation.push("Run signing doctor on a Windows host");
  } else if (!tools.anyAvailable) {
    classification = DOCTOR_CLASSES.BLOCKED_NO_SIGNING_TOOL;
    remediation.push(
      "Install Windows SDK SignTool, AzureSignTool, or ensure PowerShell Set-AuthenticodeSignature is available",
    );
  } else if (cert.category === "TRUSTED_MATERIAL_REFERENCED" && authorized) {
    if (publisher.status === "UNPROVEN") {
      classification = DOCTOR_CLASSES.BLOCKED_NO_PUBLISHER_IDENTITY;
      remediation.push("Set TRACER_CODE_SIGN_SUBJECT to the expected Authenticode publisher DN");
    } else {
      classification = DOCTOR_CLASSES.READY_WITH_CERTIFICATE;
      remediation.push(
        "Run authorized trusted signing via pnpm release:sign with TRACER_SIGNING_MODE=TRUSTED_AUTHENTICODE",
      );
    }
  } else if (cert.category === "TRUSTED_MATERIAL_REFERENCED" && !authorized) {
    classification = DOCTOR_CLASSES.READY_SELF_SIGNED_TEST_ONLY;
    remediation.push(
      "Trusted material referenced but TRACER_SIGNING_AUTHORIZED not set — use self-signed test or authorize trusted signing explicitly",
    );
  } else if (tools.anyAvailable) {
    classification = DOCTOR_CLASSES.READY_SELF_SIGNED_TEST_ONLY;
    remediation.push(
      "No trusted certificate: run pnpm release:sign:test for mechanics proof only",
    );
    remediation.push(
      "For TRUSTED_AUTHENTICODE: provision org cert / managed signing; set thumbprint or PFX via secret store; set TRACER_SIGNING_AUTHORIZED=1",
    );
  } else {
    classification = DOCTOR_CLASSES.BLOCKED_NO_CERTIFICATE;
    remediation.push("Provision a code-signing certificate or use self-signed test tooling");
  }

  // Trusted readiness separate from doctor class for Part 17
  let trustedReadiness = "BLOCKED_NO_CERTIFICATE";
  if (!tools.windows) trustedReadiness = "UNSUPPORTED_PLATFORM";
  else if (!tools.anyAvailable) trustedReadiness = "BLOCKED_NO_SIGNING_TOOL";
  else if (cert.category !== "TRUSTED_MATERIAL_REFERENCED") {
    trustedReadiness = "BLOCKED_NO_CERTIFICATE";
  } else if (publisher.status === "UNPROVEN") {
    trustedReadiness = "BLOCKED_NO_PUBLISHER_IDENTITY";
  } else if (!authorized) {
    trustedReadiness = "BLOCKED_NO_CERTIFICATE"; // material present but not authorized — still not READY
  } else {
    trustedReadiness = "READY_WITH_CERTIFICATE";
  }

  const artifactList = [];
  if (artifacts.portable) {
    artifactList.push({
      type: "portable",
      relative: path.relative(REPO_ROOT, artifacts.portable).replace(/\\/g, "/"),
      exists: true,
    });
  }
  for (const n of artifacts.nsis) {
    artifactList.push({
      type: "nsis",
      relative: path.relative(REPO_ROOT, n).replace(/\\/g, "/"),
      exists: true,
    });
  }

  return {
    kind: "tracer-signing-doctor",
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    classification,
    trustedReadiness,
    defaultMode: SIGNING_MODES.UNSIGNED,
    testOnlySubjectExample: TEST_ONLY_SUBJECT,
    platform: tools.platform,
    osRelease: tools.osRelease,
    signingTool: tools.preferred,
    tools: {
      signtool: {
        available: tools.tools.signtool.available,
        path: tools.tools.signtool.path,
        version: tools.tools.signtool.version,
      },
      azureSignTool: {
        available: tools.tools.azureSignTool.available,
        version: tools.tools.azureSignTool.version,
      },
      powershellAuthenticode: {
        available: tools.tools.powershellAuthenticode.available,
        version: tools.tools.powershellAuthenticode.version,
      },
    },
    certificate: cert,
    publisher,
    timestamp,
    authorization: {
      trustedAuthorized: authorized,
      genericCi,
      note: "doctor never signs; trusted signing requires TRACER_SIGNING_AUTHORIZED=1 and non-generic CI or release workflow",
    },
    artifacts: artifactList,
    env: safeEnvSnapshot(env),
    remediation,
    secretsLogged: false,
  };
}