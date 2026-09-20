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
import {
  ComponentId,
  CORE_COMPONENTS,
  Page,
  type Page as AppPage,
} from "./domain";
import { snapshotRepository } from "./repositories";
import { environmentService, packagesService } from "./services";
import type { Snapshot, Run } from "./types";
import { Shell, Notice } from "./components/Shell";
import Packages from "./components/Packages";
import Projects from "./components/Projects";
import Logs, { LogPreview } from "./components/Logs";
import Settings from "./components/Settings";
import Services from "./components/Services";
import EnvironmentSummary from "./components/EnvironmentSummary";
import { checkForAppUpdate, type UpdateInfo } from "./updates";

const headings: Record<AppPage, [string, string]> = {
  [Page.Overview]: [
    "Genel bakış",
    "Bu makinedeki üretim servisleri, projeler ve kayıtlar.",
  ],
  [Page.Packages]: ["Bileşenler", "PHP, MySQL ve web sunucusu paketleri."],
  [Page.Projects]: ["Projeler", "Bu makinede çalışan Laravel uygulamaları."],
  [Page.Logs]: ["Günlükler", "Kurulum, proje ve servis kayıtları."],
  [Page.Services]: [
    "Hizmetler",
    "phpMyAdmin, e-posta, PostgreSQL, Redis, GitHub ve tünel.",
  ],
  [Page.Settings]: ["Ayarlar", "Çalışma alanı, portlar ve Windows tercihleri."],
};

