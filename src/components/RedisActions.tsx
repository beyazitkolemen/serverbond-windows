import { Play, Square, Download } from "lucide-react";
import { call } from "../api";
import type { RedisState, Run } from "../types";
import ServiceRepair from "./ServiceRepair";

export default function RedisActions({
  redis,
  busy,
  run,
}: {
  redis: RedisState;
  busy: boolean;
  run: Run;
}) {
  const action = (name: string, message: string) =>
    void run(message, () => call("redis", { action: name }));
  return (
    <section className="settings-section">
      <h2>Redis servisi</h2>
      <div className="tunnel-status">
        <span className={`status-dot ${redis.running ? "on" : "off"}`} />
        <div>
          <strong>
            {redis.running
              ? `Redis çalışıyor · PID ${redis.pid ?? "-"}`
              : redis.installed
                ? "Redis durdu"
                : redis.repairable
                  ? "Kurulum eksik"
                  : "Redis kurulu değil"}
          </strong>
          <p className="section-note">
            Redis {redis.version} · 127.0.0.1:{redis.port} · loopback, parola
            yok
          </p>
        </div>
      </div>
      {redis.issue && redis.installed ? (
        <p role="alert" className="settings-feedback">
          {redis.issue}
        </p>
      ) : null}
      <div className="settings-actions">
        {!redis.installed && !redis.repairable ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() => action("install", "Redis indiriliyor…")}
          >
            <Download size={16} />
            Redis kur
          </button>
        ) : redis.installed ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() =>
              action(
                redis.running ? "stop" : "start",
                redis.running ? "Redis durduruluyor…" : "Redis başlatılıyor…",
              )
            }
          >
            {redis.running ? <Square size={16} /> : <Play size={16} />}
            {redis.running ? "Durdur" : "Başlat"}
          </button>
        ) : null}
      </div>
      <ServiceRepair
        name="Redis"
        installed={redis.installed}
        repairable={redis.repairable}
        running={redis.running}
        issue={redis.issue}
        busy={busy}
        run={run}
        keeps={[
          "Veri dizini (data/redis)",
          "Port ve otomatik başlatma",
          "Proje .env dosyaları",
        ]}
        action={() => call("redis", { action: "repair" })}
      />
      <p className="section-note">
        Port ve otomatik başlatma Ayarlar düğmesindedir. Laravel{" "}
        <code>.env</code> dosyasına <code>REDIS_CLIENT=predis</code>,{" "}
        <code>REDIS_HOST=127.0.0.1</code>, <code>REDIS_PORT={redis.port}</code>{" "}
        ve isteğe bağlı <code>CACHE_STORE=redis</code> /{" "}
        <code>QUEUE_CONNECTION=redis</code> yazın. F4Box <code>.env</code>{" "}
        yazmaz. Resmî PHP paketinde <code>redis</code> uzantısı yoktur;{" "}
        <code>predis/predis</code> kullanın. Komut:{" "}
        <code>f4box redis install|start|stop|repair</code>.
      </p>
    </section>
  );
}
