import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { call, desktop } from "../api";
import { APP_VERSION } from "../version";
import {
  checkForAppUpdate,
  formatBytes,
  installAppUpdate,
  type UpdateInfo,
  type UpdateProgress,
} from "../updates";
import type { Run } from "../types";

export default function UpdateSettings({
  busy,
  running,
  run,
  available,
  onAvailable,
  openUpdates,
}: {
  busy: boolean;
  running: boolean;
  run: Run;
  available: UpdateInfo | null;
  onAvailable: (update: UpdateInfo | null) => void;
  openUpdates: number;
}) {
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [checking, setChecking] = useState(false);
  const openRelease = async (url: string) => {
    setError("");
    try {
      await call("app_update_open", { url });
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };
  const inspect = async (interactive: boolean) => {
    if (!desktop) {
      if (interactive) {
        setError("Güncelleme denetimi masaüstü uygulamasında kullanılabilir.");
      }
      return;
    }
    setChecking(true);
    setError("");
    if (interactive) setStatus("GitHub sürümleri denetleniyor…");
    try {
      const update = await checkForAppUpdate();
      onAvailable(update);
      setStatus(
        update
          ? `ServerBond ${update.version} yayımlanmış.`
          : "ServerBond güncel. Yeni bir GitHub sürümü yok.",
      );
    } catch (caught) {
      const message = caught instanceof Error ? caught.message : String(caught);
      if (interactive) {
        onAvailable(null);
        setError(message);
      } else {
        setStatus("");
      }
    } finally {
      setChecking(false);
    }
  };
  useEffect(() => {
    void inspect(openUpdates > 0);
  }, [openUpdates]);
  const percent =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : null;
  return (
    <section className="settings-section">
      <h2>Uygulama güncellemesi</h2>
      <p className="section-note">
        Güncellemeler onayınızla indirilip kurulur.
      </p>
      <p>
        Kurulu sürüm <strong>v{APP_VERSION}</strong>
      </p>
      {!desktop && (
        <p className="section-note">
          Güncellemeleri masaüstü uygulamasından denetleyin.
        </p>
      )}
      {error && (
        <p role="alert" className="settings-feedback">
          {error}
        </p>
      )}
      {status && !error && (
        <p role="status" className="settings-feedback">
          {status}
        </p>
      )}
      {available && (
        <div className="update-notes">
          <h3>ServerBond {available.version}</h3>
          <p className="section-note">
            Mevcut sürüm: v{available.currentVersion}.{" "}
            {available.installMode === "automatic"
              ? "Kurulum servisleri durdurur ve ServerBond’ı yeniden açar."
              : "Bu yayın elle kurulur. Kurulumdan önce tepsi menüsünden Çıkış seçin."}
          </p>
          {available.notes ? <pre>{available.notes}</pre> : null}
        </div>
      )}
      {progress && (
        <div className="update-progress-block">
          <div
            className="update-progress"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent ?? undefined}
            aria-label="İndirme ilerlemesi"
          >
            <span style={{ width: `${percent ?? 15}%` }} />
          </div>
          <p>
            {formatBytes(progress.downloaded)}
            {progress.total > 0 ? ` / ${formatBytes(progress.total)}` : ""}
            {percent !== null ? ` · %${percent}` : ""}
          </p>
        </div>
      )}
      <div className="settings-actions">
        <button
          type="button"
          className="button secondary"
          disabled={busy || checking || !desktop}
          onClick={() => void inspect(true)}
        >
          <RefreshCw size={16} className={checking ? "spin" : undefined} />
          Güncellemeleri denetle
        </button>
        {available?.installMode === "automatic" && (
          <button
            type="button"
            className="button primary"
            disabled={busy || checking || !desktop}
            onClick={() =>
              void run("Güncelleme indiriliyor ve kuruluyor…", async () => {
                setError("");
                setProgress({ downloaded: 0, total: 0 });
                try {
                  await installAppUpdate(
                    available.version,
                    setProgress,
                    async () => {
                      if (running) {
                        await call("service", { id: "all", action: "stop" });
                      }
                    },
                  );
                } catch (e) {
                  setProgress(null);
                  throw e;
                }
                return "Güncelleme kuruldu. ServerBond yeniden açılıyor.";
              })
            }
          >
            {available.version} sürümünü kur ve yeniden başlat
          </button>
        )}
        {available?.installMode === "manual" && (
          <a
            className="button primary"
            href={available.installerUrl ?? available.releaseUrl}
            target="_blank"
            rel="noreferrer"
            onClick={(event) => {
              if (desktop) {
                event.preventDefault();
                void openRelease(
                  available.installerUrl ?? available.releaseUrl,
                );
              }
            }}
          >
            {available.installerUrl
              ? `${available.version} kurulumunu indir`
              : "GitHub sürümünü aç"}
          </a>
        )}
      </div>
      <p className="section-note">
        Kaynak:{" "}
        <a
          href="https://github.com/beyazitkolemen/serverbond-windows/releases/latest"
          target="_blank"
          rel="noreferrer"
          onClick={(event) => {
            if (desktop) {
              event.preventDefault();
              void openRelease(event.currentTarget.href);
            }
          }}
        >
          GitHub Releases
        </a>
        . Kaynak depo: beyazitkolemen/serverbond-windows. İmzalı yayınlar
        uygulamadan kurulur; diğer yayınlarda kurulum bağlantısı gösterilir.
      </p>
    </section>
  );
}
