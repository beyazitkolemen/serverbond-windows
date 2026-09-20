import { useEffect, useState } from "react";
import {
  Archive,
  Database,
  ExternalLink,
  Folder,
  Terminal,
  Trash2,
} from "lucide-react";
import { call } from "../api";
import type { PackageStatus, Project, Run } from "../types";
import { databaseName, projectUrl } from "../version";
import { QueueSection, ScheduleSection, useProjectJobs } from "./ProjectJobs";
import ProjectLogs from "./ProjectLogs";
import ReleasePane, { SummaryGit } from "./ProjectRelease";
import ProjectEnv from "./ProjectEnv";

const tabs = [
  { id: "summary", label: "Özet" },
  { id: "env", label: "Ortam" },
  { id: "schedule", label: "Zamanlama" },
  { id: "queue", label: "Kuyruklar" },
  { id: "release", label: "Sürüm" },
  { id: "logs", label: "Günlükler" },
  { id: "database", label: "Veritabanı" },
] as const;

type Tab = (typeof tabs)[number]["id"];

export default function ProjectDetail({
  project,
  busy,
  run,
  webPort,
  https,
  httpsPort,
  mysqlRunning,
  webRunning,
  phpVersions,
  anyRunning,
  onRemove,
  onRestore,
}: {
  project: Project;
  busy: boolean;
  run: Run;
  webPort: number;
  https: boolean;
  httpsPort: number;
  mysqlRunning: boolean;
  webRunning: boolean;
  phpVersions: PackageStatus[];
  anyRunning: boolean;
  onRemove: () => void;
  onRestore: () => void;
}) {
  const [tab, setTab] = useState<Tab>("summary");
  const jobs = useProjectJobs(project);
  useEffect(() => {
    setTab("summary");
  }, [project.id]);
  const url = projectUrl(project.host, webPort, https, httpsPort);
  const runningWorkers = jobs.runningWorkers;
  return (
    <article className="project-detail" aria-labelledby="project-detail-title">
      <header className="project-detail-head">
        <div className="project-detail-copy">
          <h3 id="project-detail-title">{project.name}</h3>
          <p className="project-url">{url}</p>
        </div>
        <div className="project-actions">
          <span
            className={`service-status ${project.running ? "running" : ""}`}
          >
            <span className={`status-dot ${project.running ? "green" : ""}`} />
            {project.running ? `PHP · ${project.phpPort}` : "PHP kapalı"}
          </span>
          <button
            className="button secondary small"
            disabled={busy || !webRunning || !project.running}
            title={
              webRunning
                ? "Projeyi tarayıcıda aç"
                : "Önce web sunucusunu başlatın"
            }
            onClick={() =>
              void run("Proje açılıyor…", () =>
                call("open_project", { id: project.id }),
              )
            }
          >
            <ExternalLink size={16} />
            Aç
          </button>
          <button
            className="button secondary small"
            disabled={busy}
            aria-label={`${project.name} terminalini aç`}
            onClick={() =>
              void run("Proje terminali açılıyor…", () =>
                call("open_project_terminal", { id: project.id }),
              )
            }
          >
            <Terminal size={16} /> Terminal
          </button>
        </div>
      </header>
      <nav className="project-detail-tabs" aria-label="Proje bölümleri">
        {tabs.map((item) => (
          <button
            type="button"
            key={item.id}
            aria-current={tab === item.id ? "page" : undefined}
            onClick={() => setTab(item.id)}
          >
            {item.label}
            {item.id === "schedule" && project.scheduleRunning ? (
              <i className="tab-dot" aria-hidden="true" />
            ) : null}
            {item.id === "queue" && runningWorkers ? (
              <i className="tab-dot" aria-hidden="true" />
            ) : null}
          </button>
        ))}
      </nav>
      {tab === "env" ? (
        <ProjectEnv project={project} busy={busy} run={run} />
      ) : null}
      {tab === "summary" ? (
        <SummaryPane
          project={project}
          url={url}
          busy={busy}
          run={run}
          phpVersions={phpVersions}
          anyRunning={anyRunning}
          runningWorkers={runningWorkers}
          onOpenTab={setTab}
          onRemove={onRemove}
        />
      ) : null}
      {tab === "schedule" ? (
        <ScheduleSection project={project} busy={busy} run={run} jobs={jobs} />
      ) : null}
      {tab === "queue" ? (
        <QueueSection project={project} busy={busy} run={run} jobs={jobs} />
      ) : null}
      {tab === "release" ? (
        <ReleasePane project={project} busy={busy} run={run} />
      ) : null}
      {tab === "logs" ? <ProjectLogs project={project} embedded /> : null}
      {tab === "database" ? (
        <DatabasePane
          project={project}
          busy={busy}
          run={run}
          mysqlRunning={mysqlRunning}
          onRestore={onRestore}
        />
      ) : null}
    </article>
  );
}

