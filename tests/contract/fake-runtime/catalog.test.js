import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  findRepoRoot,
  loadCatalog,
  listStandardCiScenarioIds,
  getScenarioEntry,
  ACP_FIXTURE_PROVENANCE,
} from "../../../packages/test-fixtures/src/index.js";
import {
  IMPLEMENTED_SCENARIOS,
  LIVE_ONLY_SCENARIOS,
  listScenarioIds,
} from "../../../tools/fake-acp-runtime/src/index.js";

describe("scenario catalog integration", () => {
  const root = findRepoRoot();
  const catalog = loadCatalog(root);

  it("loads schemaVersion 1 with scenarios", () => {
    assert.equal(catalog.schemaVersion, 1);
    assert.ok(catalog.scenarios.length >= 15);
  });

  it("standardCi scenarios have expected-events and evidence", () => {
    const ids = listStandardCiScenarioIds(catalog);
    assert.ok(ids.includes("happy_prompt_stream"));
    assert.ok(ids.includes("auth_required_session_new"));
    for (const id of ids) {
      const entry = getScenarioEntry(catalog, id);
      assert.ok(entry, id);
      assert.equal(entry.standardCi, true);
      assert.equal(entry.mayConsumeProviderUsage, false);
      assert.ok(entry.expectedEvents, `${id} expectedEvents`);
      assert.match(entry.expectedEvents, /expected-events/);
    }
  });

  it("fake implements all standardCi catalog ids", () => {
    const standard = listStandardCiScenarioIds(catalog);
    for (const id of standard) {
      assert.ok(
        IMPLEMENTED_SCENARIOS.includes(id),
        `fake missing scenario ${id}`,
      );
    }
    assert.deepEqual(listScenarioIds().sort(), [...IMPLEMENTED_SCENARIOS].sort());
  });

  it("live-only scenarios are not implemented by fake", () => {
    for (const id of LIVE_ONLY_SCENARIOS) {
      const entry = getScenarioEntry(catalog, id);
      assert.ok(entry, id);
      assert.equal(entry.standardCi, false);
      assert.ok(!IMPLEMENTED_SCENARIOS.includes(id));
    }
  });

  it("defaults declare NDJSON transport", () => {
    assert.equal(catalog.defaults.transport, "ndjson-jsonrpc-2.0");
    assert.equal(catalog.defaults.runtimeKind, "acp-stdio");
    assert.equal(catalog.defaults.standardCi, true);
  });

  it("Gate 0 ACP fixture provenance table is complete", () => {
    assert.equal(ACP_FIXTURE_PROVENANCE["session-prompt-stream.jsonl"], "synthetic");
    assert.equal(ACP_FIXTURE_PROVENANCE["session-new-auth-required.json"], "live-scrubbed");
    assert.equal(ACP_FIXTURE_PROVENANCE["initialize-response.json"], "live-scrubbed");
  });
});
