import { useUnsavedChanges } from "../hooks/useNavigationGuard";
import { useEffect, useRef, useState } from "react";
import { Cloud, Radio, ShieldCheck } from "lucide-react";
import { desktop } from "../api";
import { useDraft } from "../hooks/useDraft";
import {
  cloudService,
  normalizePairingCode,
  type CloudStatus,
} from "../services/cloud";

const connectionLabels: Record<CloudStatus["connection"], string> = {
  disconnected: "Eşleştirme bekleniyor",
  connecting: "Cloud’a bağlanıyor",
  connected: "Cloud’a bağlı",
  retrying: "Yeniden bağlanıyor",
  revoked: "Yeniden eşleştirin",
};

export default function CloudSettings() {
  const actionActive = useRef(false);
  const statusRevision = useRef(0);
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
      const revision = statusRevision.current;
      try {
        if (actionActive.current) return;
        const value = await cloudService.status();
        if (!disposed && revision === statusRevision.current) {
          setStatus(value);
          setStatusError("");
        }
      } catch {
        if (!disposed && revision === statusRevision.current)
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
    if (actionActive.current) return;
    actionActive.current = true;
    ++statusRevision.current;
    setBusy(true);
    setError("");
    try {
      await action();
      setCode("");
      try {
        setStatus(await cloudService.status());
        setStatusError("");
      } catch {
        setStatusError(
          "İşlem tamamlandı. Bağlantı durumu yeniden denetleniyor.",
        );
      }
    } catch (e) {
      setError(String(e));
    } finally {
      actionActive.current = false;
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
              Cloud’dan aldığınız tek kodla bu bilgisayarı bağlayın.
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
          <p className="cloud-notice" role="status">
            {connection === "connected"
              ? "Bu cihaz artık Cloud hesabınıza bağlı. Servislerinizi ve projelerinizi panelden yönetebilirsiniz."
              : connection === "revoked"
                ? "Cloud erişimi kaldırılmış. Eşleştirmeyi kaldırıp panelden yeni bir cihaz kodu alın."
                : "Cihaz hesabınızla eşleştirildi. Güvenli bağlantı otomatik kuruluyor; yeniden kod girmeniz gerekmez."}
          </p>
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
          <details className="cloud-advanced">
            <summary>Bağlantı ayrıntıları</summary>
            <dl className="cloud-details">
              <div>
                <dt>Cloud sunucusu</dt>
                <dd>{status.url}</dd>
              </div>
              <div>
                <dt>Soket adresi · otomatik</dt>
                <dd>{status.socketEndpoint || "Cloud’dan alınıyor…"}</dd>
              </div>
            </dl>
          </details>
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
            if (!locked && /^[A-F0-9]{32}$/.test(code))
              void act(() =>
                cloudService.pair(
                  code,
                  url.replace(/\/$/, "") === status?.defaultUrl
                    ? undefined
                    : url,
                ),
              );
          }}
        >
          <div className="cloud-pairing">
            <label htmlFor="cloud-code">Bağlantı kodu</label>
            <p className="muted" id="cloud-code-help">
              Cloud panelinde cihazınızı ekleyin, kodu kopyalayıp buraya
              yapıştırın. Hesabınız, cihazınız ve soket bağlantısı otomatik
              tanımlanır.
            </p>
            <input
              id="cloud-code"
              className="cloud-code"
              value={code}
              onChange={(e) => setCode(normalizePairingCode(e.target.value))}
              onPaste={(e) => {
                e.preventDefault();
                setCode(normalizePairingCode(e.clipboardData.getData("text")));
              }}
              required
              minLength={32}
              maxLength={256}
              pattern="[A-Fa-f0-9]{32}"
              title="32 karakterli bağlantı kodunu girin."
              autoComplete="off"
              autoCapitalize="characters"
              spellCheck={false}
              disabled={locked}
              aria-describedby="cloud-code-help"
              placeholder="Cloud’dan kopyaladığınız kod"
            />
          </div>
          <div className="cloud-actions">
            <button
              className="button primary"
              disabled={locked || !/^[A-F0-9]{32}$/.test(code)}
            >
              {busy ? "Cloud’a bağlanıyor…" : "Cloud’a bağlan"}
            </button>
          </div>
          {status && url.replace(/\/$/, "") !== status.defaultUrl && (
            <p className="cloud-notice">
              Kayıtlı özel sunucu: {url}. Değiştirmek için gelişmiş ayarları
              açın.
            </p>
          )}
          <details className="cloud-advanced">
            <summary>Gelişmiş bağlantı ayarları</summary>
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
                onInvalid={(e) =>
                  e.currentTarget.closest("details")?.setAttribute("open", "")
                }
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
          </details>
          <p className="cloud-notice">
            <ShieldCheck size={18} aria-hidden="true" /> Sunucu adresi, soket
            anahtarı veya port girmeniz gerekmez. Bağlantı kaydedilir ve sonraki
            açılışlarda otomatik kurulur.
          </p>
        </form>
      )}
    </section>
  );
}
