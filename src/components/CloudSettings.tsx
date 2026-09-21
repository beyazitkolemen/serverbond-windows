import { useEffect, useState } from "react";
import { desktop } from "../api";
import { cloudService, type CloudStatus } from "../services/cloud";

export default function CloudSettings() {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  const [url, setUrl] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const value = await cloudService.status();
        if (!disposed) {
          setStatus(value);
          if (value.url) setUrl((current) => current || value.url || "");
        }
      } catch {
        if (!disposed) setError("Cloud bağlantı durumu okunamadı.");
      } finally {
        if (!disposed) timer = setTimeout(() => void refresh(), 5000);
      }
    };
    void refresh();
    return () => {
      disposed = true;
      clearTimeout(timer);
    };
  }, []);
  const act = async (action: () => Promise<void>) => {
    setBusy(true);
    setError("");
    try {
      await action();
      setCode("");
      setStatus(await cloudService.status());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="settings-section">
      <h2>ServerBond Cloud</h2>
      <p className="muted">
        Cloud panelinden bir bağlantı kodu oluşturun ve bu cihazı hesabınıza
        bağlayın. Uygulama açıkken servislerinizi ve projelerinizi uzaktan
        yönetebilirsiniz.
      </p>
      {!desktop && (
        <p role="status">
          Bağlantı yalnızca Windows uygulamasında kullanılabilir.
        </p>
      )}
      {(error || status?.error) && <p role="alert">{error || status?.error}</p>}
      {status?.paired ? (
        <div>
          <p>
            <strong>{status.name}</strong> · {status.account}
          </p>
          <p>Cloud adresi: {status.url}</p>
          <p role="status">
            Son bağlantı:{" "}
            {status.lastContact
              ? new Date(status.lastContact).toLocaleString("tr-TR")
              : "Bağlantı bekleniyor"}
          </p>
          <button
            className="button secondary"
            disabled={busy}
            onClick={() => void act(cloudService.disconnect)}
          >
            Bağlantıyı kaldır
          </button>
        </div>
      ) : (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void act(() => cloudService.pair(url, code));
          }}
        >
          <label>
            Cloud adresi
            <input
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              required
              placeholder="https://cloud.example.com"
              disabled={busy || !desktop}
            />
          </label>
          <label>
            Bağlantı kodu
            <input
              value={code}
              onChange={(e) => setCode(e.target.value)}
              required
              minLength={32}
              maxLength={32}
              autoComplete="off"
              spellCheck={false}
              disabled={busy || !desktop}
            />
          </label>
          <button className="button" disabled={busy || !desktop || !status}>
            {busy ? "Bağlanıyor…" : "Bağlan"}
          </button>
          {status?.url && (
            <button
              type="button"
              className="button secondary"
              disabled={busy || !desktop}
              onClick={() => void act(cloudService.disconnect)}
            >
              Kayıtlı bağlantıyı kaldır
            </button>
          )}
        </form>
      )}
    </section>
  );
}