function SummaryPane({
  project,
  url,
  busy,
  run,
  phpVersions,
  anyRunning,
  runningWorkers,
  onOpenTab,
  onRemove,
}: {
  project: Project;
  url: string;
  busy: boolean;
  run: Run;
  phpVersions: PackageStatus[];
  anyRunning: boolean;
  runningWorkers: number;
  onOpenTab: (tab: Tab) => void;
  onRemove: () => void;
}) {
  return (
    <div className="project-pane">
      <dl className="project-facts">
        <div>
          <dt>Adres</dt>
          <dd>{url}</dd>
        </div>
        <div>
          <dt>Klasör</dt>
          <dd title={project.path}>{project.path.replace(/^\\\\\?\\/, "")}</dd>
        </div>
        <div>
          <dt>Veritabanı</dt>
          <dd>
            <button
              type="button"
              className="section-link"
              onClick={() => onOpenTab("database")}
            >
              {databaseName(project.name)}
            </button>
          </dd>
        </div>
        <div>
          <dt>Zamanlama</dt>
          <dd>
            <button
              type="button"
              className="section-link"
              onClick={() => onOpenTab("schedule")}
            >
              {project.scheduleRunning
                ? "Çalışıyor"
                : project.schedule?.enabled
                  ? "Hazır"
                  : "Kapalı"}
            </button>
          </dd>
        </div>
        <div>
          <dt>Kuyruk</dt>
          <dd>
            <button
              type="button"
              className="section-link"
              onClick={() => onOpenTab("queue")}
            >
              {runningWorkers
                ? `${runningWorkers} işçi çalışıyor`
                : `${project.workers?.length ?? 0} işçi`}
            </button>
          </dd>
        </div>
        <div>
          <dt>.env</dt>
          <dd>
            <button
              type="button"
              className="section-link"
              onClick={() => onOpenTab("env")}
            >
              Düzenle
            </button>
          </dd>
        </div>
        <SummaryGit project={project} onOpen={() => onOpenTab("release")} />
      </dl>
      <ProjectPhp
        project={project}
        versions={phpVersions}
        busy={busy}
        run={run}
        anyRunning={anyRunning}
      />
      <div className="project-pane-foot">
        <button
          type="button"
          className="button secondary small"
          disabled={busy}
          onClick={onRemove}
        >
          <Trash2 size={16} />
          Listeden kaldır
        </button>
      </div>
    </div>
  );
}

