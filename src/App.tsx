import { useCallback, useEffect, useRef, useState } from "react";
import {
  Check,
  Play,
  Square,
  RefreshCw,
  Monitor,
  ChevronRight,
  PanelBottomClose,
  Ellipsis,
  LogOut,
} from "lucide-react";
import { call, desktop } from "./api";
import { listen } from "@tauri-apps/api/event";
import type { Page, Snapshot, Run } from "./types";
import { Shell, Notice } from "./components/Shell";
import Packages from "./components/Packages";
import Projects from "./components/Projects";
import Logs, { LogPreview } from "./components/Logs";
import Settings from "./components/Settings";
import EnvironmentSummary from "./components/EnvironmentSummary";

const headings: Record<Page, [string, string]> = {
  overview: [
    "Geliştirmeye yer açın.",
    "Servislerinizi yönetin. Projenize odaklanın.",
  ],
  packages: [
    "Araç kutunuz.",
    "Geliştirme araçlarını kurun, başlatın ve yönetin.",
  ],
  projects: [
    "Projeleriniz.",
    "Mevcut projenizi ekleyin veya yeni bir Laravel projesi oluşturun.",
  ],
  logs: ["Günlükler.", "Kurulum ve servis kayıtlarını buradan takip edin."],
  settings: [
    "Ayarlar.",
    "PHP, veritabanı, web sunucusu ve çalışma alanı tercihleri.",
  ],
};

