import { useEffect, useRef, useState } from "react";
import { Folder, Plus, X, FolderOpen, ExternalLink } from "lucide-react";
import { call, chooseFolder, chooseSqlFile } from "../api";
import { githubService } from "../services";
import type {
  DiscoveredProject,
  GithubState,
  Project,
  Run,
  PackageStatus,
} from "../types";
import {
  databaseName,
  phpSupportsLaravel12,
  projectAddress,
  projectUrl,
} from "../version";

function githubSlug(raw: string): string {
  const cleaned = raw
    .trim()
    .replace(/\.git$/i, "")
    .replace(/\/+$/, "")
    .replace(/\\/g, "/");
  const path = cleaned
    .replace(/^git@github\.com:/i, "")
    .replace(/^https?:\/\/github\.com\//i, "")
    .replace(/^github\.com\//i, "");
  const repo =
    path.split("/").filter(Boolean)[1] ?? path.split("/").pop() ?? "";
  return repo
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 48);
}
import ProjectDetail, { CompactProjectRow } from "./ProjectDetail";

export default function Projects({
  projects,
  busy,
  run,
  webPort,
  https,
  httpsPort,
  mysqlRunning,
  home,
  serverError,
  clearError,
  webRunning,
  phpVersion,
  phpVersions,
  anyRunning,
  hostPattern,
  github,
  compact = false,
  onOpen,
}: {
  projects: Project[];
  busy: boolean;
  run: Run;
  webPort: number;
  https: boolean;
  httpsPort: number;
  mysqlRunning: boolean;
  home: string;
  serverError: string;
  clearError: () => void;
  webRunning: boolean;
  phpVersion: string;
  phpVersions: PackageStatus[];
  anyRunning: boolean;
  hostPattern: string;
  github: GithubState;
  compact?: boolean;
  onOpen?: () => void;
}) {
  const [modal, setModal] = useState(false);
  const [remove, setRemove] = useState<Project | null>(null);
  const [restore, setRestore] = useState<Project | null>(null);
  const [discovered, setDiscovered] = useState<DiscoveredProject[] | null>(
    null,
  );
  const [selectedId, setSelectedId] = useState<string | null>(
    projects[0]?.id ?? null,
  );
  useEffect(() => {
    if (!projects.some((item) => item.id === selectedId)) {
      setSelectedId(projects[0]?.id ?? null);
    }
  }, [projects, selectedId]);
  const selected = projects.find((item) => item.id === selectedId) ?? null;
  return (
    <section aria-labelledby="projects-heading">
      <div className="section-heading">
        <h2
          id="projects-heading"
          className={compact ? undefined : "visually-hidden"}
        >
          Projeler
        </h2>
        <div className="heading-actions">
          {onOpen ? (
            <button type="button" className="section-link" onClick={onOpen}>
              Tümünü yönet
            </button>
          ) : (
            <button
              className="button secondary small"
              disabled={busy}
              onClick={() => {
                clearError();
                void run("Proje klasörleri taranıyor…", async () => {
                  const found =
                    await call<DiscoveredProject[]>("discover_projects");
                  setDiscovered(found);
                });
              }}
            >
              <FolderOpen size={16} />
              Klasör tara
            </button>
          )}
          <button
            className="button primary small"
            disabled={busy}
            onClick={() => {
              clearError();
              setModal(true);
            }}
          >
            <Plus size={16} />
            Proje ekle
          </button>
        </div>
      </div>
      {projects.length && compact ? (
        <div className="project-list">
          {projects.map((project) => (
            <article className="project-row" key={project.id}>
              <div className="project-head">
                <div className="project-icon">
                  <Folder size={24} />
                </div>
                <div className="project-info">
                  <h3>{project.name}</h3>
                  <p className="project-url">
                    {projectUrl(project.host, webPort, https, httpsPort)}
                  </p>
                  <p className="project-path" title={project.path}>
                    {project.path.replace(/^\\\\\?\\/, "")}
                  </p>
                  <p className="project-path">
                    {project.running
                      ? `PHP ${project.phpVersion} · ${project.phpPort}`
                      : `PHP ${project.phpVersion}`}
                  </p>
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
                    <ExternalLink size={16} />
                    Aç
                  </button>
                  <span
                    className={`service-status ${project.running ? "running" : ""}`}
                  >
                    <span
                      className={`status-dot ${project.running ? "green" : ""}`}
                    />
                    {project.running ? "Çalışıyor" : "Kapalı"}
                  </span>
                </div>
              </div>
            </article>
          ))}
        </div>
      ) : projects.length ? (
        <div className="project-workspace">
          <div className="project-picker" role="listbox" aria-label="Projeler">
            {projects.map((project) => (
              <CompactProjectRow
                key={project.id}
                project={project}
                url={projectUrl(project.host, webPort, https, httpsPort)}
                selected={project.id === selectedId}
                onSelect={() => setSelectedId(project.id)}
              />
            ))}
          </div>
          {selected ? (
            <ProjectDetail
              project={selected}
              busy={busy}
              run={run}
              webPort={webPort}
              https={https}
              httpsPort={httpsPort}
              mysqlRunning={mysqlRunning}
              webRunning={webRunning}
              phpVersions={phpVersions}
              anyRunning={anyRunning}
              onRemove={() => setRemove(selected)}
              onRestore={() => setRestore(selected)}
            />
          ) : null}
        </div>
      ) : (
        <div className="empty-state">
          <Folder size={44} strokeWidth={1.5} />
          <div>
            <h3>Çalışma alanında proje yok</h3>
            <p>
              Mevcut bir Laravel klasörü ekleyin, GitHub’dan klonlayın veya yeni
              proje oluşturun. PHP sürümü, kuyruk ve zamanlayıcı proje
              sayfasından yönetilir.
            </p>
            <div className="empty-actions">
              <button
                className="button secondary small"
                disabled={busy}
                onClick={() => {
                  clearError();
                  void run("Proje klasörleri taranıyor…", async () => {
                    const found =
                      await call<DiscoveredProject[]>("discover_projects");
                    setDiscovered(found);
                  });
                }}
              >
                <FolderOpen size={16} />
                Klasör tara
              </button>
              <button
                className="button primary small"
                disabled={busy}
                onClick={() => {
                  clearError();
                  setModal(true);
                }}
              >
                <Plus size={16} />
                Proje ekle
              </button>
            </div>
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
          github={github}
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
      {restore ? (
        <ConfirmRestore
          project={restore}
          busy={busy}
          close={() => setRestore(null)}
          confirm={async (path) => {
            if (
              await run("Veritabanı geri yükleniyor…", () =>
                call("database", {
                  name: restore.name,
                  action: "restore",
                  path,
                }),
              )
            )
              setRestore(null);
          }}
        />
      ) : null}
      {discovered ? (
        <DiscoverDialog
          items={discovered}
          busy={busy}
          serverError={serverError}
          close={() => setDiscovered(null)}
          importAll={async () => {
            if (
              await run("Bulunan projeler ekleniyor…", () =>
                call("import_projects", {
                  paths: discovered.map((item) => item.path),
                }),
              )
            )
              setDiscovered(null);
          }}
        />
      ) : null}
    </section>
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
  github,
}: {
  home: string;
  busy: boolean;
  run: Run;
  close: () => void;
  serverError: string;
  phpVersion: string;
  hostPattern: string;
  github: GithubState;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [mode, setMode] = useState<"existing" | "create" | "github">(
    "existing",
  );
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [repository, setRepository] = useState("");
  const [branch, setBranch] = useState("");
  const [error, setError] = useState("");
  const canCreate = phpSupportsLaravel12(phpVersion);
  const create = mode === "create";
  const fromGithub = mode === "github";
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
    if (fromGithub && !repository.trim()) {
      setError("GitHub deposunu owner/repo veya URL olarak yazın.");
      return;
    }
    if (!fromGithub && !path.trim()) {
      setError("Bir klasör seçin.");
      return;
    }
    setError("");
    if (
      await run(
        fromGithub
          ? "GitHub deposu klonlanıyor…"
          : create
            ? "Laravel oluşturuluyor… Composer günlüğünden takip edebilirsiniz."
            : "Proje ekleniyor…",
        () =>
          fromGithub
            ? githubService.import({
                repository: repository.trim(),
                name,
                branch: branch.trim(),
              })
            : call("add_project", { name, path: path.trim(), create }),
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
          className={mode === "existing" ? "active" : ""}
          disabled={busy}
          onClick={() => {
            setMode("existing");
            setPath("");
          }}
        >
          Mevcut proje
        </button>
        <button
          className={mode === "create" ? "active" : ""}
          disabled={busy}
          onClick={() => {
            setMode("create");
            setPath(`${home.replace(/^\\\\\?\\/, "")}`);
          }}
        >
          Yeni Laravel projesi
        </button>
        <button
          className={mode === "github" ? "active" : ""}
          disabled={busy}
          onClick={() => {
            setMode("github");
            setPath("");
          }}
        >
          GitHub
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
        {fromGithub ? (
          <>
            <label>
              GitHub deposu
              <input
                name="repository"
                placeholder="owner/repo veya https://github.com/owner/repo"
                value={repository}
                disabled={busy}
                onChange={(e) => {
                  const value = e.target.value;
                  setRepository(value);
                  const slug = githubSlug(value);
                  if (slug && (!name || name === githubSlug(repository))) {
                    setName(slug);
                  }
                }}
                required
              />
            </label>
            <label>
              Dal (isteğe bağlı)
              <input
                name="branch"
                placeholder="varsayılan dal"
                value={branch}
                disabled={busy}
                onChange={(e) => setBranch(e.target.value)}
              />
            </label>
          </>
        ) : (
          <>
            <label htmlFor="project-path">
              {create ? "Oluşturulacağı üst klasör" : "Laravel proje klasörü"}
            </label>
            <div className="input-with-button">
              <input
                id="project-path"
                placeholder={
                  create ? "C:\\Projeler" : "C:\\Projeler\\ornek-proje"
                }
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
          </>
        )}
        <p className="field-hint">
          {fromGithub
            ? `Depo ${home.replace(/^\\\\\?\\/, "")}\\${name || "ornek-proje"} klasörüne klonlanır. ${
                github.tokenSaved
                  ? github.login
                    ? `Kayıtlı hesap: ${github.login}.`
                    : "GitHub jetonu kayıtlı."
                  : "Özel depolar için Hizmetler → GitHub ekranından jeton kaydedin."
              }`
            : create
              ? `Üst klasörde ornek-proje/ oluşur. Laravel 12 için PHP 8.2 veya üzeri ve Composer gerekir. Seçili PHP: ${phpVersion}.`
              : "public/index.php içeren kök klasörü seçin. Mevcut dosyalarınız değiştirilmez."}{" "}
          MySQL çalışıyorsa {databaseName(name || "ornek-proje")} veritabanı
          oluşturulur; proje .env dosyası yazılmaz.
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
              : fromGithub
                ? "GitHub'dan ekle"
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

function ConfirmRestore({
  project,
  busy,
  close,
  confirm,
}: {
  project: Project;
  busy: boolean;
  close: () => void;
  confirm: (path: string) => Promise<void>;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const [path, setPath] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      ref={ref}
      className="modal"
      aria-labelledby="restore-title"
      onCancel={(e) => {
        e.preventDefault();
        if (!busy) close();
      }}
    >
      <h2 id="restore-title">SQL yedeğini geri yükle</h2>
      <p className="dialog-copy">
        {databaseName(project.name)} veritabanına seçilen .sql dosyası
        uygulanır. Dosyadaki diğer veritabanı ifadeleri yok sayılır. Mevcut
        tablolar çakışırsa içe aktarma hata verebilir.
      </p>
      <div className="input-with-button">
        <input
          value={path}
          disabled={busy}
          placeholder="C:\\yedekler\\proje.sql"
          onChange={(e) => setPath(e.target.value)}
        />
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          aria-label="SQL dosyası seç"
          onClick={async () => {
            try {
              const file = await chooseSqlFile();
              if (file) setPath(file);
            } catch (e) {
              setError(String(e));
            }
          }}
        >
          <FolderOpen size={19} />
        </button>
      </div>
      {error ? (
        <p className="field-error" role="alert">
          {error}
        </p>
      ) : null}
      <div className="modal-actions">
        <button className="button secondary" disabled={busy} onClick={close}>
          Vazgeç
        </button>
        <button
          className="button primary"
          disabled={busy || !path.trim()}
          onClick={() => void confirm(path.trim())}
        >
          Geri yükle
        </button>
      </div>
    </dialog>
  );
}

function DiscoverDialog({
  items,
  busy,
  serverError,
  close,
  importAll,
}: {
  items: DiscoveredProject[];
  busy: boolean;
  serverError: string;
  close: () => void;
  importAll: () => Promise<void>;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      ref={ref}
      className="modal"
      aria-labelledby="discover-title"
      onCancel={(e) => {
        e.preventDefault();
        if (!busy) close();
      }}
    >
      <h2 id="discover-title">Klasör taraması</h2>
      {items.length ? (
        <>
          <p className="dialog-copy">
            Çalışma alanında {items.length} Laravel kökü bulundu. Kayıtlı
            projeler atlandı. İki seviye (müşteri/uygulama) taranır; .env
            dosyaları değiştirilmez.
          </p>
          <ul className="discover-list">
            {items.map((item) => (
              <li key={item.path}>
                <strong>{item.name}</strong>
                <span>{item.host}</span>
                <span>{item.path.replace(/^\\\\\?\\/, "")}</span>
              </li>
            ))}
          </ul>
        </>
      ) : (
        <p className="dialog-copy">
          Çalışma alanında henüz kayıtlı olmayan Laravel kökü yok.
          public/index.php içeren bir veya iki seviye klasörler taranır.
        </p>
      )}
      {serverError ? (
        <p className="field-error" role="alert">
          {serverError}
        </p>
      ) : null}
      <div className="modal-actions">
        <button className="button secondary" disabled={busy} onClick={close}>
          Kapat
        </button>
        {items.length ? (
          <button
            className="button primary"
            disabled={busy}
            onClick={() => void importAll()}
          >
            Hepsini ekle
          </button>
        ) : null}
      </div>
    </dialog>
  );
}
