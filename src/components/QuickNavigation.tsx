import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowUpRight, Search, X } from "lucide-react";
import type { Page } from "../domain";
import { searchText } from "../search";
import { navigation } from "./navigation";

export default function QuickNavigation({
  onPage,
}: {
  onPage: (page: Page) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const results = navigation.filter((item) =>
    searchText(item.title).includes(searchText(query)),
  );
  const current = Math.min(active, Math.max(0, results.length - 1));
  const show = useCallback(() => {
    setQuery("");
    setActive(0);
    setOpen(true);
  }, []);
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      if (
        (event.ctrlKey || event.metaKey) &&
        event.key.toLowerCase() === "k" &&
        !event.repeat
      ) {
        if (document.querySelector("dialog[open]") && !dialog.current?.open)
          return;
        event.preventDefault();
        show();
      }
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, [show]);
  useEffect(() => {
    if (open) dialog.current?.showModal();
    else dialog.current?.close();
  }, [open]);
  const choose = (page: Page) => {
    onPage(page);
    setOpen(false);
  };
  return (
    <>
      <button
        type="button"
        className="quick-nav-trigger"
        onClick={show}
        aria-label="Sayfaya git"
        aria-haspopup="dialog"
        aria-keyshortcuts="Control+k Meta+k"
      >
        <Search size={16} />
        <span>Sayfaya git</span>
        <kbd>Ctrl K</kbd>
      </button>
      <dialog
        ref={dialog}
        className="quick-nav-dialog"
        aria-label="Hızlı gezinme"
        onClose={() => setOpen(false)}
        onClick={(event) => {
          if (event.target === event.currentTarget) setOpen(false);
        }}
      >
        <div className="quick-nav-content">
          <div className="quick-nav-input">
            <Search size={20} aria-hidden="true" />
            <input
              autoFocus
              role="combobox"
              aria-label="Sayfa ara"
              aria-expanded="true"
              aria-autocomplete="list"
              aria-controls="quick-nav-results"
              aria-activedescendant={
                results.length ? `quick-nav-${results[current].id}` : undefined
              }
              placeholder="Nereye gitmek istersiniz?"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setActive(0);
              }}
              onKeyDown={(event) => {
                if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                  event.preventDefault();
                  setActive(
                    Math.max(
                      0,
                      Math.min(
                        results.length - 1,
                        current + (event.key === "ArrowDown" ? 1 : -1),
                      ),
                    ),
                  );
                } else if (event.key === "Enter" && results[current]) {
                  event.preventDefault();
                  choose(results[current].id);
                }
              }}
            />
            <button
              className="icon-button"
              type="button"
              aria-label="Hızlı gezinmeyi kapat"
              onClick={() => setOpen(false)}
            >
              <X size={18} />
            </button>
          </div>
          <div
            className="quick-nav-results"
            id="quick-nav-results"
            role="listbox"
            aria-label="Sayfalar"
          >
            {results.map(({ id, title, icon: Icon }, index) => (
              <button
                id={`quick-nav-${id}`}
                key={id}
                type="button"
                role="option"
                aria-selected={index === current}
                tabIndex={-1}
                onMouseMove={() => setActive(index)}
                onClick={() => choose(id)}
              >
                <Icon size={19} />
                <span>{title}</span>
                <ArrowUpRight size={16} />
              </button>
            ))}
          </div>
          {!results.length ? (
            <p className="search-empty" role="status">
              Eşleşen sayfa yok.
            </p>
          ) : null}
          <div className="quick-nav-footer">
            <span>
              <kbd>↑</kbd> <kbd>↓</kbd> Seç
            </span>
            <span>
              <kbd>Enter</kbd> Aç
            </span>
            <span>
              <kbd>Esc</kbd> Kapat
            </span>
          </div>
        </div>
      </dialog>
    </>
  );
}
