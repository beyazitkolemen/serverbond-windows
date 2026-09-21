import { Copy } from "lucide-react";
import type { ApiSettings, ApiStatus } from "../types";
import Toggle from "./Toggle";
import StatusBadge from "./StatusBadge";

export default function McpConnection({
  values,
  status,
  token,
  busy,
  onChange,
  copy,
}: {
  values: ApiSettings;
  status: ApiStatus | null;
  token: string;
  busy: boolean;
  onChange: (patch: Partial<ApiSettings>) => void;
  copy: (label: string, text: string) => void;
}) {
  const url = status?.mcpUrl ?? `http://127.0.0.1:${values.port}/mcp`;
  const ready = status?.listening && status.mcpEnabled && status.tokenSaved;
  const config = JSON.stringify(
    {
      mcpServers: {
        serverbond: {
          url,
          headers: { Authorization: `Bearer ${token || "<API_JETONU>"}` },
        },
      },
    },
    null,
    2,
  );
  return (
    <section aria-labelledby="mcp-heading" className="mcp-connection">
      <div className="api-reference-header">
        <h3 id="mcp-heading">MCP bağlantısı</h3>
        <StatusBadge tone={ready ? "running" : "stopped"}>
          {ready
            ? "Hazır"
            : !status
              ? "Durum okunamadı"
              : !status.mcpEnabled
                ? "Kapalı"
                : !status.listening
                  ? "API kapalı"
                  : "Jeton gerekli"}
        </StatusBadge>
      </div>
      <p className="section-note">
        Yapay zekâ istemcilerinden proje, hizmet ve uygulama yönetimi. API ile
        aynı portu ve jetonu kullanır.
      </p>
      <Toggle
        label="MCP erişimini aç"
        value={values.mcpEnabled}
        disabled={busy}
        onChange={(mcpEnabled) => onChange({ mcpEnabled })}
      />
      {values.mcpEnabled && !values.enabled ? (
        <p className="section-note">
          Bağlantı için API’yi de açıp ayarları kaydedin.
        </p>
      ) : null}
      <div className="api-connection">
        <div>
          <span>Streamable HTTP</span>
          <code>{url}</code>
        </div>
        <button
          type="button"
          className="button secondary small"
          disabled={busy}
          onClick={() => copy("MCP adresi", url)}
        >
          <Copy size={16} /> Adresi kopyala
        </button>
      </div>
      <details className="api-example-help">
        <summary>İstemci yapılandırması</summary>
        <div className="api-example">
          <pre>{config}</pre>
          <button
            type="button"
            className="button secondary small"
            disabled={busy}
            onClick={() => copy("MCP yapılandırması", config)}
          >
            <Copy size={16} /> Kopyala
          </button>
        </div>
        <p className="section-note">
          Cursor gibi HTTP MCP istemcileri için. Jeton alanını doldurun;
          istemcinizde işlem onaylarını açık tutun. ServerBond çalışıyor olmalı.
          Uzak istemciler bu yerel adrese erişemez.
        </p>
      </details>
    </section>
  );
}
