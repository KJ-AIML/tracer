import type { ReactElement } from "react";
import { Button, PresentationContainer, RuntimePill, StatusChip, type PresentationKind } from "@tracer/ui";

const KINDS: PresentationKind[] = [
  "empty",
  "loading",
  "running",
  "approval",
  "failed",
  "disconnected",
  "completed",
  "cancelled",
];

/** Dev/smoke gallery of presentation containers + status chips. */
export function PresentationGallery(): ReactElement {
  return (
    <div className="layout-stack">
      <h1 className="panel__title">Presentation containers</h1>
      <p className="list__meta">
        STATE_MATRIX §12 shorthand states. Each surface includes text (and icon/chip) — never color-only.
      </p>

      <section className="panel">
        <h2 className="panel__title">StatusChip catalog</h2>
        <div className="layout-row">
          {(
            [
              "creating",
              "starting_runtime",
              "ready",
              "running",
              "awaiting_approval",
              "cancelling",
              "completed",
              "failed",
              "disconnected",
              "stopped",
            ] as const
          ).map((s) => (
            <StatusChip key={s} status={s} />
          ))}
        </div>
      </section>

      <section className="panel">
        <h2 className="panel__title">RuntimePill catalog</h2>
        <div className="layout-row">
          {(
            [
              "not_started",
              "starting",
              "ready",
              "sign_in_required",
              "stopped",
              "crashed",
              "unavailable",
            ] as const
          ).map((o) => (
            <RuntimePill key={o} observation={o} />
          ))}
        </div>
      </section>

      {KINDS.map((kind) => (
        <PresentationContainer
          key={kind}
          kind={kind}
          actions={
            kind === "approval" ? (
              <>
                <Button variant="primary">Allow</Button>
                <Button variant="danger">Deny</Button>
              </>
            ) : kind === "empty" ? (
              <Button variant="primary">Primary CTA</Button>
            ) : undefined
          }
        />
      ))}
    </div>
  );
}
