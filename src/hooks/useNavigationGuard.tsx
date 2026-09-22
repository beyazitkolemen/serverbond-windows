import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

type Navigate = (action: () => void, labels?: string[]) => void;
type Guard = {
  navigate: Navigate;
  register: (id: string, label: string | null) => void;
};
const Context = createContext<Guard>({
  navigate: (action) => action(),
  register: () => {},
});

/** Keep only dirty-state labels here; never persist form contents or credentials. */
export function NavigationGuard({ children }: { children: ReactNode }) {
  const dirty = useRef(new Map<string, string>());
  const [pending, setPending] = useState<{
    action: () => void;
    labels: string[];
  } | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const register = useCallback((id: string, label: string | null) => {
    if (label) dirty.current.set(id, label);
    else dirty.current.delete(id);
  }, []);
  const navigate: Navigate = useCallback((action, labels) => {
    const affected = [...new Set(dirty.current.values())].filter(
      (label) => !labels || labels.includes(label),
    );
    if (affected.length) {
      returnFocus.current =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null;
      setPending({ action, labels: affected });
    } else action();
  }, []);
  useEffect(() => {
    const beforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirty.current.size) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", beforeUnload);
    return () => window.removeEventListener("beforeunload", beforeUnload);
  }, []);
  useEffect(() => {
    if (pending && !dialog.current?.open) dialog.current?.showModal();
    if (!pending && returnFocus.current) {
      const target = returnFocus.current.isConnected
        ? returnFocus.current
        : document.getElementById("main-content");
      target?.focus();
      returnFocus.current = null;
    }
  }, [pending]);
  const context = useMemo(() => ({ navigate, register }), [navigate, register]);
  return (
    <Context.Provider value={context}>
      {children}
      {pending && (
        <dialog
          ref={dialog}
          className="modal unsaved-dialog"
          aria-labelledby="unsaved-title"
          aria-describedby="unsaved-description"
          onCancel={(event) => {
            event.preventDefault();
            setPending(null);
          }}
        >
          <div className="modal-header">
            <h2 id="unsaved-title">Kaydedilmemiş değişiklikler var</h2>
          </div>
          <p id="unsaved-description" className="dialog-copy">
            Devam ederseniz bu taslaklar kapanır. Kaydetmek için düzenlemeye
            dönün.
          </p>
          <ul className="unsaved-list">
            {pending.labels.map((label) => (
              <li key={label}>{label}</li>
            ))}
          </ul>
          <div className="modal-actions">
            <button
              type="button"
              className="button primary"
              autoFocus
              onClick={() => setPending(null)}
            >
              Düzenlemeye dön
            </button>
            <button
              type="button"
              className="button secondary"
              onClick={() => {
                const action = pending.action;
                setPending(null);
                action();
              }}
            >
              Kaydetmeden devam et
            </button>
          </div>
        </dialog>
      )}
    </Context.Provider>
  );
}

export function useNavigationGuard() {
  return useContext(Context).navigate;
}
export function useUnsavedChanges(dirty: boolean, label: string) {
  const { register } = useContext(Context);
  const id = useId();
  useEffect(() => {
    register(id, dirty ? label : null);
    return () => register(id, null);
  }, [dirty, id, label, register]);
}
