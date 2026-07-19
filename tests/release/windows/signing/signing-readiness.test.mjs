import test from "node:test";
import assert from "node:assert/strict";
import {
  SIGNING_MODES,
  DOCTOR_CLASSES,
  modeToLegacyClass,
  TEST_ONLY_SUBJECT,
} from "../../../../tools/release/signing/modes.mjs";
import {
  redactSecretText,
  safeEnvSnapshot,
  assertTrustedSigningAllowed,
  isGenericCi,
  createProtectedTempDir,
  secureRemoveDir,
  writeProtectedTempFile,
  secureRemove,
} from "../../../../tools/release/signing/secrets.mjs";
import { detectSigningTools } from "../../../../tools/release/signing/detect-tools.mjs";
import {
  unsignedSigningFields,
  classifyInspection,
  signingFieldsFromVerify,
} from "../../../../tools/release/signing/verify.mjs";
import { resolveSigningMode } from "../../../../tools/release/signing/sign.mjs";
import { runSigningDoctor } from "../../../../tools/release/signing/doctor.mjs";

test("signing modes are distinct", () => {
  assert.equal(SIGNING_MODES.UNSIGNED, "UNSIGNED");
  assert.equal(SIGNING_MODES.SELF_SIGNED_TEST, "SELF_SIGNED_TEST");
  assert.equal(SIGNING_MODES.TRUSTED_AUTHENTICODE, "TRUSTED_AUTHENTICODE");
  assert.equal(
    modeToLegacyClass(SIGNING_MODES.SELF_SIGNED_TEST),
    "UNSIGNED_DEVELOPMENT_RC",
  );
  assert.ok(TEST_ONLY_SUBJECT.includes("Self-Signed Test Only"));
});

test("default resolveSigningMode is UNSIGNED", () => {
  assert.equal(resolveSigningMode({}, {}), SIGNING_MODES.UNSIGNED);
  assert.equal(
    resolveSigningMode({}, { TRACER_SIGNING_MODE: "TRUSTED_AUTHENTICODE" }),
    SIGNING_MODES.TRUSTED_AUTHENTICODE,
  );
});

test("unsigned signing fields are explicit", () => {
  const f = unsignedSigningFields();
  assert.equal(f.signaturePresent, false);
  assert.equal(f.signatureStatus, "UNSIGNED");
  assert.equal(f.signingMode, "UNSIGNED");
  assert.equal(f.timestampPresent, false);
});

test("secret redaction removes private key pem and passwords", () => {
  const pem =
    "-----BEGIN PRIVATE KEY-----\nABC\n-----END PRIVATE KEY-----";
  const out = redactSecretText("pw=sekrit " + pem, ["sekrit"]);
  assert.ok(!out.includes("sekrit"));
  assert.ok(!out.includes("BEGIN PRIVATE KEY"));
  assert.ok(out.includes("[REDACTED]"));
});

test("safeEnvSnapshot never echoes sensitive values", () => {
  const snap = safeEnvSnapshot({
    TRACER_CODE_SIGN_CERTIFICATE_PASSWORD: "super-secret",
    TRACER_SIGNING_MODE: "UNSIGNED",
    TRACER_CODE_SIGN_THUMBPRINT: "AABBCCDDEEFF00112233445566778899AABBCCDD",
  });
  assert.equal(snap.TRACER_CODE_SIGN_CERTIFICATE_PASSWORD.value, "[REDACTED]");
  assert.equal(snap.TRACER_SIGNING_MODE.value, "UNSIGNED");
  assert.ok(!String(snap.TRACER_CODE_SIGN_THUMBPRINT.value).includes("EEFF"));
});

test("generic CI isolation blocks trusted signing", () => {
  assert.equal(isGenericCi({ CI: "true" }), true);
  assert.equal(
    isGenericCi({ CI: "true", TRACER_RELEASE_SIGNING_WORKFLOW: "1" }),
    false,
  );
  assert.equal(assertTrustedSigningAllowed({ CI: "true" }).ok, false);
  assert.equal(assertTrustedSigningAllowed({}).ok, false);
  assert.equal(
    assertTrustedSigningAllowed({ TRACER_SIGNING_AUTHORIZED: "1" }).ok,
    true,
  );
});

test("temporary secret cleanup removes files", () => {
  const dir = createProtectedTempDir("tracer-sign-unit-");
  const f = writeProtectedTempFile(dir, "secret.bin", Buffer.from("secret-bytes"));
  assert.equal(secureRemove(f).removed, true);
  assert.equal(secureRemoveDir(dir).removed, true);
});

