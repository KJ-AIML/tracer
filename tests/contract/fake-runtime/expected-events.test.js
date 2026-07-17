import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  findRepoRoot,
  loadCatalog,
  listStandardCiScenarioIds,
  loadExpectedEvents,
  assertNormativeNamesOnly,
  collectConstraintTypes,
  FORBIDDEN_PRODUCT_TYPE_ALIASES,
  assertNotLiveParityClaim,
} from "../../../packages/test-fixtures/src/index.js";

describe("expected-events packs", () => {
  const root = findRepoRoot();
  const catalog = loadCatalog(root);
  const ids = listStandardCiScenarioIds(catalog);

  it("every standardCi scenario has a loadable pack", () => {
    for (const id of ids) {
      const pack = loadExpectedEvents(id, root);
      assert.equal(pack.scenarioId, id);
      assert.ok(pack.evidence);
    }
  });

  it("packs use normative W0-A names only (no W0-B aliases as types)", () => {
    for (const id of ids) {
      const pack = loadExpectedEvents(id, root);
      assert.doesNotThrow(() => assertNormativeNamesOnly(pack), id);
      const types = collectConstraintTypes(pack);
      for (const t of types) {
        assert.ok(
          !FORBIDDEN_PRODUCT_TYPE_ALIASES.includes(t),
          `${id} uses forbidden alias ${t}`,
        );
      }
    }
  });

  it("refuses live-parity claim for fake/synthetic evidence", () => {
    assert.throws(() => assertNotLiveParityClaim("fake-runtime", true));
    assert.throws(() => assertNotLiveParityClaim("synthetic", true));
    assert.doesNotThrow(() => assertNotLiveParityClaim("live-authenticated", true));
    assert.doesNotThrow(() => assertNotLiveParityClaim("fake-runtime", false));
  });

  it("auth_required pack forbids session.ready", () => {
    const pack = loadExpectedEvents("auth_required_session_new", root);
    assert.ok(pack.processVsSessionGates?.forbidSessionReady);
    assert.ok(pack.forbiddenTypes.includes("session.ready"));
    assert.ok(pack.commandError?.mustFail);
  });

  it("happy_prompt_stream requires process ready and message activity", () => {
    const pack = loadExpectedEvents("happy_prompt_stream", root);
    const types = collectConstraintTypes(pack);
    assert.ok(types.includes("runtime.process.ready"));
    assert.ok(
      types.includes("agent.message.delta") || types.includes("agent.message.completed"),
    );
  });
});
