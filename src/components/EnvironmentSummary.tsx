import { Activity } from "lucide-react";
import type { Page, Snapshot } from "../types";

export default function EnvironmentSummary({
  state,
  onPage,
}: {
  state: Snapshot;
  onPage: (page: Page) => void;
}) {
  const services = state.packages.filter((p) =>
    ["php", "mysql", "caddy"].includes(p.id),
  );
  const active = services.filter((p) => p.running).length;
  const ready = services.length === 3 && active === 3;
  const issue = services.some((p) => p.issue);
  const down = services.filter((p) => !p.running).map((p) => p.name);
  const up = services.filter((p) => p.running).map((p) => p.name);
  const status = issue
    ? "Ortamı kontrol edin"
    : ready
      ? "Ortam çalışıyor"
      : active
        ? "Servisler kısmen açık"
        : "Servisler kapalı";
  const detail = issue
    ? services
        .filter((p) => p.issue)
        .map((p) => `${p.name}: ${p.issue}`)
        .join(" ")
    : ready
      ? "PHP, MySQL ve web sunucusu bağlantıya hazır."
      : active
        ? `${up.join(", ")} çalışıyor. ${down.join(", ")} kapalı.`
        : "PHP, MySQL ve Caddy henüz çalışmıyor.";
  return (
    <section
      className={`environment-summary ${ready && !issue ? "is-running" : ""}`}
      aria-label="Ortam özeti"
    >
      <div className="environment-summary-copy">
        <div className="summary-symbol">
          <Activity size={24} strokeWidth={1.6} />
        </div>
        <div>
          <span className="summary-kicker">ORTAM DURUMU</span>
          <h2>{status}</h2>
          <p>{detail}</p>
        </div>
      </div>
      <dl className="environment-metrics">
        <div>
          <dt>Aktif servis</dt>
          <dd>
            <button
              type="button"
              className="metric-link"
              onClick={() => onPage("packages")}
            >
              {active}
              <span> / 3</span>
            </button>
          </dd>
        </div>
        <div>
          <dt>PHP sürümü</dt>
          <dd>
            <button
              type="button"
              className="metric-link"
              onClick={() => onPage("packages")}
            >
              {state.packages.find((p) => p.id === "php")?.version ?? "—"}
            </button>
          </dd>
        </div>
        <div>
          <dt>Proje</dt>
          <dd>
            <button
              type="button"
              className="metric-link"
              onClick={() => onPage("projects")}
            >
              {state.projects.length}
            </button>
          </dd>
        </div>
      </dl>
    </section>
  );
}
