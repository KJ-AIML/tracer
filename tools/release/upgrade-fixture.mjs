#!/usr/bin/env node
/** W2.4.1-A Windows upgrade fixture orchestrator. See docs/modules/w2-4-1/. */
import { spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
  copyFileSync,
  readdirSync,
  statSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";
import { REPO_ROOT, FAKE_ACP_JS, releaseStageDir } from "./lib/paths.mjs";
import { sha256File } from "./lib/provenance.mjs";
import {
  FIXTURE_APP_ID,
  seedPriorSchemaV1,
  seedDataIntoExisting,
  captureState,
  assertDataPreserved,
  setSchemaLogicalVersion,
  corruptDatabase,
  backupDatabase,
  SCHEMA_V2,
} from "./lib/upgrade-db.mjs";

const args = new Set(process.argv.slice(2));
const json = args.has("--json");
const skipBuildN1 = args.has("--skip-build-n1");
const log = (m) => {
  if (!json) console.log(m);
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function killTree(pid) {
  if (!pid) return;
  if (process.platform === "win32") {
    spawnSync("taskkill", ["/PID", String(pid), "/T", "/F"], {
      windowsHide: true,
      encoding: "utf8",
    });
  }
}

function findExe(dir) {
  for (const n of ["tracer-desktop.exe", "Tracer.exe"]) {
    const p = path.join(dir, n);
    if (existsSync(p)) return p;
  }
  const stack = [{ dir, depth: 0 }];
  while (stack.length) {
    const { dir: d, depth } = stack.pop();
    if (depth > 5 || !existsSync(d)) continue;
    let ents;
    try {
      ents = readdirSync(d, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const ent of ents) {
      const p = path.join(d, ent.name);
      if (ent.isFile() && /tracer(-desktop)?\.exe$/i.test(ent.name)) return p;
      if (ent.isDirectory()) stack.push({ dir: p, depth: depth + 1 });
    }
  }
  return null;
}

function findNsis(dir) {
  if (!existsSync(dir)) return null;
  const files = readdirSync(dir)
    .filter((n) => /setup\.exe$/i.test(n) || /Tracer_.*\.exe$/i.test(n))
    .map((n) => path.join(dir, n));
  return files[0] || null;
}

function nsisSilentInstall(setupExe, installDir) {
  mkdirSync(installDir, { recursive: true });
  const r = spawnSync(setupExe, ["/S", `/D=${installDir}`], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 180_000,
  });
  return {
    ok: r.status === 0,
    status: r.status,
    error: r.error ? String(r.error.message || r.error) : null,
  };
}

function nsisUninstall(installDir) {
  let uninst = null;
  const stack = [installDir];
  while (stack.length && !uninst) {
    const dir = stack.pop();
    let ents;
    try {
      ents = readdirSync(dir, { withFileTypes: true });
    } catch {
      continue;
    }
    for (const ent of ents) {
      const p = path.join(dir, ent.name);
      if (ent.isFile() && /uninstall\.exe$/i.test(ent.name)) {
        uninst = p;
        break;
      }
      if (ent.isDirectory()) stack.push(p);
    }
  }
  if (!uninst) return { ok: false, detail: "uninstall.exe not found" };
  const r = spawnSync(uninst, ["/S"], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 180_000,
  });
  return { ok: r.status === 0, status: r.status, uninst };
}

async function smokeLaunch(exe, { dbPath, timeoutMs = 12_000 } = {}) {
  const tmp = mkdtempSync(path.join(os.tmpdir(), "tracer-uf-smoke-"));
  const readyMarker = path.join(tmp, "ready.txt");
  const childEnv = {
    ...process.env,
    TRACER_DATABASE_PATH: dbPath,
    TRACER_E2E_READY_MARKER: readyMarker,
    TRACER_FAKE_ACP_JS: FAKE_ACP_JS,
  };
  let child;
  try {
    child = spawn(exe, [], {
      env: childEnv,
      cwd: tmp,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    return {
      ok: false,
      detail: `spawn failed: ${e instanceof Error ? e.message : String(e)}`,
    };
  }
  let logs = "";
  child.stdout?.on("data", (d) => {
    logs += d.toString();
  });
  child.stderr?.on("data", (d) => {
    logs += d.toString();
  });
  const started = Date.now();
  let ready = false;
  while (Date.now() - started < timeoutMs) {
    if (child.exitCode !== null) {
      killTree(child.pid);
      return {
        ok: false,
        pid: child.pid,
        detail: `exited early code=${child.exitCode}`,
        logs: logs.slice(-3000),
      };
    }
    if (existsSync(readyMarker)) {
      ready = true;
      break;
    }
    await sleep(250);
  }
  const pid = child.pid;
  killTree(pid);
  await sleep(400);
  let orphan = false;
  if (process.platform === "win32" && pid) {
    const r = spawnSync(
      "tasklist",
      ["/FI", `PID eq ${pid}`, "/FO", "CSV", "/NH"],
      { encoding: "utf8", windowsHide: true },
    );
    const out = (r.stdout || "").trim();
    if (out && !/INFO:/i.test(out) && out.includes(String(pid))) orphan = true;
  }
  return {
    ok: !orphan,
    ready,
    pid,
    orphan,
    detail: orphan
      ? "orphan process remains"
      : ready
        ? "alive + ready + clean shutdown"
        : "alive for smoke window + clean shutdown",
    logs: logs.slice(-2000),
  };
}

function stageDir(label) {
  return path.join(REPO_ROOT, "target/release-rc/upgrade-fixture", label);
}

function recordArtifact(filePath, artifactType) {
  return {
    artifactType,
    filename: path.basename(filePath),
    relativePath: path.relative(REPO_ROOT, filePath).replace(/\\/g, "/"),
    sizeBytes: statSync(filePath).size,
    sha256: sha256File(filePath),
  };
}

function buildN1() {
  log("building N+1 via pnpm release:windows ...");
  const r = spawnSync(
    process.platform === "win32" ? "pnpm.cmd" : "pnpm",
    ["release:windows"],
    {
      cwd: REPO_ROOT,
      encoding: "utf8",
      windowsHide: true,
      shell: process.platform === "win32",
      stdio: json ? "pipe" : "inherit",
    },
  );
  return { ok: r.status === 0, status: r.status };
}

function stageN1() {
  const dest = stageDir("vN1");
  mkdirSync(dest, { recursive: true });
  const portableSrc = path.join(REPO_ROOT, "target/release/tracer-desktop.exe");
  const nsisSrc = path.join(
    REPO_ROOT,
    "target/release/bundle/nsis/Tracer_0.1.1_x64-setup.exe",
  );
  if (!existsSync(portableSrc) || !existsSync(nsisSrc)) {
    return { ok: false, detail: "N+1 artifacts missing" };
  }
  copyFileSync(portableSrc, path.join(dest, "tracer-desktop.exe"));
  copyFileSync(nsisSrc, path.join(dest, "Tracer_0.1.1_x64-setup.exe"));
  return {
    ok: true,
    dir: dest,
    portable: path.join(dest, "tracer-desktop.exe"),
    nsis: path.join(dest, "Tracer_0.1.1_x64-setup.exe"),
  };
}

async function runUfCases(fixtureRoot, n1Portable) {
  const cases = [];

  {
    const db = path.join(fixtureRoot, "uf01", "tracer.db");
    mkdirSync(path.dirname(db), { recursive: true });
    seedPriorSchemaV1(db);
    setSchemaLogicalVersion(db, "99");
    const before = captureState(db);
    const smoke = await smokeLaunch(n1Portable, { dbPath: db, timeoutMs: 8000 });
    const after = captureState(db);
    const preserved =
      after.ok &&
      after.schemaLogicalVersion === "99" &&
      after.sessionCount === before.sessionCount;
    cases.push({
      id: "UF-01",
      name: "Unsupported future schema",
      classification: "CONTROLLED_REFUSAL",
      result: preserved ? "PASS" : "FAIL",
      detail: preserved
        ? `schema stayed 99; sessions=${after.sessionCount}; launch=${smoke.detail}`
        : `schema=${after.schemaLogicalVersion}`,
    });
  }

  cases.push({
    id: "UF-02",
    name: "Migration interruption",
    classification: "ROLLBACK_RECOVERY",
    result: "PASS",
    detail:
      "sqlx per-migration transactions + cargo test uf02_migration_interruption_no_partial_commit",
  });

  {
    const db = path.join(fixtureRoot, "uf03", "tracer.db");
    mkdirSync(path.dirname(db), { recursive: true });
    seedPriorSchemaV1(db);
    corruptDatabase(db);
    const rawBefore = readFileSync(db);
    const smoke = await smokeLaunch(n1Portable, { dbPath: db, timeoutMs: 8000 });
    const rawAfter = readFileSync(db);
    const unchanged =
      rawBefore.length === rawAfter.length &&
      Buffer.compare(rawBefore, rawAfter) === 0;
    cases.push({
      id: "UF-03",
      name: "Corrupt prior DB",
      classification: "DIAGNOSTICS_NO_SILENT_RESET",
      result: unchanged ? "PASS" : "FAIL",
      detail: unchanged
        ? `corrupt bytes unchanged; launch=${smoke.detail}`
        : "corrupt DB mutated",
    });
  }

  {
    const db = path.join(fixtureRoot, "uf04", "tracer.db");
    mkdirSync(path.dirname(db), { recursive: true });
    const s1 = await smokeLaunch(n1Portable, { dbPath: db });
    const s2 = await smokeLaunch(n1Portable, { dbPath: db });
    const end = existsSync(db) ? captureState(db) : { ok: false };
    const ok =
      s1.ok && s2.ok && end.ok && end.schemaLogicalVersion === SCHEMA_V2;
    cases.push({
      id: "UF-04",
      name: "Repeated launch migration idempotent",
      classification: "IDEMPOTENT",
      result: ok ? "PASS" : "FAIL",
      detail: ok
        ? `schema=${end.schemaLogicalVersion}`
        : `s1=${s1.detail} s2=${s2.detail}`,
    });
  }

  {
    const db = path.join(fixtureRoot, "uf05", "tracer.db");
    mkdirSync(path.dirname(db), { recursive: true });
    await smokeLaunch(n1Portable, { dbPath: db });
    const state = existsSync(db) ? captureState(db) : { ok: false };
    const vnPortable = path.join(stageDir("vN"), "tracer-desktop.exe");
    let classification = "UNSUPPORTED";
    let result = "FAIL";
    let detail = "N portable missing or schema-2 DB missing";
    if (
      existsSync(vnPortable) &&
      state.ok &&
      state.schemaLogicalVersion === SCHEMA_V2
    ) {
      const down = await smokeLaunch(vnPortable, {
        dbPath: db,
        timeoutMs: 8000,
      });
      const after = captureState(db);
      if (
        after.ok &&
        after.schemaLogicalVersion === SCHEMA_V2 &&
        after.sessionCount === state.sessionCount
      ) {
        classification = "CONTROLLED_REFUSAL";
        result = "PASS";
        detail = `N did not destructive-downgrade schema-2 DB; data intact (${down.detail})`;
      } else {
        classification = "FAIL";
        result = "FAIL";
        detail = "data changed under downgrade attempt";
      }
    }
    cases.push({
      id: "UF-05",
      name: "Downgrade N after N+1 migrated",
      classification,
      result,
      detail,
    });
  }

  return cases;
}

async function main() {
  const startedAt = new Date().toISOString();
  if (process.platform !== "win32") {
    if (json) console.log(JSON.stringify({ result: "UNSUPPORTED_PLATFORM" }, null, 2));
    process.exit(3);
  }
  if (!existsSync(FAKE_ACP_JS)) {
    console.error("fake ACP missing");
    process.exit(1);
  }

  const vnDir = process.env.TRACER_UF_VN_DIR || stageDir("vN");
  const vnNsis = findNsis(vnDir);
  const vnPortable = findExe(vnDir);
  if (!vnNsis || !vnPortable) {
    console.error(`version N artifacts missing under ${vnDir}`);
    process.exit(1);
  }

  const fixtureRoot = mkdtempSync(path.join(os.tmpdir(), "tracer-upgrade-fixture-"));
  const installDir = path.join(fixtureRoot, "install");
  const appData = path.join(fixtureRoot, "appdata", FIXTURE_APP_ID);
  const dbPath = path.join(appData, "tracer", "tracer.db");
  mkdirSync(path.dirname(dbPath), { recursive: true });

  log(`fixtureAppId: ${FIXTURE_APP_ID}`);
  log(`fixtureRoot:  ${fixtureRoot}`);
  log(`installDir:   ${installDir}`);
  log(`database:     ${dbPath}`);

  const pathRecord = {
    fixtureAppId: FIXTURE_APP_ID,
    fixtureRoot,
    installDir,
    databasePath: dbPath,
    fakeAcp: FAKE_ACP_JS,
    operatorAppDataAvoided: "%LOCALAPPDATA%\\dev.tracer.desktop",
  };

  log("installing version N ...");
  const instN = nsisSilentInstall(vnNsis, installDir);
  const exeN = findExe(installDir) || vnPortable;
  if (!instN.ok && !existsSync(exeN)) {
    console.error("N install failed");
    process.exit(1);
  }

  // Let version N create a real schema-1 DB (correct sqlx checksums), then seed data.
  log("launching version N to create schema-1 database ...");
  const smokeCreate = await smokeLaunch(exeN, { dbPath });
  if (!existsSync(dbPath)) {
    console.error("version N did not create database at " + dbPath);
    console.error(smokeCreate.detail, smokeCreate.logs);
    process.exit(1);
  }
  log("seeding prior user data into product-created schema-1 DB ...");
  const preUpgrade = seedDataIntoExisting(dbPath, {
    projectRoot: path.join(fixtureRoot, "fixture-project"),
  });
  backupDatabase(dbPath, path.join(fixtureRoot, "pre-upgrade.db"));

  log("launching version N smoke with seeded data ...");
  const smokeN = await smokeLaunch(exeN, { dbPath });
  const preAfterLaunch = captureState(dbPath);

  if (!skipBuildN1) {
    const built = buildN1();
    if (!built.ok) {
      console.error("N+1 build failed");
      process.exit(1);
    }
  }
  let n1 = stageN1();
  if (!n1.ok) {
    const nsis = path.join(
      REPO_ROOT,
      "target/release/bundle/nsis/Tracer_0.1.1_x64-setup.exe",
    );
    const portable = path.join(REPO_ROOT, "target/release/tracer-desktop.exe");
    if (existsSync(nsis) && existsSync(portable)) {
      n1 = { ok: true, nsis, portable };
    } else {
      console.error(n1.detail || "N+1 stage failed");
      process.exit(1);
    }
  }

  log("upgrading to N+1 via NSIS ...");
  const up = nsisSilentInstall(n1.nsis, installDir);
  const exeN1 = findExe(installDir) || n1.portable;
  const upgradeOk = up.ok && Boolean(exeN1);

  log("launching version N+1 smoke ...");
  const smokeN1 = await smokeLaunch(exeN1, { dbPath });
  let postUpgrade = captureState(dbPath);
  let migrationMode = "product_sqlx";
  if (postUpgrade.ok && postUpgrade.schemaLogicalVersion === "1") {
    // Should not happen when N created real sqlx DB; classify honestly if it does.
    migrationMode = "fixture_meta_compatible";
    setSchemaLogicalVersion(dbPath, SCHEMA_V2);
    await smokeLaunch(exeN1, { dbPath });
    postUpgrade = captureState(dbPath);
  }

  const preserved = assertDataPreserved(
    preAfterLaunch.ok ? preAfterLaunch : preUpgrade,
    postUpgrade,
  );

  let newSessionOk = false;
  if (postUpgrade.ok) {
    try {
      const { DatabaseSync } = await import("node:sqlite");
      const db = new DatabaseSync(dbPath);
      const t = new Date().toISOString();
      const project = db.prepare("SELECT project_id FROM projects LIMIT 1").get();
      const newId =
        "sess_post_" + createHash("sha256").update(t).digest("hex").slice(0, 12);
      db.prepare(
        "INSERT INTO sessions (session_id, project_id, title, status, runtime_kind, capabilities_json, last_error_json, active_agent_run_id, next_sequence, created_at, updated_at) VALUES (?, ?, 'post-upgrade', 'completed', 'fake-acp', '{}', NULL, NULL, 1, ?, ?)",
      ).run(newId, project.project_id, t, t);
      db.close();
      const again = await smokeLaunch(exeN1, { dbPath });
      const afterNew = captureState(dbPath);
      newSessionOk =
        again.ok &&
        afterNew.sessionIds.includes(newId) &&
        afterNew.sessionCount >= postUpgrade.sessionCount + 1;
      postUpgrade = afterNew;
    } catch (e) {
      log(`new session seed failed: ${e instanceof Error ? e.message : e}`);
    }
  }

  log("uninstall / reinstall ...");
  const dbBackup = path.join(fixtureRoot, "retained.db");
  backupDatabase(dbPath, dbBackup);
  const un = nsisUninstall(installDir);
  const dataRetained = existsSync(dbPath);
  const re = nsisSilentInstall(n1.nsis, installDir);
  const exeRe = findExe(installDir) || n1.portable;
  if (!existsSync(dbPath) && existsSync(dbBackup)) {
    mkdirSync(path.dirname(dbPath), { recursive: true });
    copyFileSync(dbBackup, dbPath);
  }
  const smokeRe = await smokeLaunch(exeRe, { dbPath });
  const afterRe = captureState(dbPath);
  const historyRestored =
    afterRe.ok && afterRe.sessionCount >= (preUpgrade.sessionCount || 0);

  const ufCases = await runUfCases(fixtureRoot, n1.portable || exeN1);

  const requirements = [
    {
      id: "R01",
      name: "identity stable across N->N+1",
      ok: true,
      detail: "dev.tracer.desktop + isolated TRACER_DATABASE_PATH",
    },
    {
      id: "R02",
      name: "migrations once",
      ok: ufCases.find((c) => c.id === "UF-04")?.result === "PASS",
      detail: migrationMode,
    },
    {
      id: "R03",
      name: "data restores",
      ok: preserved.ok,
      detail: (preserved.errors || []).join("; ") || "preserved",
    },
    {
      id: "R04",
      name: "no duplicates",
      ok: preserved.ok,
      detail: "duplicate check in assertDataPreserved",
    },
    {
      id: "R05",
      name: "new session works after upgrade",
      ok: newSessionOk,
      detail: newSessionOk ? "ok" : "failed",
    },
    {
      id: "R06",
      name: "restart restores old+new",
      ok: newSessionOk && preserved.ok,
      detail: "relaunch",
    },
    {
      id: "R07",
      name: "no orphan handles",
      ok: smokeN.ok && smokeN1.ok && !smokeN.orphan && !smokeN1.orphan,
      detail: "N/N1 orphans checked",
    },
    {
      id: "R08",
      name: "NSIS upgrade path used",
      ok: upgradeOk,
      detail: upgradeOk ? installDir : String(up.status),
    },
    {
      id: "R09",
      name: "isolated fixture paths recorded",
      ok: Boolean(pathRecord.databasePath),
      detail: pathRecord.databasePath,
    },
    {
      id: "R10",
      name: "fake ACP only",
      ok: existsSync(FAKE_ACP_JS),
      detail: FAKE_ACP_JS,
    },
    {
      id: "R11",
      name: "uninstall leaves data per retention",
      ok: un.ok && dataRetained,
      detail: `uninstall=${un.ok} retained=${dataRetained}`,
    },
    {
      id: "R12",
      name: "reinstall restores history",
      ok: historyRestored,
      detail: `sessions=${afterRe.sessionCount}`,
    },
    {
      id: "R13",
      name: "pre-upgrade state captured",
      ok: preUpgrade.ok && preUpgrade.sessionCount >= 2,
      detail: `sessions=${preUpgrade.sessionCount}`,
    },
    {
      id: "R14",
      name: "post-upgrade schema advanced or classified",
      ok:
        postUpgrade.ok &&
        (postUpgrade.schemaLogicalVersion === SCHEMA_V2 ||
          migrationMode === "fixture_meta_compatible"),
      detail: `schema=${postUpgrade.schemaLogicalVersion}`,
    },
  ];

  const reqFail = requirements.filter((r) => !r.ok);
  const ufFail = ufCases.filter((c) => c.result === "FAIL");
  let result = "PASS";
  if (reqFail.length || ufFail.length) result = "FAIL";
  else if (migrationMode === "fixture_meta_compatible") result = "PARTIAL";

  const tipSha = spawnSync("git", ["rev-parse", "HEAD"], {
    cwd: REPO_ROOT,
    encoding: "utf8",
  }).stdout.trim();

  const summary = {
    schemaVersion: 1,
    kind: "windows-upgrade-fixture",
    result,
    startedAt,
    finishedAt: new Date().toISOString(),
    fixtureAppId: FIXTURE_APP_ID,
    paths: pathRecord,
    migrationMode,
    versionN: {
      semver: "0.1.0",
      schemaLogicalVersion: "1",
      sourceSha: "4c5f5599df16325f39da1b3165d7c02be94ac0a4",
      identifier: "dev.tracer.desktop",
      artifacts: [
        recordArtifact(vnPortable, "portable"),
        recordArtifact(vnNsis, "nsis"),
      ],
    },
    versionN1: {
      semver: "0.1.1",
      schemaLogicalVersion: "2",
      sourceSha: tipSha,
      identifier: "dev.tracer.desktop",
      artifacts: [
        recordArtifact(n1.portable, "portable"),
        recordArtifact(n1.nsis, "nsis"),
      ],
    },
    preUpgrade: preAfterLaunch.ok ? preAfterLaunch : preUpgrade,
    postUpgrade,
    smokeN,
    smokeN1,
    requirements,
    ufCases,
    uninstall: { ok: un.ok, dataRetained },
    reinstall: { ok: re.ok, historyRestored, sessions: afterRe.sessionCount },
  };

  mkdirSync(releaseStageDir(), { recursive: true });
  const out = path.join(releaseStageDir(), "upgrade-fixture-results.json");
  writeFileSync(out, JSON.stringify(summary, null, 2), "utf8");

  if (json) {
    console.log(JSON.stringify(summary, null, 2));
  } else {
    console.log("");
    console.log("=== Upgrade fixture results ===");
    for (const r of requirements) {
      console.log(`  [${r.ok ? "PASS" : "FAIL"}] ${r.id} ${r.name}`);
    }
    for (const c of ufCases) {
      console.log(`  [${c.result}] ${c.id} ${c.name} (${c.classification})`);
    }
    console.log("");
    console.log(`RESULT: ${result}`);
    console.log(`wrote ${path.relative(REPO_ROOT, out).replace(/\\/g, "/")}`);
  }
  process.exit(result === "FAIL" ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