export default function App() {
  const [page, setPage] = useState<AppPage>(Page.Overview);
  const [state, setState] = useState<Snapshot | null>(null);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [connectionError, setConnectionError] = useState("");
  const [message, setMessage] = useState("");
  const [appUpdate, setAppUpdate] = useState<UpdateInfo | null>(null);
  const [openUpdates, setOpenUpdates] = useState(0);
  const [leaveOpen, setLeaveOpen] = useState(false);
  const inFlight = useRef(false);
  const requestNumber = useRef(0);
  const refresh = useCallback(async () => {
    const request = ++requestNumber.current;
    try {
      const next = await snapshotRepository.get();
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
        setPage(target as AppPage);
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
      const updates = await listen("desktop:check-update", () => {
        if (active) {
          setPage(Page.Settings);
          setOpenUpdates((n) => n + 1);
        }
      });
      if (!active) {
        updates();
        return;
      }
      cleanup.push(updates);
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
  useEffect(() => {
    if (!desktop) return;
    let active = true;
    void checkForAppUpdate()
      .then((update) => {
        if (active) setAppUpdate(update);
      })
      .catch(() => {
        /* silent launch check; Ayarlar → Güncellemeler shows errors */
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    if (!message || error) return;
    const timer = window.setTimeout(() => setMessage(""), 4000);
    return () => clearTimeout(timer);
  }, [message, error]);
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
      .filter((p) => (CORE_COMPONENTS as readonly string[]).includes(p.id))
      .every((p) => p.installed) ?? false;
  return (
    <Shell
      page={page}
      onPage={setPage}
      updateAvailable={Boolean(appUpdate)}
      onOpenUpdates={() => {
        setPage(Page.Settings);
        setOpenUpdates((n) => n + 1);
      }}
      toolbar={
        <div className="workspace-toolbar">
          <div className="workspace-toolbar-inner">
            <div className="breadcrumb">
              <Monitor size={16} />
              <span>Yerel ortam</span>
              <ChevronRight size={16} />
              <strong>
                {
                  {
                    overview: "Genel bakış",
                    packages: "Bileşenler",
                    projects: "Projeler",
                    logs: "Günlükler",
                    services: "Hizmetler",
                    settings: "Ayarlar",
                  }[page]
                }
              </strong>
            </div>
            {desktop && (
              <div className="desktop-actions" aria-label="Masaüstü işlemleri">
                <button
                  className="toolbar-button"
                  title="Uygulama menüsü"
                  onClick={() =>
                    void run("Menü açılıyor…", () =>
                      call("desktop_action", { action: "menu" }),
                    )
                  }
                >
                  <Ellipsis size={18} />
                  <span>Uygulama menüsü</span>
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
                  <PanelBottomClose size={16} />
                </button>
                <button
                  className="icon-button"
                  aria-label="ServerBond'tan çık"
                  title="ServerBond'tan çık"
                  disabled={operationBusy}
                  onClick={() => {
                    if (running) {
                      setLeaveOpen(true);
                      return;
                    }
                    void call("desktop_action", { action: "exit" }).catch((e) =>
                      setError(String(e)),
                    );
                  }}
                >
                  <LogOut size={16} />
                </button>
              </div>
            )}
          </div>
        </div>
      }
    >
      <header className="page-header">
        <div>
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
              () => environmentService.toggle(running),
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
          {appUpdate ? (
            <div className="setup-banner update-banner">
              <div className="setup-icon">
                <RefreshCw size={16} />
              </div>
              <div>
                <h2>ServerBond {appUpdate.version} yayımlandı</h2>
                <p>
                  Kurulu sürüm v{appUpdate.currentVersion}. Güncelleme GitHub
                  üzerinden indirilir; onayınız olmadan kurulmaz.
                </p>
              </div>
              <button
                className="button banner-button"
                onClick={() => {
                  setPage(Page.Settings);
                  setOpenUpdates((n) => n + 1);
                }}
              >
                Güncellemeyi gör
              </button>
            </div>
          ) : null}
          {page === Page.Overview && (
            <EnvironmentSummary state={state} onPage={setPage} />
          )}
          {page === Page.Overview && !installed ? (
            <div className="setup-banner">
              <div className="setup-icon">
                <Check size={16} />
              </div>
              <ol className="setup-steps">
                <li>
                  <strong>1. Bileşenler</strong>
                  <span>PHP, MySQL ve Caddy</span>
                </li>
                <li>
                  <strong>2. Proje</strong>
                  <span>Mevcut klasör veya yeni Laravel</span>
                </li>
                <li>
                  <strong>3. Ortam</strong>
                  <span>Servisleri buradan başlatın</span>
                </li>
              </ol>
              <button
                className="button banner-button"
                disabled={disabled}
                onClick={() =>
                  void run(
                    "Bileşenler kuruluyor… İlerlemeyi günlüklerden takip edebilirsiniz.",
                    () => packagesService.install("all"),
                  )
                }
              >
                Bileşenleri kur
              </button>
            </div>
          ) : null}
          {page === Page.Overview || page === Page.Packages ? (
            <Packages
              packages={state.packages}
              phpVersions={state.phpVersions}
              busy={disabled}
              run={run}
              detailed={page === Page.Packages}
              pmaEnabled={state.settings.phpmyadmin.enabled}
              running={running}
              onOpen={
                page === Page.Overview
                  ? () => setPage(Page.Packages)
                  : undefined
              }
            />
          ) : null}
          {page === Page.Overview || page === Page.Projects ? (
            <Projects
              projects={state.projects}
              phpVersions={state.phpVersions}
              serverError={error}
              anyRunning={running}
              clearError={() => setError("")}
              webRunning={state.packages.some(
                (p) => p.id === ComponentId.Caddy && p.running,
              )}
              phpVersion={
                state.packages.find((p) => p.id === ComponentId.Php)?.version ??
                ""
              }
              busy={disabled}
              run={run}
              webPort={state.settings.webPort}
              https={state.settings.web.https}
              httpsPort={state.settings.web.httpsPort}
              mysqlRunning={state.packages.some(
                (p) => p.id === ComponentId.Mysql && p.running,
              )}
              home={state.settings.projectsDir || `${state.home}/projects`}
              hostPattern={state.settings.web.hostPattern}
              github={state.github}
              compact={page === Page.Overview}
              onOpen={
                page === Page.Overview
                  ? () => setPage(Page.Projects)
                  : undefined
              }
            />
          ) : null}
          {page === Page.Overview ? (
            <LogPreview logs={state.logs} onOpen={() => setPage(Page.Logs)} />
          ) : null}
          {page === Page.Logs ? <Logs /> : null}
          {page === Page.Services ? (
            <Services
              key={JSON.stringify(state.settings)}
              settings={state.settings}
              busy={disabled}
              running={running}
              run={run}
              tunnel={state.tunnel}
              mail={state.mail}
              postgres={state.postgres}
              redis={state.redis}
              github={state.github}
              phpmyadmin={state.packages.find(
                (p) => p.id === ComponentId.PhpMyAdmin,
              )}
            />
          ) : null}
          {page === Page.Settings ? (
            <Settings
              key={JSON.stringify(state.settings)}
              settings={state.settings}
              versions={state.phpVersions}
              phpVersion={
                state.packages.find((p) => p.id === ComponentId.Php)?.version ??
                ""
              }
              mysqlRunning={state.packages.some(
                (p) => p.id === ComponentId.Mysql && p.running,
              )}
              home={state.home}
              busy={disabled}
              running={running}
              run={run}
              node={state.node}
              permissions={state.permissions}
              appUpdate={appUpdate}
              onAppUpdate={setAppUpdate}
              openUpdates={openUpdates}
            />
          ) : null}
        </>
      )}
      {leaveOpen ? (
        <LeaveDialog
          close={() => setLeaveOpen(false)}
          confirm={() => {
            setLeaveOpen(false);
            void call("desktop_action", { action: "exit" }).catch((e) =>
              setError(String(e)),
            );
          }}
        />
      ) : null}
    </Shell>
  );
}

function LeaveDialog({
  close,
  confirm,
}: {
  close: () => void;
  confirm: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="modal"
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
      aria-labelledby="leave-title"
    >
      <div className="modal-header">
        <h2 id="leave-title">ServerBond kapatılsın mı?</h2>
      </div>
      <p className="dialog-copy">
        Ortam çalışıyor. Çıkış PHP, MySQL ve web sunucusunu durdurur.
      </p>
      <div className="modal-actions">
        <button type="button" className="button secondary" onClick={close}>
          Vazgeç
        </button>
        <button
          type="button"
          className="button danger-button"
          onClick={confirm}
        >
          Çık
        </button>
      </div>
    </dialog>
  );
}
