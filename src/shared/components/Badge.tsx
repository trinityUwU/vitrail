import type { ReactNode, ReactElement } from "react";

interface BadgeProps {
  variant: "ok" | "meta" | "attrib" | "unknown" | "danger" | "warn";
  children: ReactNode;
  withDot?: boolean;
}

export function Badge({ variant, children, withDot = false }: BadgeProps): ReactElement {
  return (
    <span className={`badge badge-${variant}`}>
      {withDot && <span className="badge-dot" />}
      {children}
    </span>
  );
}
