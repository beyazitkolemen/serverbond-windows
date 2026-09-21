import type { ReactNode } from "react";

export type StatusTone = "running" | "stopped" | "issue" | "warning";

/**
 * The single pill used everywhere a service, project, worker or release
 * reports its state, so colours and spacing stay identical across pages.
 */
export default function StatusBadge({
  tone,
  children,
  title,
  className,
}: {
  tone: StatusTone;
  children: ReactNode;
  title?: string;
  className?: string;
}) {
  const dot =
    tone === "running"
      ? "green"
      : tone === "issue"
        ? "red"
        : tone === "warning"
          ? "amber"
          : "";
  return (
    <span
      className={`service-status ${tone === "stopped" ? "" : tone}${className ? ` ${className}` : ""}`}
      title={title}
    >
      <span className={`status-dot ${dot}`} />
      {children}
    </span>
  );
}
