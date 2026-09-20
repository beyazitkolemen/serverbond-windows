import { Play, Square, Download, Wrench } from "lucide-react";
import { call } from "../api";
import type { RedisState, Run } from "../types";

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
                : "Redis kurulu değil"}
          </strong>
          <p className="section-note">
            Redis {redis.version} · 127.0.0.1:{redis.port} · loopback, parola
            yok
          </p>
        </div>
      </div>
      {redis.issue ? (
        <p role="alert" className="settings-feedback">
          {redis.issue}
        </p>
      ) : null}
      <div className="settings-actions">
        {!redis.installed ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() => action("install", "Redis indiriliyor…")}
          >
            <Download size={16} />
            Redis kur
          </button>
        ) : (
          <>
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
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => action("repair", "Redis onarılıyor…")}
            >
              <Wrench size={16} />
              Onar
            </button>
          </>
        )}
      </div>
      <p className="section-note">
        Laravel <code>.env</code> dosyasına <code>REDIS_CLIENT=predis</code>,{" "}
        <code>REDIS_HOST=127.0.0.1</code>, <code>REDIS_PORT={redis.port}</code>{" "}
        ve isteğe bağlı <code>CACHE_STORE=redis</code> /{" "}
        <code>QUEUE_CONNECTION=redis</code> yazın. F4Box <code>.env</code>{" "}
        yazmaz. Resmî PHP paketinde <code>redis</code> uzantısı yoktur;{" "}
        <code>predis/predis</code> kullanın. Komut:{" "}
        <code>f4box redis install|start|stop</code>.
      </p>
    </section>
  );
}
