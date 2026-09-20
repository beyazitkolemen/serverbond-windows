import { useEffect, useRef, useState } from "react";
import {
  Folder,
  Plus,
  ExternalLink,
  Trash2,
  X,
  FolderOpen,
  Database,
  Archive,
  Terminal,
} from "lucide-react";
import { call, chooseFolder } from "../api";
import type { Project, Run, PackageStatus } from "../types";
import { phpSupportsLaravel12, projectAddress } from "../version";
import ProjectJobs from "./ProjectJobs";

export default function Projects({
  projects,
  busy,
  run,
  webPort,
  mysqlRunning,
  home,
  serverError,
  clearError,
  webRunning,
  phpVersion,
  phpVersions,
  anyRunning,
  hostPattern,
}: {
  projects: Project[];
  busy: boolean;
  run: Run;
  webPort: number;
  mysqlRunning: boolean;
  home: string;
  serverError: string;
  clearError: () => void;
  webRunning: boolean;
  phpVersion: string;
  phpVersions: PackageStatus[];
  anyRunning: boolean;
  hostPattern: string;
}) {
  const [modal, setModal] = useState(false);
  const [remove, setRemove] = useState<Project | null>(null);
  return (
    <section aria-labelledby="projects-heading">
      <div className="section-heading">
        <h2 id="projects-heading">Projeler</h2>
        <button
          className="button primary small"
          disabled={busy}
          onClick={() => {
            clearError();
            setModal(true);
          }}
        >
          <Plus size={17} />
          Proje ekle
        </button>
      </div>
      {projects.length ? (
        <div className="project-list">
          {projects.map((project) => (
            <article className="project-row" key={project.id}>
              <div className="project-icon">
                <Folder size={24} />
              </div>
              <div className="project-info">
                <h3>{project.name}</h3>
                <p className="project-url">
                  http://{project.host}:{webPort}
                </p>
                <p className="project-path" title={project.path}>
                  {project.path.replace(/^\\\\\?\\/, "")}
                </p>
                <ProjectPhp
                  project={project}
                  versions={phpVersions}
                  busy={busy}
                  run={run}
                  anyRunning={anyRunning}
                />
                <ProjectJobs project={project} busy={busy} run={run} />
              </div>
              <div className="project-actions">
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
                  <ExternalLink size={15} />
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
                  <Terminal size={15} /> Terminal
                </button>
                <button
                  className="button secondary small"
                  disabled={busy || !mysqlRunning}
                  title={
                    mysqlRunning
                      ? "Proje adıyla veritabanı oluştur"
                      : "Önce MySQL'i başlatın"
                  }
                  onClick={() =>
                    void run("Veritabanı oluşturuluyor…", () =>
                      call("database", {
                        name: project.name,
                        action: "create",
                      }),
                    )
                  }
                >
                  <Database size={15} />
                  Veritabanı
                </button>
                <button
                  className="button secondary small"
                  disabled={busy || !mysqlRunning}
                  title={
                    mysqlRunning ? "SQL yedeği al" : "Önce MySQL'i başlatın"
                  }
                  onClick={() =>
                    void run("Yedek alınıyor…", () =>
                      call("database", {
                        name: project.name,
                        action: "backup",
                      }),
                    )
                  }
                >
                  <Archive size={15} />
                  Yedek
                </button>
                <button
                  className="icon-button danger"
                  disabled={busy}
                  title="Listeden kaldır"
                  aria-label={`${project.name} listeden kaldır`}
                  onClick={() => setRemove(project)}
                >
                  <Trash2 size={17} />
                </button>
              </div>
            </article>
          ))}
        </div>
      ) : (
        <div className="empty-state">
          <Folder size={44} strokeWidth={1.5} />
          <div>
            <h3>İlk Laravel projenizi ekleyin</h3>
            <p>
              Mevcut klasörü seçin veya yeni proje oluşturun. PHP sürümü, kuyruk
              işçileri ve zamanlayıcı proje kartından yönetilir.
            </p>
          </div>
        </div>
      )}
      {modal ? (
        <ProjectDialog
          home={home}
          busy={busy}
          run={run}
          serverError={serverError}
          phpVersion={phpVersion}
          hostPattern={hostPattern}
          close={() => setModal(false)}
        />
      ) : null}
      {remove ? (
        <ConfirmRemove
          project={remove}
          busy={busy}
          close={() => setRemove(null)}
          confirm={async () => {
            if (
              await run("Proje kaldırılıyor…", () =>
                call("remove_project", { id: remove.id }),
              )
            )
              setRemove(null);
          }}
        />
      ) : null}
    </section>
  );
}

function ProjectPhp({
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
  const [log, setLog] = useState<string | null>(null);
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
      <button
        className="runtime-log-toggle"
        disabled={busy}
        onClick={() => {
          if (log !== null) setLog(null);
          else
            void run("PHP günlüğü okunuyor…", async () =>
              setLog(
                await call<string>("read_log", {
                  id: `php-project-${project.id}`,
                }),
              ),
            );
        }}
      >
        {log !== null ? "PHP günlüğünü gizle" : "PHP günlüğü"}
      </button>
      {log !== null ? (
        <pre className="console project-console">{log}</pre>
      ) : null}
    </div>
  );
}