test("classifyInspection covers unsigned/wrong subject/expired/not-yet-valid/timestamp/tamper", () => {
  assert.equal(
    classifyInspection({
      ok: true,
      signaturePresent: false,
      signatureStatus: "NotSigned",
    }).classification,
    "UNSIGNED",
  );
  const future = new Date(Date.now() + 86400000).toISOString();
  const past = new Date(Date.now() - 86400000).toISOString();
  assert.equal(
    classifyInspection(
      {
        ok: true,
        signaturePresent: true,
        signatureStatus: "Valid",
        certificateSubject: "CN=Other",
        certificateValidity: { notBefore: past, notAfter: future },
      },
      { expectedSubject: "CN=Expected Publisher" },
    ).classification,
    "WRONG_CERTIFICATE_SUBJECT",
  );
  assert.equal(
    classifyInspection({
      ok: true,
      signaturePresent: true,
      signatureStatus: "Valid",
      certificateSubject: "CN=X",
      certificateValidity: { notBefore: past, notAfter: past },
    }).classification,
    "CERTIFICATE_EXPIRED",
  );
  assert.equal(
    classifyInspection({
      ok: true,
      signaturePresent: true,
      signatureStatus: "Valid",
      certificateSubject: "CN=X",
      certificateValidity: {
        notBefore: future,
        notAfter: new Date(Date.now() + 172800000).toISOString(),
      },
    }).classification,
    "CERTIFICATE_NOT_YET_VALID",
  );
  assert.equal(
    classifyInspection(
      {
        ok: true,
        signaturePresent: true,
        signatureStatus: "Valid",
        certificateSubject: "CN=X",
        certificateIssuer: "CN=X",
        certificateValidity: { notBefore: past, notAfter: future },
        timestampPresent: false,
      },
      { requireTimestamp: true },
    ).classification,
    "MISSING_TIMESTAMP",
  );
  assert.equal(
    classifyInspection({
      ok: true,
      signaturePresent: true,
      signatureStatus: "HashMismatch",
      certificateSubject: "CN=X",
    }).classification,
    "TAMPERED_OR_HASH_MISMATCH",
  );
});

test("signingFieldsFromVerify maps verify result", () => {
  const fields = signingFieldsFromVerify(
    {
      artifactSha256: "abc",
      signaturePresent: true,
      signatureStatus: "Valid",
      certificateSubject: "CN=Tracer Self-Signed Test Only",
      signingMode: SIGNING_MODES.SELF_SIGNED_TEST,
      timestampPresent: false,
    },
    {
      preSignSha256: "pre",
      postSignSha256: "post",
      signingTool: "powershell",
    },
  );
  assert.equal(fields.signaturePresent, true);
  assert.equal(fields.preSignSha256, "pre");
  assert.equal(fields.postSignSha256, "post");
  assert.equal(fields.signingTool, "powershell");
});

test("detectSigningTools reports platform without assuming signtool", () => {
  const d = detectSigningTools();
  assert.ok(d.platform);
  assert.equal(typeof d.anyAvailable, "boolean");
  assert.equal(typeof d.tools.signtool.available, "boolean");
});

test("doctor is non-destructive and classifies without trusted cert", () => {
  const env = { ...process.env };
  delete env.TRACER_SIGNING_AUTHORIZED;
  delete env.TRACER_CODE_SIGN_THUMBPRINT;
  delete env.TRACER_CODE_SIGN_CERTIFICATE_PATH;
  delete env.WINDOWS_CERTIFICATE_THUMBPRINT;
  const report = runSigningDoctor({ env });
  assert.equal(report.kind, "tracer-signing-doctor");
  assert.equal(report.secretsLogged, false);
  assert.ok(
    [
      DOCTOR_CLASSES.READY_SELF_SIGNED_TEST_ONLY,
      DOCTOR_CLASSES.BLOCKED_NO_SIGNING_TOOL,
      DOCTOR_CLASSES.UNSUPPORTED_PLATFORM,
      DOCTOR_CLASSES.BLOCKED_NO_CERTIFICATE,
    ].includes(report.classification),
  );
  assert.ok(
    [
      "BLOCKED_NO_CERTIFICATE",
      "BLOCKED_NO_SIGNING_TOOL",
      "UNSUPPORTED_PLATFORM",
    ].includes(report.trustedReadiness),
  );
});

test("doctor classification vocabulary includes publisher and cert ready states", () => {
  assert.ok(
    Object.values(DOCTOR_CLASSES).includes("READY_WITH_CERTIFICATE"),
  );
  assert.ok(
    Object.values(DOCTOR_CLASSES).includes("BLOCKED_NO_PUBLISHER_IDENTITY"),
  );
});

test("standard CI path never auto-authorizes trusted signing", () => {
  const env = { CI: "true", TRACER_SIGNING_AUTHORIZED: "1" };
  assert.equal(isGenericCi(env), true);
  assert.equal(assertTrustedSigningAllowed(env).ok, false);
});
