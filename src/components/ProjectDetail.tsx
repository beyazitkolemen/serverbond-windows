import { useEffect, useId, useState, type ReactNode } from "react";
import {
  Archive,
  Database,
  ExternalLink,
  LayoutDashboard,
  FileCode2,
  Clock3,
  ListTodo,
  GitBranch,
  ScrollText,
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
import StatusBadge from "./StatusBadge";
import SectionTabs from "./SectionTabs";

const tabs = [
  { id: "summary", label: "Özet", icon: LayoutDashboard },
  { id: "env", label: "Ortam", icon: FileCode2 },
  { id: "schedule", label: "Zamanlama", icon: Clock3 },
  { id: "queue", label: "Kuyruklar", icon: ListTodo },
  { id: "release", label: "Sürüm", icon: GitBranch },
  { id: "logs", label: "Günlükler", icon: ScrollText },
  { id: "database", label: "Veritabanı", icon: Database },
] as const;

type Tab = (typeof tabs)[number]["id"];

export default function ProjectDetail({
  project,
  projectPicker,
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
  projectPicker: ReactNode;
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
  const [openedEditors, setOpenedEditors] = useState<
    Partial<Record<Tab, string>>
  >({});
  const openTab = (next: Tab) => {
    if (next === "env" || next === "release") {
      setOpenedEditors((current) => ({ ...current, [next]: project.id }));
    }
    setTab(next);
  };
  const tabsId = useId();
  const jobs = useProjectJobs(project);
  useEffect(() => {
    setTab("summary");
  }, [project.id]);
  const url = projectUrl(project.host, webPort, https, httpsPort);
  const runningWorkers = jobs.runningWorkers;
  return (
    <article
      className="project-detail project-detail-workspace"
      aria-labelledby="project-detail-title"
    >
      <aside className="project-detail-nav">
        {projectPicker}
        <SectionTabs
          id={tabsId}
          label="Proje bölümleri"
          orientation="vertical"
          value={tab}
          onChange={openTab}
          items={tabs.map((item) => ({
            id: item.id,
            label: item.label,
            icon: <item.icon size={16} aria-hidden />,
            indicator:
              (item.id === "schedule" && project.scheduleRunning) ||
              (item.id === "queue" && runningWorkers) ? (
                <i className="tab-dot" aria-hidden />
              ) : null,
          }))}
        />
        <label className="project-section-select">
          <span>Bölüm</span>
          <select
            aria-label="Proje bölümü"
            value={tab}
            onChange={(event) => openTab(event.target.value as Tab)}
          >
            {tabs.map((item) => (
              <option key={item.id} value={item.id}>
                {item.label}
              </option>
            ))}
          </select>
        </label>
      </aside>
      <div className="project-detail-content">
        <header className="project-detail-head">
          <div className="project-detail-copy">
            <h3 id="project-detail-title">{project.name}</h3>
            <p className="project-url">{url}</p>
          </div>
          <div className="project-actions">
            <StatusBadge tone={project.running ? "running" : "stopped"}>
              {project.running ? `PHP · ${project.phpPort}` : "PHP kapalı"}
            </StatusBadge>
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
        <h4 className="project-section-title" id={`${tabsId}-heading`}>
          {tabs.find((item) => item.id === tab)?.label}
        </h4>
        <div
          id={`${tabsId}-panel`}
          role="tabpanel"
          aria-labelledby={`${tabsId}-heading`}
          tabIndex={0}
        >
          {openedEditors.env === project.id ? (
            <div hidden={tab !== "env"}>
              <ProjectEnv project={project} busy={busy} run={run} />
            </div>
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
              onOpenTab={openTab}
              onRemove={onRemove}
            />
          ) : null}
          {tab === "schedule" ? (
            <ScheduleSection
              project={project}
              busy={busy}
              run={run}
              jobs={jobs}
            />
          ) : null}
          {tab === "queue" ? (
            <QueueSection project={project} busy={busy} run={run} jobs={jobs} />
          ) : null}
          {openedEditors.release === project.id ? (
            <div hidden={tab !== "release"}>
              <ReleasePane project={project} busy={busy} run={run} />
            </div>
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
        </div>
      </div>
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
        <span className="section-note">
          Kayıttan kaldırma proje dosyalarını silmez.
        </span>
        <button
          type="button"
          className="button secondary small danger"
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
        Bağlantı bilgilerini Ortam sekmesindeki <code>.env</code> dosyasına
        girin.
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
  // A pending pick must not carry over to another project that happens to
  // run the same PHP version.
  useEffect(
    () => setVersion(project.phpVersion),
    [project.id, project.phpVersion],
  );
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
              ? "Onarmak için önce sunucuyu durdurun"
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
        <StatusBadge tone={project.running ? "running" : "stopped"}>
          {project.running ? `Çalışıyor · ${project.phpPort}` : "Durduruldu"}
        </StatusBadge>
      </div>
      {project.issue ? (
        <p className="field-error" role="status">
          {project.issue}
        </p>
      ) : null}
    </div>
  );
}
