import { call } from "../api";
import LogViewer from "./LogViewer";

const sources = [
  { id: "f4box", label: "F4Box" },
  { id: "php", label: "PHP" },
  { id: "mysql", label: "MySQL" },
  { id: "caddy", label: "Caddy" },
  { id: "composer", label: "Composer" },
  { id: "postgres", label: "PostgreSQL" },
  { id: "redis", label: "Redis" },
];

export function LogPreview({
  logs,
  onOpen,
}: {
  logs: string[];
  onOpen?: () => void;
}) {
  return (
    <section aria-labelledby="logs-heading">
      <div className="section-heading">
        <h2 id="logs-heading">Son kayıtlar</h2>
        {onOpen ? (
          <button type="button" className="section-link" onClick={onOpen}>
            Tüm günlükler
          </button>
        ) : null}
      </div>
      <pre
        className="console preview-console"
        aria-label="Son günlük kayıtları"
      >
        {logs.slice(-3).join("\n") || "Henüz kayıt yok."}
      </pre>
    </section>
  );
}

export default function Logs() {
  return (
    <section>
      <LogViewer
        tall
        sources={sources}
        load={(id) => call<string>("read_log", { id })}
        label="Sistem günlükleri"
      />
    </section>
  );
}
