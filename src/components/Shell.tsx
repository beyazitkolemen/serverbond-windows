import { Box, Monitor, LoaderCircle, X } from "lucide-react";
import { type Page as AppPage } from "../domain";
import type { ReactNode } from "react";
import { desktop } from "../api";
import { APP_VERSION } from "../version";
import ErrorBoundary from "./ErrorBoundary";

import {
  workspaceNav,
  environmentNav,
  manageNav,
  pageLabel,
  type NavEntry,
} from "./navigation";

function NavButtons({
  items,
  page,
  onPage,
}: {
  items: readonly NavEntry[];
  page: AppPage;
  onPage: (p: AppPage) => void;
}) {
  return (
    <>
      {items.map(({ id, title, icon: Icon }) => (
        <button
          key={id}
          type="button"
          aria-label={title}
          title={title}
          className={`nav-item ${page === id ? "selected" : ""}`}
          aria-current={page === id ? "page" : undefined}
          onClick={() => onPage(id)}
        >
          <Icon size={18} strokeWidth={1.75} />
          <span>{title}</span>
        </button>
      ))}
    </>
  );
}

function NavGroup({
  label,
  items,
  page,
  onPage,
}: {
  label: string;
  items: readonly NavEntry[];
  page: AppPage;
  onPage: (p: AppPage) => void;
}) {
  return (
    <div className="sidebar-nav-group">
      <p className="nav-caption">{label}</p>
      <nav aria-label={label}>
        <NavButtons items={items} page={page} onPage={onPage} />
      </nav>
    </div>
  );
}

export function Shell({
  page,
  onPage,
  children,
  toolbar,
  updateAvailable = false,
  onOpenUpdates,
}: {
  page: AppPage;
  onPage: (p: AppPage) => void;
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
            <Box size={22} strokeWidth={1.7} />
          </div>
          <div className="brand-copy">
            <span>ServerBond</span>
            <p>Çalışma alanı</p>
          </div>
        </div>
        <div className="sidebar-nav">
          <NavGroup
            label="Çalışma alanı"
            items={workspaceNav}
            page={page}
            onPage={onPage}
          />
          <NavGroup
            label="Ortam"
            items={environmentNav}
            page={page}
            onPage={onPage}
          />
        </div>
        <div className="sidebar-dock">
          <NavGroup
            label="Yönetim"
            items={manageNav}
            page={page}
            onPage={onPage}
          />
          <div className="sidebar-status">
            <span className="local-label">
              <span className="status-dot green" />
              Bu bilgisayarda
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
            <Monitor size={14} />
            <span>Windows x64</span>
            <span className="build-version">v{APP_VERSION}</span>
          </div>
        </div>
      </aside>
      <div className="workspace">
        {toolbar}
        <main id="main-content" tabIndex={-1}>
          <ErrorBoundary scope={pageLabel(page)} resetKey={page}>
            {children}
          </ErrorBoundary>
        </main>
        <footer>
          <div className="content-rail">
            {desktop ? `ServerBond v${APP_VERSION}` : "Salt okunur önizleme"}
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
