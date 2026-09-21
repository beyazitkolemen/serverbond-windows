import { useEffect, useState } from "react";
import { Copy, KeyRound, RefreshCw, Trash2 } from "lucide-react";
import { apiService } from "../services";
import type { ApiSettings as ApiValues, ApiStatus, Run } from "../types";
import NumberField from "./NumberField";
import Toggle from "./Toggle";
import StatusBadge, { type StatusTone } from "./StatusBadge";

/**
 * API: switch the local management API on, pick its port, create
 * or revoke the bearer token and copy a ready-to-run example. The token is
 * shown once; only its hash is stored on disk.
 */
export default function ApiSettings({
  values,
  onChange,
  busy,
  run,
  dirty,
}: {
  values: ApiValues;
  onChange: (patch: Partial<ApiValues>) => void;
  busy: boolean;
  run: Run;
  dirty: boolean;
}) {
  const [status, setStatus] = useState<ApiStatus | null>(null);
  const [token, setToken] = useState("");
  const refresh = () =>
    apiService
      .status()
      .then(setStatus)
      .catch(() => setStatus(null));
  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 5000);
    return () => clearInterval(timer);
  }, []);
  const baseUrl = status?.baseUrl ?? `http://127.0.0.1:${values.port}/api/v1`;
  const example = `curl -H "Authorization: Bearer ${token || "<jeton>"}" ${baseUrl}/status`;
  const copy = (label: string, text: string) =>
    void run(`${label} kopyalanıyor…`, async () => {
      await navigator.clipboard.writeText(text);
      return `${label} panoya kopyalandı.`;
    });
  const state: { tone: StatusTone; text: string } = !status
    ? { tone: "stopped", text: "Durum okunamadı" }
    : status.listening
      ? { tone: "running", text: `Dinliyor · ${status.baseUrl}` }
      : status.enabled
        ? {
            tone: "issue",
            text: "Açık ama dinlemiyor · günlükleri kontrol edin",
          }
        : { tone: "stopped", text: "Kapalı" };
  return (
    <section className="settings-section api-settings">
      <h2>Yönetim API'si</h2>
      <p className="section-note">
        Yerel yönetim API’si. İşlemler için <code>Authorization: Bearer</code>{" "}
        jetonu gerekir.
      </p>
      <div className="api-status">
        <StatusBadge tone={state.tone}>{state.text}</StatusBadge>
      </div>
      <div className="api-connection">
        <div>
          <span>Bağlantı adresi</span>
          <code>{baseUrl}</code>
        </div>
        <button
          type="button"
          className="button secondary small"
          disabled={busy}
          onClick={() => copy("API adresi", baseUrl)}
        >
          <Copy size={16} />
          Adresi kopyala
        </button>
      </div>
      <div className="settings-grid">
        <Toggle
          label="API'yi aç"
          value={values.enabled}
          onChange={(enabled) => onChange({ enabled })}
        />
        <NumberField
          label="Port"
          value={values.port}
          min={1024}
          max={65535}
          onChange={(port) => onChange({ port })}
          hint="Diğer hizmet portlarından farklı olmalı."
        />
      </div>
      {dirty ? (
        <p className="section-note">
          Kaydet ile uygulanır; servisleri durdurmanız gerekmez.
        </p>
      ) : null}
      <h3>Jeton</h3>
      <p className="section-note">
        {status?.tokenSaved
          ? "Bir jeton kayıtlı. Yenisini oluşturmak eskisini geçersiz kılar."
          : "Jeton oluşturun. Sağlık denetimi dışındaki istekler jeton gerektirir."}
      </p>
      <div className="settings-actions">
        <button
          type="button"
          className="button primary small"
          disabled={busy}
          onClick={() =>
            void run("API jetonu oluşturuluyor…", async () => {
              const created = await apiService.createToken();
              setToken(created);
              await refresh();
              return "Yeni jeton oluşturuldu. Yalnızca bu ekranda gösterilir.";
            })
          }
        >
          {status?.tokenSaved ? (
            <RefreshCw size={16} />
          ) : (
            <KeyRound size={16} />
          )}
          {status?.tokenSaved ? "Jetonu yenile" : "Jeton oluştur"}
        </button>
        <button
          type="button"
          className="button secondary small"
          disabled={busy || !status?.tokenSaved}
          onClick={() =>
            void run("API jetonu siliniyor…", async () => {
              await apiService.forgetToken();
              setToken("");
              await refresh();
              return "Jeton silindi; API istekleri artık kabul edilmez.";
            })
          }
        >
          <Trash2 size={16} />
          Jetonu sil
        </button>
      </div>
      {token ? (
        <div className="api-token" role="status">
          <code>{token}</code>
          <button
            type="button"
            className="button secondary small"
            disabled={busy}
            onClick={() => copy("Jeton", token)}
          >
            <Copy size={16} />
            Kopyala
          </button>
          <p className="section-note">
            Jeton yalnızca bir kez gösterilir. Güvenli bir yere kaydedin.
          </p>
        </div>
      ) : null}
      <details className="api-example-help">
        <summary>Bağlantı örneği</summary>
        <div className="api-example">
          <pre>{example}</pre>
          <button
            type="button"
            className="button secondary small"
            disabled={busy}
            onClick={() => copy("Örnek komut", example)}
          >
            <Copy size={16} />
            Kopyala
          </button>
        </div>
        <p className="section-note">
          Servisler, projeler, masaüstü ve güncellemeler API üzerinden
          yönetilir. Sözleşme: <code>GET {baseUrl}/openapi.json</code>.
          Yetenekler: <code>GET /capabilities</code>.
        </p>
      </details>
    </section>
  );
}
