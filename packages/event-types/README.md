# @tracer/event-types

TypeScript types and validators for **Tracer Event Protocol v1** (W1-B).

## Usage

```ts
import {
  parseEnvelope,
  validateSessionEventStream,
  KNOWN_EVENT_TYPES,
} from "@tracer/event-types";
```

## Develop

```bash
npm install
npm test
```

Standalone package until root `pnpm-workspace.yaml` includes `packages/*`
(see `docs/modules/w1-b/SHARED_MANIFEST_REQUESTS.md`).