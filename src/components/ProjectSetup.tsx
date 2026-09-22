import { useEffect, useRef, useState } from "react";
import type { PackageStatus, Run } from "../types";
import {
  projectSetup,
  type SetupReport,
  type SetupRequest,
} from "../services/projectSetup";
const labels: Record<string, string> = {
  workspace: "Klasör erişimi",
  destination: "Proje ve hedef klasör",
  disk: "Boş disk",
  php: "PHP",
  composer: "Composer",
  git: "Git",
  node: "Node.js",
};
export default function ProjectSetup({
  versions,
  selectedPhp,
  busy,
  run,
  close,
  serverError,
}: {
  versions: PackageStatus[];
  selectedPhp: string;
  busy: boolean;
  run: Run;
  close: () => void;
  serverError: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const active = useRef(false);
  const [step, setStep] = useState(0);
  const [report, setReport] = useState<SetupReport | null>(null);
  const [error, setError] = useState("");
  const [checking, setChecking] = useState(false);
  const [input, setInput] = useState<SetupRequest>({
    requestId: crypto.randomUUID(),
    source: "github",
    name: "",
    location: "",
    branch: "",
    phpVersion: selectedPhp,
    installDependencies: true,
    composer: true,
    build: false,
  });
  const change = (patch: Partial<SetupRequest>) => {
    setInput((i) => ({ ...i, ...patch, requestId: crypto.randomUUID() }));
    setReport(null);
    setError("");
  };
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  async function check() {
    if (active.current) return;
    active.current = true;
    setChecking(true);
    setError("");
    try {
      const result = await projectSetup.check(input);
      setReport(result);
      setStep(2);
    } catch (e) {
      setError(String(e));
    } finally {
      active.current = false;
      setChecking(false);
    }
  }
  return (
    <dialog
      ref={dialog}
      className="modal"
      aria-labelledby="setup-title"
      onClose={() => {
        if (busy || active.current) dialog.current?.showModal();
        else close();
      }}
      onCancel={(e) => {
        e.preventDefault();
        if (!busy && !active.current) close();
      }}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (step < 2) {
            if (step === 0) setStep(1);
            else void check();
            return;
          }
          if (active.current || busy || !report?.ready) return;
          active.current = true;
          void run("Proje hazırlanıyor…", () => projectSetup.run(input))
            .then((ok) => {
              if (ok) close();
              else {
                setInput((i) => ({ ...i, requestId: crypto.randomUUID() }));
                setReport(null);
                setStep(1);
              }
            })
            .finally(() => {
              active.current = false;
            });
        }}
      >
        <div className="modal-header">
          <h2 id="setup-title">Yeni proje kurulumu</h2>
          <button
            type="button"
            className="icon-button"
            disabled={busy || checking}
            onClick={close}
            aria-label="Kapat"
          >
            ×
          </button>
        </div>
        <p className="muted">
          {step + 1}/3 ·{" "}
          {["Kaynak", "Gereksinimler", "Kontrol ve kurulum"][step]}
        </p>
        {(error || serverError) && (
          <p role="alert" className="notice error">
            {error || serverError}
          </p>
        )}
        <fieldset disabled={busy || checking}>
          {step === 0 && (
            <>
              <label>
                Kaynak
                <select
                  value={input.source}
                  onChange={(e) =>
                    change({ source: e.target.value as SetupRequest["source"] })
                  }
                >
                  <option value="github">GitHub deposu</option>
                  <option value="git">Git deposu</option>
                  <option value="laravel">Yeni Laravel</option>
                  <option value="existing">Mevcut klasör</option>
                </select>
              </label>
              <label>
                Proje adı
                <input
                  required
                  maxLength={48}
                  pattern="[a-z0-9]+(-[a-z0-9]+)*"
                  value={input.name}
                  onChange={(e) => change({ name: e.target.value })}
                />
              </label>
              {input.source !== "laravel" && (
                <label>
                  {input.source === "github"
                    ? "GitHub deposu (ekip/depo)"
                    : input.source === "git"
                      ? "HTTPS depo adresi"
                      : "Proje klasörü"}
                  <input
                    required
                    value={input.location}
                    onChange={(e) => change({ location: e.target.value })}
                  />
                </label>
              )}
              {["git", "github"].includes(input.source) && (
                <label>
                  Dal (boşsa varsayılan)
                  <input
                    maxLength={128}
                    value={input.branch}
                    onChange={(e) => change({ branch: e.target.value })}
                  />
                </label>
              )}
            </>
          )}
          {step === 1 && (
            <>
              <label>
                PHP sürümü
                <select
                  value={input.phpVersion}
                  onChange={(e) => change({ phpVersion: e.target.value })}
                >
                  {versions.map((v) => (
                    <option key={v.version} value={v.version}>
                      {v.version}
                      {v.installed ? " · Kurulu" : " · Kurulacak"}
                    </option>
                  ))}
                </select>
              </label>
              {(["installDependencies", "composer", "build"] as const).map(
                (key, index) => (
                  <label className="checkbox-row" key={key}>
                    <input
                      type="checkbox"
                      checked={input[key]}
                      onChange={(e) => change({ [key]: e.target.checked })}
                    />
                    {
                      [
                        "Eksik PHP, Composer ve Node.js bileşenlerini kur",
                        "Composer bağımlılıklarını kur",
                        "Frontend derlemesini çalıştır (npm ci, npm run build)",
                      ][index]
                    }
                  </label>
                ),
              )}
              <p className="muted">
                Mevcut .env ve veritabanı korunur. Projeye özel PHP seçimi genel
                PHP ayarını değiştirmez.
              </p>
            </>
          )}
          {step === 2 && report && (
            <>
              <p>
                <strong>{input.name}</strong> · PHP {input.phpVersion}
              </p>
              <ul>
                {report.checks.map((c) => (
                  <li key={c.id}>
                    {labels[c.id]}:{" "}
                    {c.ok
                      ? "Hazır"
                      : c.installable && input.installDependencies
                        ? "Kurulacak"
                        : "Kontrol gerekli"}
                  </li>
                ))}
              </ul>
              {report.sourceInspectionPending && (
                <p>
                  Depo erişimi ve PHP/uzantı gereksinimleri kaynak alındıktan
                  sonra kontrol edilir.
                </p>
              )}
              <p>
                Kurulum internet yayını açmaz. Eksik ortam ayarlarını proje
                sayfasından tamamlayın.
              </p>
            </>
          )}
        </fieldset>
        <div className="modal-actions">
          {step > 0 && (
            <button
              type="button"
              className="button secondary"
              disabled={busy || checking}
              onClick={() => setStep(step - 1)}
            >
              Geri
            </button>
          )}
          <button
            className="button primary"
            disabled={busy || checking || (step === 2 && !report?.ready)}
          >
            {["Devam", "Gereksinimleri kontrol et", "Kurulumu başlat"][step]}
          </button>
        </div>
      </form>
    </dialog>
  );
}
