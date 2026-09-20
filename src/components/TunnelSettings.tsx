import { useState } from "react";
import { Globe, Play, Square, Download, Wrench, Trash2 } from "lucide-react";
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
  return (
    <section className="settings-section">
      <h2>Cloudflare tüneli</h2>
      <p className="section-note">
        Cloudflared {tunnel.version} yerel projelerinizi Cloudflare üzerinden
        dış bir adrese açar. Hangi adresin hangi porta gittiğini Cloudflare Zero
        Trust panelindeki tünel yapılandırması belirler; F4Box yalnızca
        bağlayıcıyı çalıştırır.
      </p>
      <div className="tunnel-status">
        <span className={`status-dot ${tunnel.running ? "on" : "off"}`} />
        <div>
          <strong>
            {tunnel.running
              ? `Tünel çalışıyor · PID ${tunnel.pid ?? "-"}`
              : tunnel.installed
                ? "Tünel durdu"
                : "Cloudflared kurulu değil"}
          </strong>
          <p className="section-note">
            {tunnel.tokenSaved
              ? "Jeton Windows hesabınıza bağlı olarak şifrelenmiş halde saklanıyor."
              : "Jeton kaydedilmedi. Tünel jeton olmadan başlatılmaz."}
          </p>
        </div>
      </div>
      {tunnel.issue && (
        <p role="alert" className="settings-feedback">
          {tunnel.issue}
        </p>
      )}
      <label>
        Tünel jetonu
        <input
          type="password"
          autoComplete="off"
          spellCheck={false}
          placeholder={
            tunnel.tokenSaved ? "Kayıtlı jeton korunuyor" : "eyJhIjoi…"
          }
          value={token}
          onChange={(e) => setToken(e.target.value)}
        />
      </label>
      <p className="section-note">
        Cloudflare Zero Trust → Networks → Tunnels ekranında tüneli oluşturup
        “Install and run a connector” adımındaki jetonu buraya yapıştırın. Jeton
        günlüklere ve komut satırına yazılmaz.
      </p>
      <div className="settings-actions">
        <button
          type="button"
          className="button primary"
          disabled={busy || token.trim().length === 0}
          onClick={() =>
            void run("Tünel jetonu kaydediliyor…", async () => {
              await call("save_tunnel_token", { token });
              setToken("");
              return "Tünel jetonu şifrelenerek kaydedildi.";
            })
          }
        >
          Jetonu kaydet
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
          <>
            <button
              type="button"
              className="button secondary"
              disabled={busy || !tunnel.tokenSaved}
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
          </>
        )}
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
        hata bu kartta ve Günlükler → cloudflared bölümünde görünür.
      </p>
    </section>
  );
}
