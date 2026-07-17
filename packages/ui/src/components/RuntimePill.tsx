import type { ReactElement } from "react";
import { Icon, type IconName } from "../icons";
import { getRuntimeObservationPresentation } from "../statusCatalog";
import type { RuntimeObservation } from "../types";

export interface RuntimePillProps {
  observation: RuntimeObservation;
  className?: string;
}

const ICON_MAP: Record<string, IconName> = {
  spinner: "spinner",
  check: "check",
  shield: "shield",
  error: "error",
  stop: "stop",
  plug: "plug",
};

/**
 * Runtime process health pill — distinct from session StatusChip.
 * Text + icon always present (accessibility A1).
 */
export function RuntimePill({ observation, className }: RuntimePillProps): ReactElement {
  const presentation = getRuntimeObservationPresentation(observation);
  const iconName = ICON_MAP[presentation.iconHint] ?? "plug";
  const classes = ["tracer-runtime-pill", className].filter(Boolean).join(" ");

  return (
    <span
      className={classes}
      data-runtime={observation}
      data-color-role={presentation.colorRole}
      role="status"
      aria-label={presentation.label}
    >
      <Icon name={iconName} />
      <span className="tracer-runtime-pill__label">{presentation.label}</span>
    </span>
  );
}
