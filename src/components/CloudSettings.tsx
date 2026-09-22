import { useUnsavedChanges } from "../hooks/useNavigationGuard";
import { useEffect, useState } from "react";
import { Cloud, Radio, ShieldCheck } from "lucide-react";
import { desktop } from "../api";
import { useDraft } from "../hooks/useDraft";
import { cloudService, type CloudStatus } from "../services/cloud";

const connectionLabels: Record<CloudStatus["connection"], string> = {
  disconnected: "Eşleştirme bekleniyor",
  connecting: "Sokete bağlanıyor",
  connected: "Soket bağlı",
  retrying: "Yeniden bağlanıyor",
  revoked: "Yeniden eşleştirin",
};

export default function CloudSettings() {
  const [status, setStatus] = useState<CloudStatus | null>(null);
  const { values: url, setValues: setUrl } = useDraft(
    status?.url ?? status?.defaultUrl ?? "",
    "Cloud eşleştirmesi",
  );
  const [code, setCode] = useState("");
  useUnsavedChanges(Boolean(code.trim()), "Cloud eşleştirmesi");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [statusError, setStatusError] = useState("");
  useEffect(() => {
    let disposed = false;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const value = await cloudService.status();
        if (!disposed) {
          setStatus(value);
          setStatusError("");
        }
      } catch {
        if (!disposed)
          setStatusError(
            "Cloud bağlantı durumu okunamadı. Tekrar denetleniyor.",
          );
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
      setStatusError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };
  const connection = status?.connection ?? "disconnected";
  const locked = busy || !desktop || !status;
  const alert = error || statusError || status?.error;
  return (
    <section className="settings-section cloud-settings">
      <div className="cloud-heading">
        <div className="cloud-title">
          <Cloud size={24} aria-hidden="true" />
          <div>
            <h2>Cloud bağlantısı</h2>
            <p className="muted">
              Cihazınızı ve projelerinizi uzaktan yönetin.
            </p>
          </div>
        </div>
        <span
          className="cloud-status"
          data-state={statusError ? "retrying" : connection}
          role="status"
        >
          <span aria-hidden="true" />
          {statusError
            ? "Durum okunamıyor"
            : status
              ? connectionLabels[connection]
              : "Durum denetleniyor…"}
        </span>
      </div>
      {!desktop && (
        <p className="cloud-notice" role="status">
          Bağlantı yalnızca Windows uygulamasında kullanılabilir.
        </p>
      )}
      {alert && (
        <p className="cloud-error" role="alert">
          {alert}
        </p>
      )}
      {status?.paired ? (
        <>
          <dl className="cloud-details">
            <div>
              <dt>Cihaz</dt>
              <dd>{status.name || "Bu cihaz"}</dd>
            </div>
            <div>
              <dt>Hesap</dt>
              <dd>{status.account || "—"}</dd>
            </div>
            <div>
              <dt>Cloud sunucusu</dt>
              <dd>{status.url}</dd>
            </div>
            <div>
              <dt>Soket adresi · otomatik</dt>
              <dd>{status.socketEndpoint || "Cloud’dan alınıyor…"}</dd>
            </div>
            <div>
              <dt>Son başarılı iletişim</dt>
              <dd>
                {status.lastContact
                  ? new Date(status.lastContact).toLocaleString("tr-TR")
                  : "İlk bağlantı bekleniyor"}
              </dd>
            </div>
            <div>
              <dt>Yeniden bağlantı</dt>
              <dd>Uygulama açıkken otomatik</dd>
            </div>
          </dl>
          <p className="cloud-notice">
            <Radio size={18} aria-hidden="true" /> Bağlantı kesildiğinde
            otomatik tekrar denenir. Uygulamayı yeniden açtığınızda bağlantı
            kodu gerekmez.
          </p>
          <div className="cloud-actions">
            <button
              className="button secondary"
              disabled={locked}
              onClick={() => void act(cloudService.disconnect)}
            >
              {busy ? "Bağlantı kaldırılıyor…" : "Eşleştirmeyi kaldır"}
            </button>
            <span className="muted">
              Bu cihazın Cloud üzerinden yönetimini kapatır.
            </span>
          </div>
        </>
      ) : (
        <form
          className="cloud-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (!locked) void act(() => cloudService.pair(url, code));
          }}
        >
          <div className="cloud-server">
            <div className="cloud-field-heading">
              <label htmlFor="cloud-url">Cloud sunucusu</label>
              <span className="muted">
                {url.replace(/\/$/, "") === status?.defaultUrl
                  ? "Varsayılan sunucu"
                  : "Özel sunucu"}
              </span>
            </div>
            <input
              id="cloud-url"
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              required
              disabled={locked}
              spellCheck={false}
              autoCapitalize="none"
              aria-describedby="cloud-server-help"
            />
            <p id="cloud-server-help" className="muted">
              Forge sunucusu hazır gelir. Soket adresi ve bağlantı ayarları
              Cloud’dan otomatik alınır.
            </p>
            {status && url.replace(/\/$/, "") !== status.defaultUrl && (
              <button
                type="button"
                className="button secondary small"
                disabled={locked}
                onClick={() => setUrl(status.defaultUrl)}
              >
                Varsayılan sunucuyu kullan
              </button>
            )}
          </div>
          <div className="cloud-pairing">
            <label htmlFor="cloud-code">Bağlantı kodu</label>
            <p className="muted" id="cloud-code-help">
              Cloud panelinde cihaz ekleyerek oluşturduğunuz 32 karakterli kodu
              yapıştırın. Bu işlem yalnızca ilk bağlantıda gerekir.
            </p>
            <input
              id="cloud-code"
              className="cloud-code"
              value={code}
              onChange={(e) => setCode(e.target.value.trim().toUpperCase())}
              required
              minLength={32}
              maxLength={32}
              pattern="[A-Fa-f0-9]{32}"
              title="32 karakterli bağlantı kodunu girin."
              autoComplete="off"
              autoCapitalize="characters"
              spellCheck={false}
              disabled={locked}
              aria-describedby="cloud-code-help"
              placeholder="32 karakterli bağlantı kodu"
            />
          </div>
          <div className="cloud-actions">
            <button
              className="button primary"
              disabled={locked || !/^[A-F0-9]{32}$/.test(code)}
            >
              {busy ? "Eşleştiriliyor…" : "Cihazı bağla"}
            </button>
            {status?.url && (
              <button
                type="button"
                className="button secondary"
                disabled={locked}
                onClick={() => void act(cloudService.disconnect)}
              >
                Kayıtlı eşleştirmeyi temizle
              </button>
            )}
          </div>
          <p className="cloud-notice">
            <ShieldCheck size={18} aria-hidden="true" /> Eşleştirme bu Windows
            kullanıcısına kaydedilir. Sonraki açılışlarda güvenli bağlantı
            otomatik kurulur.
          </p>
        </form>
      )}
    </section>
  );
}
