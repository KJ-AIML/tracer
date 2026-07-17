# @tracer/test-fixtures (W1-G)

Helpers for loading:

- `tests/specifications/scenarios/catalog.yaml`
- `tests/specifications/expected-events/*.json`
- ACP fixture provenance rules
- Paths to the fake ACP runtime binary

Zero runtime dependencies. Node ≥ 18.

## Usage

```js
import {
  findRepoRoot,
  loadCatalog,
  listStandardCiScenarioIds,
  loadExpectedEvents,
  assertNormativeNamesOnly,
  fakeRuntimeBin,
} from "@tracer/test-fixtures";

const root = findRepoRoot();
const catalog = loadCatalog(root);
const ids = listStandardCiScenarioIds(catalog);
const pack = loadExpectedEvents("happy_prompt_stream", root);
assertNormativeNamesOnly(pack);
const bin = fakeRuntimeBin(root);
```

## Provenance

See `src/provenance.js` and `docs/testing/TEST_STRATEGY.md` §3.2.

- `fake-runtime` / `synthetic` evidence never counts as live multi-turn Grok parity.
- Live-only catalog scenarios stay out of standard CI.
