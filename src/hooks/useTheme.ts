import { useEffect, useState } from "react";
import { call, desktop } from "../api";
import { listen } from "@tauri-apps/api/event";

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "serverbond.theme";
const query = "(prefers-color-scheme: dark)";
const changed = "serverbond:theme";

function store(preference: ThemePreference) {
  try {
    window.localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    /* Native preference remains persisted. */
  }
  document.documentElement.dataset.theme = resolve(preference);
  window.dispatchEvent(new CustomEvent(changed, { detail: preference }));
}

function read(): ThemePreference {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return stored === "light" || stored === "dark" ? stored : "system";
  } catch {
    return "system";
  }
}

function resolve(preference: ThemePreference): "light" | "dark" {
  if (preference !== "system") return preference;
  return window.matchMedia?.(query).matches ? "dark" : "light";
}

/** Apply the preference to `<html data-theme>` before React paints. */
export function applyStoredTheme() {
  document.documentElement.dataset.theme = resolve(read());
}

/**
 * Appearance preference: follows Windows by default and can be pinned to
 * light or dark. Desktop preferences persist in Rust and synchronize with
 * HTTP callers. Browser previews keep using localStorage.
 */
export function useTheme(): {
  preference: ThemePreference;
  setPreference: (next: ThemePreference) => void;
  resolved: "light" | "dark";
  error: string;
  busy: boolean;
} {
  const [preference, setPreferenceState] = useState<ThemePreference>(read);
  const [resolved, setResolved] = useState(() => resolve(preference));
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    const update = (event: Event) =>
      setPreferenceState((event as CustomEvent<ThemePreference>).detail);
    window.addEventListener(changed, update);
    if (desktop) {
      void (async () => {
        const remove = await listen<ThemePreference>(
          "desktop:theme",
          (event) => {
            if (active) store(event.payload);
          },
        );
        if (!active) {
          remove();
          return;
        }
        unlisten = remove;
        const theme = await call<ThemePreference>("appearance_save", {
          theme: read(),
          initializeOnly: true,
        });
        if (active) store(theme);
      })().catch((error) => {
        if (active) setError(String(error));
      });
    }
    return () => {
      active = false;
      unlisten?.();
      window.removeEventListener(changed, update);
    };
  }, []);
  useEffect(() => {
    const media = window.matchMedia?.(query);
    const update = () => {
      const next = resolve(preference);
      setResolved(next);
      document.documentElement.dataset.theme = next;
    };
    update();
    media?.addEventListener("change", update);
    return () => media?.removeEventListener("change", update);
  }, [preference]);
  const setPreference = async (next: ThemePreference) => {
    setBusy(true);
    setError("");
    try {
      const saved = desktop
        ? await call<ThemePreference>("appearance_save", {
            theme: next,
            initializeOnly: false,
          })
        : next;
      store(saved);
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  };
  return { preference, setPreference, resolved, error, busy };
}