function ProjectDialog({
  home,
  busy,
  run,
  close,
  serverError,
  phpVersion,
  hostPattern,
}: {
  home: string;
  busy: boolean;
  run: Run;
  close: () => void;
  serverError: string;
  phpVersion: string;
  hostPattern: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [create, setCreate] = useState(false);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [error, setError] = useState("");
  const canCreate = phpSupportsLaravel12(phpVersion);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!/^[a-z0-9](?:[a-z0-9-]{0,46}[a-z0-9])?$/.test(name)) {
      setError(
        "Küçük harf, rakam ve arada tire kullanın. En fazla 48 karakter.",
      );
      return;
    }
    if (!path.trim()) {
      setError("Bir klasör seçin.");
      return;
    }
    setError("");
    if (
      await run(
        create
          ? "Laravel oluşturuluyor… Composer günlüğünden takip edebilirsiniz."
          : "Proje ekleniyor…",
        () => call("add_project", { name, path: path.trim(), create }),
      )
    )
      close();
  }
  return (
    <dialog
      ref={dialog}
      className="modal"
      onCancel={(e) => {
        e.preventDefault();
        if (!busy) close();
      }}
      aria-labelledby="project-dialog-title"
    >
      <div className="modal-header">
        <h2 id="project-dialog-title">Proje ekle</h2>
        <button
          className="icon-button"
          disabled={busy}
          aria-label="Kapat"
          onClick={close}
        >
          <X size={20} />
        </button>
      </div>
      <div className="tabs" role="group" aria-label="Proje türü">
        <button
          className={!create ? "active" : ""}
          disabled={busy}
          onClick={() => {
            setCreate(false);
            setPath("");
          }}
        >
          Mevcut proje
        </button>
        <button
          className={create ? "active" : ""}
          disabled={busy}
          onClick={() => {
            setCreate(true);
            setPath(`${home.replace(/^\\\\\?\\/, "")}`);
          }}
        >
          Yeni Laravel projesi
        </button>
      </div>
      <form onSubmit={(e) => void submit(e)}>
        <label>
          Proje adı
          <input
            autoFocus
            name="name"
            placeholder="ornek-proje"
            value={name}
            disabled={busy}
            onChange={(e) => setName(e.target.value)}
            required
            maxLength={48}
          />
        </label>
        <p className="field-hint">
          {projectAddress(hostPattern, name)} adresinden erişilir.
        </p>
        <label htmlFor="project-path">
          {create ? "Oluşturulacağı üst klasör" : "Laravel proje klasörü"}
        </label>
        <div className="input-with-button">
          <input
            id="project-path"
            placeholder="C:\\Projeler\\ornek-proje"
            value={path}
            disabled={busy}
            onChange={(e) => setPath(e.target.value)}
            required
          />
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            aria-label="Klasör seç"
            onClick={async () => {
              try {
                const folder = await chooseFolder();
                if (folder) setPath(folder);
              } catch (e) {
                setError(String(e));
              }
            }}
          >
            <FolderOpen size={19} />
          </button>
        </div>
        <p className="field-hint">
          {create
            ? `Laravel 12 için PHP 8.2 veya üzeri ve Composer gerekir. Seçili PHP: ${phpVersion}.`
            : "public/index.php içeren kök klasörü seçin. Mevcut dosyalarınız değiştirilmez."}
        </p>
        {create && !canCreate ? (
          <p className="field-error" role="status">
            Yeni proje için Bileşenler ekranından PHP 8.2 veya üzerini seçin.
          </p>
        ) : null}
        {error || serverError ? (
          <p className="field-error" role="alert">
            {error || serverError}
          </p>
        ) : null}
        {busy && create ? <CreationProgress /> : null}
        <div className="modal-actions">
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={close}
          >
            Vazgeç
          </button>
          <button
            className="button primary"
            disabled={busy || (create && !canCreate)}
            type="submit"
          >
            {busy
              ? "İşlem sürüyor…"
              : create
                ? "Laravel oluştur"
                : "Projeyi ekle"}
          </button>
        </div>
      </form>
    </dialog>
  );
}

function CreationProgress() {
  const [log, setLog] = useState("Composer hazırlanıyor…");
  useEffect(() => {
    let active = true;
    const refresh = async () => {
      try {
        const value = await call<string>("read_log", { id: "composer" });
        if (active) setLog(value.split("\n").slice(-8).join("\n"));
      } catch (error) {
        if (active) setLog(String(error));
      }
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), 2000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, []);
  return (
    <pre
      className="console creation-console"
      aria-label="Laravel kurulum günlüğü"
    >
      {log}
    </pre>
  );
}

function ConfirmRemove({
  project,
  busy,
  close,
  confirm,
}: {
  project: Project;
  busy: boolean;
  close: () => void;
  confirm: () => Promise<void>;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      ref={ref}
      className="modal"
      aria-labelledby="remove-title"
      onCancel={(e) => {
        e.preventDefault();
        if (!busy) close();
      }}
    >
      <h2 id="remove-title">Projeyi listeden kaldır</h2>
      <p className="dialog-copy">
        {project.name} listeden kaldırılacak. Proje dosyaları ve veritabanı
        korunacak.
      </p>
      <div className="modal-actions">
        <button className="button secondary" disabled={busy} onClick={close}>
          Vazgeç
        </button>
        <button
          className="button danger-button"
          disabled={busy}
          onClick={() => void confirm()}
        >
          Listeden kaldır
        </button>
      </div>
    </dialog>
  );
}
