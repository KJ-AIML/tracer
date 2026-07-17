import type { ButtonHTMLAttributes, ReactElement } from "react";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "primary" | "danger" | "ghost";
  /** Visible reason when disabled (A4) — rendered as title/aria-description. */
  disabledReason?: string;
}

export function Button({
  variant = "default",
  disabledReason,
  disabled,
  className,
  children,
  title,
  ...rest
}: ButtonProps): ReactElement {
  const classes = ["tracer-button", `tracer-button--${variant}`, className]
    .filter(Boolean)
    .join(" ");
  const isDisabled = Boolean(disabled);
  const tip = isDisabled && disabledReason ? disabledReason : title;

  return (
    <button
      type="button"
      className={classes}
      disabled={isDisabled}
      title={tip}
      aria-disabled={isDisabled || undefined}
      aria-description={isDisabled && disabledReason ? disabledReason : undefined}
      {...rest}
    >
      {children}
    </button>
  );
}