function DatabasePane({
  project,
  busy,
  run,
  mysqlRunning,
  onRestore,
}: {
  project: Project;
  busy: boolean;
  run: Run;
  mysqlRunning: boolean;
  onRestore: () => void;
}) {
  const name = databaseName(project.name);
  return (
    <div className="project-pane">
      <p className="section-note">
        Veritabanı adı proje adından türetilir. ServerBond proje <code>.env</code>{" "}
        dosyasına kendiliğinden yazmaz. Ortam sekmesinden düzenleyebilirsiniz.
      </p>
      <dl className="project-facts">
        <div>
          <dt>MySQL veritabanı</dt>
          <dd>
            <code>{name}</code>
          </dd>
        </div>
      </dl>
      <div className="project-db-actions">
        <button
          className="button secondary"
          disabled={busy || !mysqlRunning}
          title={
            mysqlRunning
              ? "Proje adıyla veritabanı oluştur"
              : "Önce MySQL'i başlatın"
          }
          onClick={() =>
            void run("Veritabanı oluşturuluyor…", () =>
              call("database", { name: project.name, action: "create" }),
            )
          }
        >
          <Database size={16} />
          Oluştur
        </button>
        <button
          className="button secondary"
          disabled={busy || !mysqlRunning}
          title={mysqlRunning ? "SQL yedeği al" : "Önce MySQL'i başlatın"}
          onClick={() =>
            void run("Yedek alınıyor…", () =>
              call("database", { name: project.name, action: "backup" }),
            )
          }
        >
          <Archive size={16} />
          Yedek al
        </button>
        <button
          className="button secondary"
          disabled={busy || !mysqlRunning}
          title={
            mysqlRunning ? "SQL yedeğini geri yükle" : "Önce MySQL'i başlatın"
          }
          onClick={onRestore}
        >
          <Archive size={16} />
          Geri yükle
        </button>
      </div>
    </div>
  );
}

export function ProjectPhp({
  project,
  versions,
  busy,
  run,
  anyRunning,
}: {
  project: Project;
  versions: PackageStatus[];
  busy: boolean;
  run: Run;
  anyRunning: boolean;
}) {
  const [version, setVersion] = useState(project.phpVersion);
  useEffect(() => setVersion(project.phpVersion), [project.phpVersion]);
  const selected = versions.find((p) => p.version === version);
  const repair = Boolean(selected?.repairable && !selected.installed);
  return (
    <div className="project-runtime">
      <div className="project-php-controls">
        <label htmlFor={`php-${project.id}`}>PHP</label>
        <select
          id={`php-${project.id}`}
          aria-label={`${project.name} PHP sürümü`}
          value={version}
          disabled={busy}
          onChange={(e) => setVersion(e.target.value)}
        >
          {versions.map((p) => (
            <option key={p.version} value={p.version}>
              {p.version}
              {p.installed ? " · Kurulu" : ""}
            </option>
          ))}
        </select>
        <button
          className="button secondary small"
          disabled={
            busy ||
            (repair && anyRunning) ||
            (version === project.phpVersion && selected?.installed)
          }
          title={
            repair && anyRunning
              ? "Onarmak için önce ortamı durdurun"
              : undefined
          }
          onClick={() =>
            void run(`${project.name} için PHP ${version} hazırlanıyor…`, () =>
              call(repair ? "repair_project_php" : "select_project_php", {
                id: project.id,
                version,
              }),
            )
          }
        >
          {repair
            ? "Onar ve uygula"
            : selected?.installed
              ? "Uygula"
              : "İndir ve uygula"}
        </button>
        <span className={`service-status ${project.running ? "running" : ""}`}>
          <span className={`status-dot ${project.running ? "green" : ""}`} />
          {project.running ? `Çalışıyor · ${project.phpPort}` : "Durduruldu"}
        </span>
      </div>
      {project.issue ? (
        <p className="field-error" role="status">
          {project.issue}
        </p>
      ) : null}
    </div>
  );
}

export function CompactProjectRow({
  project,
  url,
  selected,
  onSelect,
}: {
  project: Project;
  url: string;
  selected?: boolean;
  onSelect?: () => void;
}) {
  return (
    <button
      type="button"
      className={`project-picker-item ${selected ? "selected" : ""}`}
      onClick={onSelect}
      aria-current={selected ? "true" : undefined}
    >
      <span className="project-icon">
        <Folder size={20} />
      </span>
      <span className="project-info">
        <strong>{project.name}</strong>
        <span className="project-url">{url}</span>
      </span>
      <span className={`service-status ${project.running ? "running" : ""}`}>
        <span className={`status-dot ${project.running ? "green" : ""}`} />
        {project.running ? "Çalışıyor" : "Kapalı"}
      </span>
    </button>
  );
}
