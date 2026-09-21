import { useEffect, useState } from "react";

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "serverbond.theme";
const query = "(prefers-color-scheme: dark)";

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
 * light or dark. Stored in the WebView's localStorage; the Rust side has no
 * say in colours.
 */
export function useTheme(): {
  preference: ThemePreference;
  setPreference: (next: ThemePreference) => void;
  resolved: "light" | "dark";
} {
  const [preference, setPreferenceState] = useState<ThemePreference>(read);
  const [resolved, setResolved] = useState(() => resolve(preference));
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
  const setPreference = (next: ThemePreference) => {
    try {
      if (next === "system") window.localStorage.removeItem(STORAGE_KEY);
      else window.localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* private mode or blocked storage: the choice lasts for this session */
    }
    setPreferenceState(next);
  };
  return { preference, setPreference, resolved };
}
