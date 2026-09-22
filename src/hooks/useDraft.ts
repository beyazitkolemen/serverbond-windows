import { useUnsavedChanges } from "./useNavigationGuard";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";

/**
 * A local editable copy of a value that arrives from the Rust snapshot.
 *
 * While the user has unsaved edits the draft is left alone. Once the draft is
 * clean (fresh, saved, or reset) it follows the source again, so a change made
 * elsewhere — another pane, the CLI, a save that normalised the input — shows
 * up without remounting the form and without wiping what the user typed.
 */
export function useDraft<T>(
  source: T,
  label = "Ayar değişiklikleri",
): {
  values: T;
  setValues: Dispatch<SetStateAction<T>>;
  dirty: boolean;
  reset: () => void;
} {
  const serialized = JSON.stringify(source);
  const [values, setValues] = useState(() => structuredClone(source));
  const dirty = JSON.stringify(values) !== serialized;
  // Whether the draft matched the previous source; read by the sync effect
  // so it can tell "user edited" apart from "source moved on".
  const wasDirty = useRef(false);
  const previous = useRef(serialized);

  useEffect(() => {
    if (previous.current === serialized) {
      wasDirty.current = dirty;
      return;
    }
    previous.current = serialized;
    if (!wasDirty.current) {
      setValues(structuredClone(source));
      wasDirty.current = false;
    }
    // `dirty` is intentionally read via the ref: the effect must react to the
    // source moving, not to every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [serialized, source]);

  useEffect(() => {
    wasDirty.current = dirty;
  }, [dirty]);

  const reset = useCallback(() => {
    setValues(structuredClone(source));
  }, [source]);

  useUnsavedChanges(dirty, label);
  return { values, setValues, dirty, reset };
}
