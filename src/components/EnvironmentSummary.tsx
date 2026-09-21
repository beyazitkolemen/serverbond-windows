import { Activity } from "lucide-react";
import { CORE_COMPONENTS, Page, type Page as AppPage } from "../domain";
import type { Snapshot } from "../types";

export default function EnvironmentSummary({
  state,
  onPage,
}: {
  state: Snapshot;
  onPage: (page: AppPage) => void;
}) {
  const services = state.packages.filter((p) =>
    (CORE_COMPONENTS as readonly string[]).includes(p.id),
  );
  const active = services.filter((p) => p.running).length;
  const ready = services.length === 3 && active === 3;
  const issue = services.some((p) => p.issue);
  const status = issue
    ? "Sunucuyu kontrol edin"
    : ready
      ? "Sunucu çalışıyor"
      : active
        ? "Servisler kısmen çalışıyor"
        : "Servisler kapalı";
  return (
    <section
      className={`environment-summary ${ready && !issue ? "is-running" : ""}`}
      aria-label="Sunucu özeti"
    >
      <div className="environment-summary-copy">
        <div className="summary-symbol">
          <Activity size={24} strokeWidth={1.6} />
        </div>
        <div>
          <h2>{status}</h2>
        </div>
      </div>
      <dl className="environment-metrics">
        <div>
          <dt>Aktif servis</dt>
          <dd>
            <button
              type="button"
              className="metric-link"
              onClick={() => onPage(Page.Packages)}
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
              onClick={() => onPage(Page.Packages)}
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
              onClick={() => onPage(Page.Projects)}
            >
              {state.projects.length}
            </button>
          </dd>
        </div>
      </dl>
    </section>
  );
}