export default function App() {
  const [page, setPage] = useState<Page>("overview");
  const [state, setState] = useState<Snapshot | null>(null);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [connectionError, setConnectionError] = useState("");
  const [message, setMessage] = useState("");
  const inFlight = useRef(false);
  const requestNumber = useRef(0);
  const refresh = useCallback(async () => {
    const request = ++requestNumber.current;
    try {
      const next = await call<Snapshot>("snapshot");
      if (request === requestNumber.current) {
        setState(next);
        setConnectionError("");
      }
    } catch (error) {
      if (request === requestNumber.current) {
        setConnectionError(`Ortam bilgileri alınamadı: ${String(error)}`);
      }
      throw error;
    }
  }, []);
  useEffect(() => {
    let loading = false;
    const poll = async () => {
      if (loading) return;
      loading = true;
      try {
        await refresh();
      } catch {
        /* refresh owns the latest error */
      } finally {
        loading = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1500);
    return () => {
      ++requestNumber.current;
      clearInterval(timer);
    };
  }, [refresh]);
  useEffect(() => {
    if (!desktop) return;
    let active = true;
    const cleanup: (() => void)[] = [];
    const navigate = (target: string | null) => {
      if (active && target && Object.hasOwn(headings, target))
        setPage(target as Page);
    };
    const connect = async () => {
      const nav = await listen<string>("desktop:navigate", () => {
        void call<string | null>("desktop_navigation")
          .then(navigate)
          .catch(() => {});
      });
      if (!active) {
        nav();
        return;
      }
      cleanup.push(nav);
      const errors = await listen<string>("desktop:error", (event) => {
        if (active) setError(event.payload);
      });
      if (!active) {
        errors();
        return;
      }
      cleanup.push(errors);
      navigate(await call<string | null>("desktop_navigation"));
    };
    void connect().catch((error) => {
      if (active)
        setError(`Masaüstü bildirimleri bağlanamadı: ${String(error)}`);
    });
    return () => {
      active = false;
      cleanup.forEach((remove) => remove());
    };
  }, []);
  const run: Run = async (label, action) => {
    if (inFlight.current) return false;
    inFlight.current = true;
    setBusy(label);
    setError("");
    setMessage("");
    try {
      const result = await action();
      await refresh().catch(() => {
        /* operation completed; keep latest connection state */
      });
      setMessage(typeof result === "string" ? result : "İşlem tamamlandı.");
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return false;
    } finally {
      setBusy("");
      inFlight.current = false;
    }
  };
  const operationBusy = Boolean(busy || state?.busy);
  const disabled = Boolean(
    operationBusy || connectionError || state?.recoveryIssue,
  );
  const running = state?.anyRunning ?? false;
  const installed = state?.packages.every((p) => p.installed) ?? false;
  const servicesInstalled =
    state?.packages
      .filter((p) => ["php", "mysql", "caddy"].includes(p.id))
      .every((p) => p.installed) ?? false;
  return (
    <Shell page={page} onPage={setPage}>
      <div className="workspace-toolbar">
        <div className="breadcrumb">
          <Monitor size={15} />
          <span>Yerel ortam</span>
          <ChevronRight size={13} />
          <strong>
            {
              {
                overview: "Genel bakış",
                packages: "Bileşenler",
                projects: "Projeler",
                logs: "Günlükler",
                settings: "Ayarlar",
              }[page]
            }
          </strong>
        </div>
        {desktop && (
          <div className="desktop-actions" aria-label="Masaüstü işlemleri">
            <button
              className="toolbar-button"
              title="Hızlı menü"
              onClick={() =>
                void run("Menü açılıyor…", () =>
                  call("desktop_action", { action: "menu" }),
                )
              }
            >
              <Ellipsis size={18} />
              <span>Hızlı menü</span>
            </button>
            <button
              className="icon-button"
              aria-label="Tepsiye küçült"
              title="Tepsiye küçült"
              onClick={() =>
                void run("Tepsiye küçültülüyor…", () =>
                  call("desktop_action", { action: "hide" }),
                )
              }
            >
              <PanelBottomClose size={17} />
            </button>
            <button
              className="icon-button"
              aria-label="F4Box'tan çık"
              title="F4Box'tan çık"
              disabled={operationBusy}
              onClick={() =>
                void call("desktop_action", { action: "exit" }).catch((e) =>
                  setError(String(e)),
                )
              }
            >
              <LogOut size={17} />
            </button>
          </div>
        )}
      </div>
      <header className="page-header">
        <div>
          <p className="eyebrow">
            {page === "overview"
              ? "KODUNUZ İÇİN HAZIR"
              : "F4BOX / ÇALIŞMA ALANI"}
          </p>
          <h1>{headings[page][0]}</h1>
          <p>{headings[page][1]}</p>
        </div>
        <button
          className={`button ${running ? "secondary" : "primary"} environment-button`}
          disabled={
            operationBusy ||
            Boolean(connectionError) ||
            (!running && (disabled || !servicesInstalled))
          }
          onClick={() =>
            void run(
              running ? "Ortam durduruluyor…" : "Ortam başlatılıyor…",
              () =>
                call("service", {
                  id: "all",
                  action: running ? "stop" : "start",
                }),
            )
          }
        >
          {running ? (
            <Square size={16} />
          ) : (
            <Play size={18} fill="currentColor" />
          )}
          {running ? "Ortamı durdur" : "Ortamı başlat"}
        </button>
      </header>
      <Notice
        error={error || connectionError}
        busy={busy}
        message={message}
        dismiss={() => {
          setError("");
          setMessage("");
        }}
      />
      {!state ? (
        <div className="loading-state">
          <RefreshCw className="spin" size={24} />
          <p>Ortam bilgileri alınıyor…</p>
          <button
            className="button secondary"
            onClick={() => void refresh().catch((e) => setError(String(e)))}
          >
            Yeniden dene
          </button>
        </div>
      ) : state.recoveryIssue ? (
        <>
          <section className="loading-state" role="alert">
            <h2>
              {state.restartRequired
                ? "İşlemler güvenli biçimde duraklatıldı"
                : "Yapılandırma kurtarma"}
            </h2>
            <p>{state.recoveryIssue}</p>
            {!state.restartRequired && (
              <>
                <p>
                  Son geçerli yedek, önceki ayarları ve proje listesini geri
                  getirir. Mevcut bozuk dosya ayrıca saklanır; proje ve
                  veritabanı dosyaları değiştirilmez.
                </p>
                <button
                  className="button primary"
                  disabled={operationBusy || Boolean(connectionError)}
                  onClick={() =>
                    void run("Yapılandırma kurtarılıyor…", () =>
                      call("recover_configuration"),
                    )
                  }
                >
                  Son geçerli yedeği geri yükle
                </button>
              </>
            )}
            <button
              className="button secondary"
              onClick={() =>
                void run("Veri klasörü açılıyor…", () => call("open_home"))
              }
            >
              Veri klasörünü aç
            </button>
          </section>
          <Logs />
        </>
      ) : (
        <>
          {page === "overview" && <EnvironmentSummary state={state} />}
          {page === "overview" && !installed ? (
            <div className="setup-banner">
              <div className="setup-icon">
                <Check size={19} />
              </div>
              <div>
                <h2>İlk projenize hazır olun</h2>
                <p>
                  Gerekli bileşenleri kurun, projenizi ekleyin ve çalışmaya
                  başlayın.
                </p>
              </div>
              <button
                className="button banner-button"
                disabled={disabled}
                onClick={() =>
                  void run(
                    "Bileşenler kuruluyor… İlerlemeyi günlüklerden takip edebilirsiniz.",
                    () => call("install", { id: "all" }),
                  )
                }
              >
                Bileşenleri kur
              </button>
            </div>
          ) : null}
          {page === "overview" || page === "packages" ? (
            <Packages
              packages={state.packages}
              phpVersions={state.phpVersions}
              busy={disabled}
              run={run}
              detailed={page === "packages"}
              pmaEnabled={state.settings.phpmyadmin.enabled}
              running={running}
            />
          ) : null}
          {page === "overview" || page === "projects" ? (
            <Projects
              projects={state.projects}
              phpVersions={state.phpVersions}
              serverError={error}
              anyRunning={running}
              clearError={() => setError("")}
              webRunning={state.packages.some(
                (p) => p.id === "caddy" && p.running,
              )}
              phpVersion={
                state.packages.find((p) => p.id === "php")?.version ?? ""
              }
              busy={disabled}
              run={run}
              webPort={state.settings.webPort}
              mysqlRunning={state.packages.some(
                (p) => p.id === "mysql" && p.running,
              )}
              home={state.settings.projectsDir || `${state.home}/www`}
            />
          ) : null}
          {page === "overview" ? <LogPreview logs={state.logs} /> : null}
          {page === "overview" ? (
            <button
              className="button secondary"
              onClick={() => setPage("settings")}
            >
              Kurulum gereksinimlerini denetle
            </button>
          ) : null}
          {page === "logs" ? <Logs /> : null}
          {page === "settings" ? (
            <Settings
              key={JSON.stringify(state.settings)}
              settings={state.settings}
              versions={state.phpVersions}
              home={state.home}
              busy={disabled}
              running={running}
              run={run}
            />
          ) : null}
        </>
      )}
    </Shell>
  );
}
