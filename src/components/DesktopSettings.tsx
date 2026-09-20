import { useEffect, useState } from "react";
import { call, desktop } from "../api";
import type { Run } from "../types";

type Preferences = { closeToTray: boolean; startMinimized: boolean };
type Status = {
  preferences: Preferences;
  autostart: boolean | null;
  issue: string | null;
  trayAvailable: boolean;
  quitting: boolean;
};

export default function DesktopSettings({
  busy,
  run,
}: {
  busy: boolean;
  run: Run;
}) {
  const [status, setStatus] = useState<Status | null>(null);
  const [draft, setDraft] = useState<{
    preferences: Preferences;
    autostart: boolean;
  } | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!desktop) return;
    let active = true,
      loading = false;
    const refresh = async () => {
      if (loading) return;
      loading = true;
      try {
        const next = await call<Status>("desktop_status");
        if (active) {
          setStatus(next);
          setError("");
        }
      } catch (error) {
        if (active) setError(String(error));
      } finally {
        loading = false;
      }
    };
    void refresh();
    const timer = setInterval(() => void refresh(), 3000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);
  const current =
    draft ??
    (status
      ? {
          preferences: status.preferences,
          autostart: status.autostart ?? false,
        }
      : null);
  const update = (key: keyof Preferences | "autostart", checked: boolean) => {
    if (!current) return;
    setDraft(
      key === "autostart"
        ? { ...current, autostart: checked }
        : {
            ...current,
            preferences: { ...current.preferences, [key]: checked },
          },
    );
  };
  return (
    <form
      className="settings-section"
      onSubmit={(event) => {
        event.preventDefault();
        if (!current) return;
        void run("Masaüstü tercihleri kaydediliyor…", async () => {
          await call("desktop_save", current);
          setStatus(await call<Status>("desktop_status"));
          setDraft(null);
          return "Masaüstü tercihleri uygulandı.";
        });
      }}
    >
      <h2>Windows ve sistem tepsisi</h2>
      <p className="section-note">
        Bu tercihler servisler çalışırken de değiştirilebilir. Tepsi simgesi
        saatin yanında veya gizli simgeler menüsünde görünür.
      </p>
      {!desktop && (
        <p className="section-note">
          Bu seçenekler masaüstü uygulamasında kullanılabilir.
        </p>
      )}
      {(error || status?.issue) && (
        <p role="alert" className="settings-feedback">
          {error || status?.issue}
        </p>
      )}
      {desktop && status && !status.trayAvailable && (
        <p role="alert">
          Sistem tepsisi kullanılamıyor. Pencere gizlenmez; kapatma düğmesi
          uygulamadan çıkar.
        </p>
      )}
      <fieldset
        className="settings-fields"
        disabled={
          !desktop ||
          !status ||
          status.autostart === null ||
          busy ||
          status.quitting ||
          Boolean(error)
        }
      >
        {(
          [
            ["autostart", "Windows oturumu açıldığında F4Box'ı çalıştır"],
            [
              "startMinimized",
              "Windows başlangıcında pencereyi açmadan tepside çalıştır",
            ],
            ["closeToTray", "Pencereyi kapatınca sistem tepsisine küçült"],
          ] as const
        ).map(([key, label]) => (
          <label className="setting-toggle" key={key}>
            <input
              type="checkbox"
              checked={
                key === "autostart"
                  ? (current?.autostart ?? false)
                  : (current?.preferences[key] ?? key === "closeToTray")
              }
              onChange={(e) => update(key, e.target.checked)}
            />
            <span>{label}</span>
          </label>
        ))}
        <p className="section-note">
          Tepsiye küçültmek servisleri durdurmaz. Tamamen kapatmak için tepsi
          menüsündeki Çıkış komutunu kullanın. PHP/MySQL'in otomatik başlaması
          aşağıdaki ortam seçeneğine bağlıdır.
        </p>
        <button className="button primary" type="submit" disabled={!draft}>
          Masaüstü tercihlerini kaydet
        </button>
        {draft && (
          <button
            type="button"
            className="button secondary"
            onClick={() => setDraft(null)}
          >
            Değişiklikleri iptal et
          </button>
        )}
      </fieldset>
    </form>
  );
}
