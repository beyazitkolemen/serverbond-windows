import { useEffect, useState } from "react";
import { Rocket } from "lucide-react";
import { call } from "../api";
import type {
  Project,
  ProjectGitStatus,
  ProjectRelease,
  ReleaseRecord,
  Run,
} from "../types";
import { defaultRelease } from "../types";
import { useDraft } from "../hooks/useDraft";
import StatusBadge from "./StatusBadge";

function normalize(release?: ProjectRelease): ProjectRelease {
  return {
    ...defaultRelease(),
    ...release,
    extraArtisan: release?.extraArtisan ?? [],
  };
}

export default function ReleasePane({
  project,
  busy,
  run,
}: {
  project: Project;
  busy: boolean;
  run: Run;
}) {
  const source = normalize(project.release);
  const { values, setValues, reset } = useDraft(
    { release: source, extra: source.extraArtisan.join("\n") },
    `${project.name} · Sürüm tarifi`,
  );
  const { release: saved, extra } = values;
  const draft: ProjectRelease = {
    ...saved,
    extraArtisan: extra
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean),
  };
  const [history, setHistory] = useState<ReleaseRecord[]>([]);
  const [historyError, setHistoryError] = useState("");
  const [consoleText, setConsoleText] = useState("");
  const dirty = JSON.stringify(draft) !== JSON.stringify(source);
  const last = history[0];
  useEffect(() => {
    let cancelled = false;
    void call<ReleaseRecord[]>("list_project_releases", { id: project.id })
      .then((records) => {
        if (!cancelled) {
          setHistoryError("");
          setHistory(records);
          setConsoleText(records[0]?.output ?? "");
        }
      })
      .catch(() => {
        if (!cancelled) setHistoryError("Sürüm geçmişi okunamadı.");
      });
    return () => {
      cancelled = true;
    };
  }, [project.id]);
  const update = <K extends keyof ProjectRelease>(
    key: K,
    value: ProjectRelease[K],
  ) =>
    setValues((current) => ({
      ...current,
      release: { ...current.release, [key]: value },
    }));
  const saveRecipe = async () => {
    await call("save_project_release", { id: project.id, release: draft });
    setValues({ release: draft, extra: draft.extraArtisan.join("\n") });
  };
  const refreshHistory = () =>
    call<ReleaseRecord[]>("list_project_releases", { id: project.id }).then(
      (records) => {
        setHistoryError("");
        setHistory(records);
        if (records[0]) setConsoleText(records[0].output);
      },
    );
  return (
    <div className="project-pane project-release">
      <p className="section-note">
        Seçilen adımları bu bilgisayarda uygular. <code>.env</code> korunur.
      </p>
      <div className="release-toolbar">
        <StatusBadge
          tone={last ? (last.success ? "running" : "issue") : "stopped"}
        >
          {last
            ? last.success
              ? `Son sürüm başarılı · ${last.startedAt}`
              : `Son sürüm başarısız · ${last.startedAt}`
            : historyError
              ? "Sürüm durumu okunamadı"
              : "Henüz sürüm çalıştırılmadı"}
        </StatusBadge>
        <div className="release-actions">
          {dirty && (
            <button
              type="button"
              className="button secondary small"
              disabled={busy}
              onClick={reset}
            >
              Vazgeç
            </button>
          )}
          <button
            type="button"
            className="button secondary small"
            disabled={busy || !dirty}
            onClick={() => void run("Sürüm tarifi kaydediliyor…", saveRecipe)}
          >
            Kaydet
          </button>
          <button
            type="button"
            className="button primary small"
            disabled={busy}
            onClick={() =>
              void run("Yerel sürüm çalışıyor…", async () => {
                if (dirty) {
                  await saveRecipe();
                }
                const record = await call<ReleaseRecord>("deploy_project", {
                  id: project.id,
                }).catch(async (error: unknown) => {
                  await refreshHistory().catch(() =>
                    setHistoryError("Sürüm geçmişi okunamadı."),
                  );
                  throw error;
                });
                setConsoleText(record.output);
                await refreshHistory().catch(() =>
                  setHistoryError("Sürüm uygulandı ancak geçmiş yenilenemedi."),
                );
                return record;
              })
            }
          >
            <Rocket size={16} />
            Çalıştır
          </button>
        </div>
      </div>
      {historyError && (
        <p className="field-error" role="alert">
          {historyError}
        </p>
      )}
      <fieldset className="release-steps" disabled={busy}>
        <legend>Tarif</legend>
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={draft.gitPull}
            onChange={(e) => update("gitPull", e.target.checked)}
          />
          <span>git pull</span>
        </label>
        <label className="release-branch">
          Dal
          <input
            value={draft.branch}
            disabled={!draft.gitPull}
            placeholder="Mevcut dal"
            onChange={(e) => update("branch", e.target.value)}
          />
        </label>
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={draft.composer}
            onChange={(e) => update("composer", e.target.checked)}
          />
          <span>composer install --prefer-dist</span>
        </label>
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={draft.composerNoDev}
            disabled={!draft.composer}
            onChange={(e) => update("composerNoDev", e.target.checked)}
          />
          <span>--no-dev (üretim)</span>
        </label>
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={draft.migrate}
            onChange={(e) => update("migrate", e.target.checked)}
          />
          <span>php artisan migrate --force</span>
        </label>
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={draft.optimizeClear}
            onChange={(e) => update("optimizeClear", e.target.checked)}
          />
          <span>php artisan optimize:clear</span>
        </label>
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={draft.restartJobs}
            onChange={(e) => update("restartJobs", e.target.checked)}
          />
          <span>Kuyruk işçileri ve zamanlayıcıyı yeniden başlat</span>
        </label>
        <label className="release-extra">
          Ek Artisan satırları
          <textarea
            rows={3}
            value={extra}
            placeholder={"config:cache\nroute:cache --no-ansi"}
            onChange={(e) =>
              setValues((current) => ({ ...current, extra: e.target.value }))
            }
          />
          <span className="muted">
            Her satır bir komut; yalnızca a-z0-9:_- ve --bayrak /
            --bayrak=değer.
          </span>
        </label>
      </fieldset>
      <section className="release-console" aria-label="Sürüm çıktısı">
        <header>
          <h4>Çıktı</h4>
          {last?.sha ? (
            <span className="muted">
              {last.branch || "HEAD"} · {last.sha} ·{" "}
              {Math.round(last.durationMs / 1000)} sn
            </span>
          ) : null}
        </header>
        <pre>{consoleText || "Çalıştırdıktan sonra çıktı burada görünür."}</pre>
      </section>
      {history.length ? (
        <section className="release-history" aria-label="Sürüm geçmişi">
          <h4>Geçmiş</h4>
          <ol>
            {history.map((record, index) => (
              <li key={`${index}-${record.startedAt}-${record.sha}`}>
                <button
                  type="button"
                  className="section-link"
                  onClick={() => setConsoleText(record.output)}
                >
                  {record.startedAt}
                </button>
                <span className={record.success ? "ok" : "fail"}>
                  {record.success ? "başarılı" : "başarısız"}
                </span>
                <span className="muted">
                  {record.branch || "HEAD"}
                  {record.sha ? ` · ${record.sha}` : ""}
                </span>
              </li>
            ))}
          </ol>
        </section>
      ) : null}
    </div>
  );
}

export function SummaryGit({
  project,
  onOpen,
}: {
  project: Project;
  onOpen: () => void;
}) {
  const [git, setGit] = useState<ProjectGitStatus | null>(null);
  useEffect(() => {
    let cancelled = false;
    setGit(null);
    void call<ProjectGitStatus>("project_git_status", { id: project.id })
      .then((status) => {
        if (!cancelled) setGit(status);
      })
      .catch(() => {
        if (!cancelled) setGit(null);
      });
    return () => {
      cancelled = true;
    };
  }, [project.id]);
  if (!git?.present) return null;
  return (
    <div>
      <dt>Sürüm</dt>
      <dd>
        <button type="button" className="section-link" onClick={onOpen}>
          {git.branch || "HEAD"} · {git.sha}
        </button>
      </dd>
    </div>
  );
}
