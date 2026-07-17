import type { ReactElement } from "react";
import { Banner, Button } from "@tracer/ui";
import type { GlobalBannerKind } from "../shared/store/mockStore";

interface Props {
  banner: GlobalBannerKind;
  onDismiss: () => void;
}

export function GlobalStatusRegion({ banner, onDismiss }: Props): ReactElement | null {
  if (banner === "none") return null;

  if (banner === "runtime_missing") {
    return (
      <Banner
        severity="error"
        title="Agent runtime not found"
        live="assertive"
        actions={
          <Button variant="ghost" onClick={onDismiss}>
            Dismiss
          </Button>
        }
      >
        <p>
          Install or configure the ACP runtime. Configure path is a control-plane concern (W1-F); this
          is a shell placeholder.
        </p>
      </Banner>
    );
  }

  if (banner === "storage_error") {
    return (
      <Banner
        severity="warning"
        title="Could not save session data"
        live="polite"
        actions={
          <Button variant="ghost" onClick={onDismiss}>
            Dismiss
          </Button>
        }
      >
        <p>On-screen history may not reload after restart. Tracer must not claim persistence succeeded.</p>
      </Banner>
    );
  }

  return (
    <Banner
      severity="error"
      title="Tracer control plane is not responding"
      live="assertive"
      actions={
        <Button variant="ghost" onClick={onDismiss}>
          Dismiss
        </Button>
      }
    >
      <p>Invoke failures against the local control plane. Retry later or check W1-F wiring.</p>
    </Banner>
  );
}
