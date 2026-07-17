import type { ReactElement, ReactNode } from "react";
import { Icon, type IconName } from "../icons";

export type BannerSeverity = "error" | "warning" | "info" | "success";

export interface BannerProps {
  severity: BannerSeverity;
  title: string;
  children?: ReactNode;
  actions?: ReactNode;
  /** Live region politeness; use assertive for disconnect/approval-blocking. */
  live?: "off" | "polite" | "assertive";
  className?: string;
}

const SEVERITY_ICON: Record<BannerSeverity, IconName> = {
  error: "error",
  warning: "warning",
  info: "info",
  success: "check-circle",
};

/**
 * Accessible banner: icon + title text always (never color alone).
 */
export function Banner({
  severity,
  title,
  children,
  actions,
  live = "polite",
  className,
}: BannerProps): ReactElement {
  const classes = ["tracer-banner", `tracer-banner--${severity}`, className]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      className={classes}
      role={severity === "error" || severity === "warning" ? "alert" : "status"}
      aria-live={live === "off" ? undefined : live}
      data-severity={severity}
    >
      <Icon name={SEVERITY_ICON[severity]} title={severity} />
      <div className="tracer-banner__content">
        <p className="tracer-banner__title">{title}</p>
        {children ? <div className="tracer-banner__body">{children}</div> : null}
        {actions ? <div className="tracer-banner__actions">{actions}</div> : null}
      </div>
    </div>
  );
}
