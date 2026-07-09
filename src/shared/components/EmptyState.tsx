import type { LucideIcon } from "lucide-react";
import type { ReactNode, ReactElement } from "react";

interface EmptyStateProps {
  icon: LucideIcon;
  message: string;
  action?: ReactNode;
}

export function EmptyState({ icon: Icon, message, action }: EmptyStateProps): ReactElement {
  return (
    <div className="empty-state">
      <Icon />
      <p>{message}</p>
      {action}
    </div>
  );
}
