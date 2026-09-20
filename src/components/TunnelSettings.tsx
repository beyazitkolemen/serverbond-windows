import { useState } from "react";
import {
  Globe,
  Play,
  Square,
  Download,
  Wrench,
  Trash2,
  Eye,
  EyeOff,
} from "lucide-react";
import { call } from "../api";
import type { Run, TunnelState } from "../types";

export default function TunnelSettings({
  tunnel,
  busy,
  run,
}: {
  tunnel: TunnelState;
  busy: boolean;
  run: Run;
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
  return (
    <section className="settings-section">
      <h2>Cloudflare tüneli</h2>
      <p className="section-note">
        Cloudflared {tunnel.version} yerel projelerinizi Cloudflare üzerinden
        dış bir adrese açar. Jetonu bu ekrana yapıştırın; kurulum, kayıt ve
        başlatma uygulamadan yönetilir. Hangi adresin hangi porta gittiğini
        Cloudflare Zero Trust panelindeki tünel yapılandırması belirler.
      </p>
      <div className="tunnel-status">
        <span className={`status-dot ${tunnel.running ? "on" : "off"}`} />
        <div>
          <strong>
            {tunnel.running
              ? `Tünel çalışıyor · PID ${tunnel.pid ?? "-"}`
              : tunnel.installed
                ? "Cloudflared kurulu · tünel durdu"
                : "Cloudflared henüz kurulmadı"}
          </strong>
          <p className="section-note">
            {tunnel.tokenSaved
              ? "Jeton Windows hesabınıza bağlı olarak şifrelenmiş halde saklanıyor."
              : "Jeton kaydedilmedi. Aşağıya yapıştırıp kaydedin."}
          </p>
        </div>
      </div>
      {tunnel.issue && (
        <p role="alert" className="settings-feedback">
          {tunnel.issue}
        </p>
      )}

      <h3>1. Cloudflared</h3>
      <p className="section-note">
        Sabit Windows x64 paketi SHA-256 ile doğrulanarak F4Box klasörüne iner.
        Ortamın çalışması için gerekli değildir.
      </p>
      <div className="settings-actions">
        {!tunnel.installed ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() =>
              void run("Cloudflared indiriliyor…", () =>
                call("tunnel", { action: "install" }),
              )
            }
          >
            <Download size={16} />
            Cloudflared kur
          </button>
        ) : (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() =>
              void run("Cloudflared onarılıyor…", () =>
                call("tunnel", { action: "repair" }),
              )
            }
          >
            <Wrench size={16} />
            Onar
          </button>
        )}
      </div>

      <h3>2. Tünel jetonu</h3>
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
        Cloudflare Zero Trust → Networks → Tunnels → tüneli oluşturun → Install
        and run a connector. Jetonu veya tüm{" "}
        <code>cloudflared.exe service install …</code> satırını buraya
        yapıştırın. F4Box jetonu ayıklar; günlüklere ve komut satırına yazmaz.
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

      <h3>3. Çalıştırma</h3>
      <div className="settings-actions">
        <button
          type="button"
          className="button secondary"
          disabled={busy || !tunnel.tokenSaved}
          title={
            tunnel.tokenSaved
              ? undefined
              : "Önce jetonu kaydedin. Cloudflared yoksa başlatırken kurulur."
          }
          onClick={() =>
            void run(
              tunnel.running ? "Tünel durduruluyor…" : "Tünel açılıyor…",
              () =>
                call("tunnel", {
                  action: tunnel.running ? "stop" : "start",
                }),
            )
          }
        >
          {tunnel.running ? <Square size={16} /> : <Play size={16} />}
          {tunnel.running ? "Tüneli durdur" : "Tüneli başlat"}
        </button>
      </div>
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
      <p className="section-note">
        <Globe size={14} /> Tünel başarısız olursa ortam çalışmaya devam eder;
        hata bu kartta ve Günlükler → cloudflared bölümünde görünür. Tepsi
        menüsündeki Servisler → Cloudflare tüneli aynı başlat/durdur işini
        yapar.
      </p>
    </section>
  );
}
