import {
  Box,
  Folder,
  Home,
  ScrollText,
  Settings,
  Monitor,
  LoaderCircle,
  X,
  ChevronRight,
} from "lucide-react";
import type { Page } from "../types";
import type { ReactNode } from "react";
import { desktop } from "../api";
import { APP_VERSION } from "../version";

const navigation = [
  { id: "overview", title: "Genel bakış", icon: Home },
  { id: "packages", title: "Bileşenler", icon: Box },
  { id: "projects", title: "Projeler", icon: Folder },
  { id: "logs", title: "Günlükler", icon: ScrollText },
  { id: "settings", title: "Ayarlar", icon: Settings },
] as const;

export function Shell({
  page,
  onPage,
  children,
  toolbar,
  updateAvailable = false,
  onOpenUpdates,
}: {
  page: Page;
  onPage: (p: Page) => void;
  children: ReactNode;
  toolbar?: ReactNode;
  updateAvailable?: boolean;
  onOpenUpdates?: () => void;
}) {
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        İçeriğe geç
      </a>
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <Box size={24} strokeWidth={1.7} />
          </div>
          <div className="brand-copy">
            <span>F4Box</span>
            <p>Windows Laravel üretimi</p>
          </div>
        </div>
        <p className="nav-caption">ÇALIŞMA ALANI</p>
        <nav aria-label="Ana menü">
          {navigation.map(({ id, title, icon: Icon }) => (
            <button
              key={id}
              aria-label={title}
              className={`nav-item ${page === id ? "selected" : ""}`}
              aria-current={page === id ? "page" : undefined}
              onClick={() => onPage(id)}
            >
              <Icon size={20} />
              <span>{title}</span>
              {page === id && (
                <ChevronRight className="nav-chevron" size={16} />
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-note">
          <span className="local-label">
            <span className="status-dot green" /> Bu bilgisayarda çalışır
          </span>
          {updateAvailable && onOpenUpdates ? (
            <button
              type="button"
              className="sidebar-update"
              onClick={onOpenUpdates}
            >
              Yeni sürüm var
            </button>
          ) : null}
        </div>
        <div className="sidebar-footer">
          <Monitor size={16} />
          <span>Windows x64</span>
          <span className="build-version">v{APP_VERSION}</span>
        </div>
      </aside>
      <div className="workspace">
        {toolbar}
        <main id="main-content" tabIndex={-1}>
          {children}
        </main>
        <footer>
          <div className="content-rail">
            {desktop
              ? `F4Box v${APP_VERSION}`
              : "Tarayıcı önizlemesi · Kurulum için masaüstü uygulamasını açın"}
          </div>
        </footer>
      </div>
    </div>
  );
}

export function Notice({
  error,
  busy,
  message,
  dismiss,
}: {
  error: string;
  busy: string;
  message: string;
  dismiss: () => void;
}) {
  if (!error && !busy && !message) return null;
  return (
    <div
      className={`notice ${error ? "error" : message && !busy ? "success" : ""}`}
      role={error ? "alert" : "status"}
    >
      {busy && !error ? <LoaderCircle className="spin" size={20} /> : null}
      <span>{error || busy || message}</span>
      {!busy ? (
        <button
          className="icon-button"
          aria-label="Bildirimi kapat"
          onClick={dismiss}
        >
          <X size={18} />
        </button>
      ) : null}
    </div>
  );
}
