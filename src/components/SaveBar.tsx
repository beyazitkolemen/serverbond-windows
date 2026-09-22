import { CircleAlert } from "lucide-react";

export default function SaveBar({
  dirty,
  busy,
  canSave,
  onReset,
  label = "Değişiklikleri kaydet",
  blocked,
}: {
  dirty: boolean;
  busy: boolean;
  canSave: boolean;
  onReset: () => void;
  label?: string;
  blocked?: string;
}) {
  if (!dirty) return null;
  return (
    <div className="settings-save save-bar">
      <span role="status">
        <CircleAlert size={16} aria-hidden />
        {blocked || "Kaydedilmemiş değişiklikler"}
      </span>
      <div className="save-bar-actions">
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={(event) => {
            onReset();
            event.currentTarget.form?.dispatchEvent(new Event("reset"));
          }}
        >
          Vazgeç
        </button>
        <button
          type="submit"
          className="button primary"
          disabled={!canSave || busy}
        >
          {label}
        </button>
      </div>
    </div>
  );
}
