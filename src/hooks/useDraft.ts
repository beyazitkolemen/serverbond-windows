import { useUnsavedChanges } from "./useNavigationGuard";
import {
  useCallback,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";

/** A snapshot-backed draft with the original values used for guarded saves. */
export function useDraft<T>(
  source: T,
  label = "Ayar değişiklikleri",
): {
  values: T;
  setValues: Dispatch<SetStateAction<T>>;
  dirty: boolean;
  baseline: T;
  conflicted: boolean;
  reset: () => void;
} {
  const serialized = JSON.stringify(source);
  const [draft, setDraft] = useState(() => ({
    source: serialized,
    values: structuredClone(source),
    baseline: structuredClone(source),
  }));
  // Reconcile before committing children: a newly loaded clean form must not
  // briefly expose the old values with the new snapshot's enabled actions.
  if (draft.source !== serialized) {
    const current = JSON.stringify(draft.values);
    const follows = current === draft.source || current === serialized;
    setDraft({
      source: serialized,
      values: follows ? structuredClone(source) : draft.values,
      baseline: follows ? structuredClone(source) : draft.baseline,
    });
  }
  const setValues: Dispatch<SetStateAction<T>> = useCallback((next) => {
    setDraft((current) => ({
      ...current,
      values:
        typeof next === "function"
          ? (next as (value: T) => T)(current.values)
          : next,
    }));
  }, []);
  const reset = useCallback(() => {
    setDraft({
      source: serialized,
      values: structuredClone(source),
      baseline: structuredClone(source),
    });
  }, [serialized, source]);
  const dirty = JSON.stringify(draft.values) !== serialized;
  const conflicted = dirty && JSON.stringify(draft.baseline) !== serialized;
  useUnsavedChanges(dirty, label);
  return {
    values: draft.values,
    setValues,
    baseline: draft.baseline,
    dirty,
    conflicted,
    reset,
  };
}
