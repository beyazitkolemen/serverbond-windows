import { useState } from "react";
import { Globe, Play, Trash2, Eye, EyeOff } from "lucide-react";
import { call } from "../api";
import type { Run, TunnelState } from "../types";
import ServiceRepair from "./ServiceRepair";

export default function TunnelSettings({
  tunnel,
  busy,
  run,
  pane = "all",
}: {
  tunnel: TunnelState;
  busy: boolean;
  run: Run;
  pane?: "all" | "ops" | "settings";
}) {
  const [token, setToken] = useState("");
  const [visible, setVisible] = useState(false);
  const ready = token.trim().length > 0;
  const save = (start: boolean) =>
    void run(
      start
        ? "Jeton kaydediliyor, Cloudflared kuruluyor ve tünel açılıyor…"
        : "Tünel jetonu kaydediliyor…",
      async () => {
        await call("save_tunnel_token", { token, start });
        setToken("");
        return start
          ? "Jeton kaydedildi ve tünel başlatıldı."
          : "Tünel jetonu şifrelenerek kaydedildi.";
      },
    );
  const showOps = pane !== "settings";
  const showSettings = pane !== "ops";
  return (
    <section className="settings-section">
      {pane !== "ops" ? (
        <h2>{pane === "settings" ? "Tünel ayarları" : "Cloudflare tüneli"}</h2>
      ) : null}
      {showOps ? (
        <p className="section-note">
          Cloudflared {tunnel.version} yerel projelerinizi Cloudflare üzerinden
          dış bir adrese açar. Hangi adresin hangi porta gittiğini Cloudflare
          Zero Trust panelindeki tünel yapılandırması belirler. Kurulum ve
          başlat/durdur üstteki denetim şeridindedir.
        </p>
      ) : (
        <p className="section-note">
          Jetonu buraya yapıştırın. F4Box jetonu ayıklar ve Windows DPAPI ile
          şifreler; günlüklere yazılmaz.
        </p>
      )}
      {showSettings && !showOps ? (
        <p className="section-note">
          {tunnel.tokenSaved
            ? "Jeton Windows hesabınıza bağlı olarak şifrelenmiş halde saklanıyor."
            : "Jeton kaydedilmedi. Aşağıya yapıştırıp kaydedin."}
        </p>
      ) : null}

      {showOps ? (
        <ServiceRepair
          name="Cloudflared"
          installed={tunnel.installed}
          repairable={tunnel.repairable}
          running={tunnel.running}
          issue={tunnel.issue}
          busy={busy}
          run={run}
          keeps={[
            "Kayıtlı tünel jetonu",
            "Otomatik başlatma tercihi",
            "Proje .env dosyaları",
          ]}
          action={() => call("tunnel", { action: "repair" })}
        />
      ) : null}

      {showSettings ? (
        <>
          <h3>Tünel jetonu</h3>
          <label htmlFor="tunnel-token">Cloudflare bağlayıcı jetonu</label>
          <div className="input-with-button">
            {visible ? (
              <textarea
                id="tunnel-token"
                autoComplete="off"
                spellCheck={false}
                rows={3}
                placeholder="eyJhIjoi… veya cloudflared.exe service install eyJhIjoi…"
                value={token}
                onChange={(e) => setToken(e.target.value)}
              />
            ) : (
              <input
                id="tunnel-token"
                type="password"
                autoComplete="off"
                spellCheck={false}
                placeholder={
                  tunnel.tokenSaved
                    ? "Kayıtlı jeton korunuyor — değiştirmek için yeni jeton yapıştırın"
                    : "eyJhIjoi… veya tüm service install komutu"
                }
                value={token}
                onChange={(e) => setToken(e.target.value)}
              />
            )}
            <button
              type="button"
              className="icon-button"
              disabled={busy}
              aria-label={visible ? "Jetonu gizle" : "Jetonu göster"}
              onClick={() => setVisible((value) => !value)}
            >
              {visible ? <EyeOff size={18} /> : <Eye size={18} />}
            </button>
          </div>
          <p className="section-note">
            Cloudflare Zero Trust → Networks → Tunnels → tüneli oluşturun →
            Install and run a connector. Jetonu veya tüm{" "}
            <code>cloudflared.exe service install …</code> satırını buraya
            yapıştırın. F4Box jetonu ayıklar; günlüklere ve komut satırına
            yazmaz.
          </p>
          <div className="settings-actions">
            <button
              type="button"
              className="button primary"
              disabled={busy || !ready}
              onClick={() => save(true)}
            >
              <Play size={16} />
              Kaydet ve tüneli başlat
            </button>
            <button
              type="button"
              className="button secondary"
              disabled={busy || !ready}
              onClick={() => save(false)}
            >
              Yalnızca jetonu kaydet
            </button>
            {tunnel.tokenSaved && (
              <button
                type="button"
                className="button secondary"
                disabled={busy}
                onClick={() =>
                  void run("Jeton siliniyor…", () =>
                    call("tunnel", { action: "forget" }),
                  )
                }
              >
                <Trash2 size={16} />
                Jetonu sil
              </button>
            )}
          </div>
        </>
      ) : null}

      {showSettings ? (
        <label className="setting-toggle">
          <input
            type="checkbox"
            checked={tunnel.autoStart}
            disabled={busy}
            onChange={(e) =>
              void run("Tünel tercihi kaydediliyor…", () =>
                call("save_tunnel_auto_start", { autoStart: e.target.checked }),
              )
            }
          />
          <span>Ortam başlatıldığında tüneli de başlat</span>
        </label>
      ) : null}
      <p className="section-note">
        <Globe size={14} /> Tünel başarısız olursa ortam çalışmaya devam eder;
        hata bu kartta ve Günlükler → cloudflared bölümünde görünür. Tepsi
        menüsündeki Servisler → Cloudflare tüneli aynı başlat/durdur işini
        yapar.
      </p>
    </section>
  );
}
