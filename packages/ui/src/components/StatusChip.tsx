import type { ReactElement } from "react";
import { Icon, type IconName } from "../icons";
import { getSessionStatusPresentation } from "../statusCatalog";
import type { SessionStatus } from "../types";

export interface StatusChipProps {
  status: SessionStatus;
  /** Optional one-line sublabel (reason / lastError). */
  sublabel?: string;
  className?: string;
}

const ICON_MAP: Record<string, IconName> = {
  spinner: "spinner",
  check: "check",
  "check-circle": "check-circle",
  activity: "activity",
  shield: "shield",
  error: "error",
  unlink: "unlink",
  stop: "stop",
  plus: "plus",
  plug: "plug",
};

/**
 * Session StatusChip — text + icon (STATE_MATRIX A1/A2). Never color-only.
 */
export function StatusChip({ status, sublabel, className }: StatusChipProps): ReactElement {
  const presentation = getSessionStatusPresentation(status);
  const iconName = ICON_MAP[presentation.iconHint] ?? "info";
  const classes = [
    "tracer-status-chip",
    `tracer-status-chip--${presentation.colorRole}`,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <span
      className={classes}
      data-status={status}
      data-color-role={presentation.colorRole}
      role="status"
      aria-label={sublabel ? `${presentation.label}. ${sublabel}` : presentation.label}
    >
      <Icon name={iconName} />
      <span className="tracer-status-chip__text">{presentation.label}</span>
      {sublabel ? (
        <span className="tracer-status-chip__sub" style={{ fontWeight: 400, opacity: 0.85 }}>
          — {sublabel}
        </span>
      ) : null}
    </span>
  );
}
