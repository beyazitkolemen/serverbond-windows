import { Activity, ArrowUpRight } from "lucide-react";
import type { Snapshot } from "../types";

export default function EnvironmentSummary({ state }: { state: Snapshot }) {
  const services = state.packages.filter((p) =>
    ["php", "mysql", "caddy"].includes(p.id),
  );
  const active = services.filter((p) => p.running).length;
  const ready = services.length === 3 && active === 3;
  const issue = services.some((p) => p.issue);
  const status = issue
    ? "Ortamı kontrol edin"
    : ready
      ? "Ortam çalışıyor"
      : active
        ? "Servisler kısmen açık"
        : "Servisler kapalı";
  return (
    <section
      className={`environment-summary ${ready && !issue ? "is-running" : ""}`}
      aria-label="Ortam özeti"
    >
      <div className="environment-summary-copy">
        <div className="summary-symbol">
          <Activity size={25} strokeWidth={1.6} />
        </div>
        <div>
          <span className="summary-kicker">ORTAM DURUMU</span>
          <h2>{status}</h2>
          <p>
            {issue
              ? "Bileşenlerdeki uyarıları inceleyin."
              : ready
                ? "PHP, MySQL ve web sunucusu bağlantıya hazır."
                : "PHP, MySQL ve Caddy henüz çalışmıyor."}
          </p>
        </div>
      </div>
      <dl className="environment-metrics">
        <div>
          <dt>Aktif servis</dt>
          <dd>
            {active}
            <span> / 3</span>
          </dd>
        </div>
        <div>
          <dt>PHP sürümü</dt>
          <dd>{state.packages.find((p) => p.id === "php")?.version ?? "—"}</dd>
        </div>
        <div>
          <dt>Proje</dt>
          <dd>
            {state.projects.length}
            <ArrowUpRight size={17} aria-hidden="true" />
          </dd>
        </div>
      </dl>
    </section>
  );
}
