import type { ReactElement } from "react";
import { Banner, Button } from "@tracer/ui";
import type { GlobalBannerKind } from "../shared/store/snapshotStore";

interface Props {
  banner: GlobalBannerKind;
  heliSummary?: string;
  onDismiss: () => void;
}

export function GlobalStatusRegion({ banner, heliSummary, onDismiss }: Props): ReactElement | null {
  if (banner === "none") return null;

  if (banner === "runtime_missing") {
    return (
      <div data-testid="tracer-banner-runtime-missing">
        <Banner
          severity="error"
          title="Agent runtime not found"
          live="assertive"
          actions={
            <Button variant="ghost" data-testid="tracer-banner-dismiss" onClick={onDismiss}>
              Dismiss
            </Button>
          }
        >
          <p>
            Install or configure the ACP runtime. Path configuration is a control-plane concern; the
            shell only displays the typed failure state.
          </p>
        </Banner>
      </div>
    );
  }

  if (banner === "storage_error") {
    return (
      <div data-testid="tracer-banner-storage-error">
        <Banner
          severity="warning"
          title="Could not save session data"
          live="polite"
          actions={
            <Button variant="ghost" data-testid="tracer-banner-dismiss" onClick={onDismiss}>
              Dismiss
            </Button>
          }
        >
          <p>
            On-screen history may not reload after restart. Tracer must not claim persistence
            succeeded.
          </p>
        </Banner>
      </div>
    );
  }

  if (banner === "heli_unavailable") {
    return (
      <div data-testid="tracer-banner-heli-unavailable">
        <Banner
          severity="info"
          title="Heli workspace unavailable"
          live="polite"
          actions={
            <Button variant="ghost" data-testid="tracer-banner-dismiss" onClick={onDismiss}>
              Dismiss
            </Button>
          }
        >
          <p>
            {heliSummary ?? "Heli was not detected."} Tracer continues without Heli coordination —
            this is not a fatal error.
          </p>
        </Banner>
      </div>
    );
  }

  return (
    <div data-testid="tracer-banner-control-plane-down">
      <Banner
        severity="error"
        title="Tracer control plane is not responding"
        live="assertive"
        actions={
          <Button variant="ghost" data-testid="tracer-banner-dismiss" onClick={onDismiss}>
            Dismiss
          </Button>
        }
      >
        <p>
          Invoke failures against the local control plane. Retry later or check desktop command
          wiring. Fail-closed: no silent mock downgrade.
        </p>
      </Banner>
    </div>
  );
}