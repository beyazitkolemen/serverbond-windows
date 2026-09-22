import {
  useUnsavedChanges,
  useNavigationGuard,
} from "../hooks/useNavigationGuard";
import { useEffect, useState } from "react";
import { call } from "../api";
import type { Project, ProjectEnv as EnvFile, Run } from "../types";

export default function ProjectEnv({
  project,
  busy,
  run,
}: {
  project: Project;
  busy: boolean;
  run: Run;
}) {
  const navigate = useNavigationGuard();
  const [reload, setReload] = useState(0);
  const [file, setFile] = useState<EnvFile | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    setFile(null);
    setError("");
    void call<EnvFile>("read_project_env", { id: project.id })
      .then((value) => {
        if (!active) return;
        setFile(value);
        setDraft(value.content);
      })
      .catch((reason) => {
        if (active) setError(String(reason));
      });
    return () => {
      active = false;
    };
  }, [project.id, reload]);
  const dirty = file !== null && draft !== file.content;
  useUnsavedChanges(dirty, `${project.name} · Ortam dosyası`);
  return (
    <div className="project-pane env-editor">
      <p className="section-note">
        Değişiklikler Kaydet ile uygulanır. Ardından PHP, kuyruk ve
        zamanlayıcıyı yeniden başlatın.
      </p>
      {error ? (
        <p className="field-error" role="alert">
          {error}
        </p>
      ) : null}
      {file ? (
        <p className="section-note">
          {file.exists
            ? "Dosya mevcut."
            : ".env henüz yok; kaydedince oluşturulur."}
          {file.example ? " .env.example bulundu." : null}
        </p>
      ) : !error ? (
        <p className="section-note" role="status">
          .env okunuyor…
        </p>
      ) : null}
      {dirty && (
        <p className="draft-indicator" role="status">
          Kaydedilmemiş değişiklikler
        </p>
      )}
      <label className="env-editor-label">
        .env
        <textarea
          spellCheck={false}
          autoComplete="off"
          autoCorrect="off"
          value={draft}
          disabled={busy || !file}
          onChange={(e) => setDraft(e.target.value)}
          aria-label={`${project.name} .env`}
        />
      </label>
      <div className="settings-actions">
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={() =>
            navigate(
              () => setReload((value) => value + 1),
              [`${project.name} · Ortam dosyası`],
            )
          }
        >
          Dosyayı yeniden yükle
        </button>
        {file?.example ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() => setDraft(file.example ?? "")}
          >
            Örnekten doldur
          </button>
        ) : null}
        <button
          type="button"
          className="button secondary"
          disabled={busy || !dirty}
          onClick={() => setDraft(file?.content ?? "")}
        >
          Vazgeç
        </button>
        <button
          type="button"
          className="button primary"
          disabled={busy || !file || !dirty}
          onClick={() =>
            void run(".env kaydediliyor…", async () => {
              const saved = await call<EnvFile>("save_project_env", {
                id: project.id,
                content: draft,
                expected: file,
              });
              setFile(saved);
              return ".env kaydedildi. Açık PHP ve iş süreçlerini yeniden başlatın.";
            })
          }
        >
          Kaydet
        </button>
      </div>
    </div>
  );
}
