import { Download, ExternalLink, Play, Square } from "lucide-react";

export type ConsoleKind =
  "running" | "stopped" | "missing" | "repair" | "ready" | "off";

export type ConsoleAction = {
  id: string;
  label: string;
  tone: "primary" | "secondary";
  icon: "play" | "stop" | "install" | "open";
  disabled?: boolean;
  title?: string;
  onClick: () => void;
};

const icons = {
  play: Play,
  stop: Square,
  install: Download,
  open: ExternalLink,
};

export default function ServiceConsole({
  kind,
  title,
  detail,
  issue,
  actions,
}: {
  kind: ConsoleKind;
  title: string;
  detail: string;
  issue?: string | null;
  actions: ConsoleAction[];
}) {
  const live = kind === "running" || kind === "ready";
  return (
    <section
      className={`service-console${live ? " live" : ""}${kind === "repair" || issue ? " attention" : ""}`}
      aria-label="Hizmet denetimi"
    >
      <div className="service-console-status">
        <span
          className={`service-console-dot ${live ? "on" : kind === "repair" ? "warn" : ""}`}
          aria-hidden="true"
        />
        <div className="service-console-copy">
          <strong>{title}</strong>
          <p>{detail}</p>
        </div>
      </div>
      {actions.length ? (
        <div className="service-console-actions">
          {actions.map((action) => {
            const Icon = icons[action.icon];
            return (
              <button
                key={action.id}
                type="button"
                className={`button ${action.tone} service-console-button`}
                disabled={action.disabled}
                title={action.title}
                onClick={action.onClick}
              >
                <Icon size={16} />
                {action.label}
              </button>
            );
          })}
        </div>
      ) : null}
      {issue ? (
        <p role="alert" className="field-error service-console-issue">
          {issue}
        </p>
      ) : null}
    </section>
  );
}
