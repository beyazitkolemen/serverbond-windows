import type { ReactNode } from "react";

/**
 * Placeholder for a list or pane with nothing to show yet: an icon, a short
 * title, one explanatory paragraph and the actions that fill it.
 */
export default function EmptyState({
  icon,
  title,
  children,
  actions,
  compact,
}: {
  icon?: ReactNode;
  title: string;
  children?: ReactNode;
  actions?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div className={`empty-state${compact ? " compact" : ""}`}>
      {icon}
      <div>
        <h3>{title}</h3>
        {children ? <p>{children}</p> : null}
        {actions ? <div className="empty-actions">{actions}</div> : null}
      </div>
    </div>
  );
}
