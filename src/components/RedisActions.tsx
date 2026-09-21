import { redisService } from "../services";
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
  return (
    <section className="settings-section">
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
        action={() => redisService.repair()}
      />
      <details className="connection-help">
        <summary>Laravel bağlantısı</summary>
        <p className="section-note">
          <code>.env</code>: <code>REDIS_CLIENT=predis</code>,{" "}
          <code>REDIS_HOST=127.0.0.1</code>,{" "}
          <code>REDIS_PORT={redis.port}</code> ve isteğe bağlı{" "}
          <code>CACHE_STORE=redis</code> / <code>QUEUE_CONNECTION=redis</code>{" "}
          yazın. ServerBond <code>.env</code> yazmaz. PHP’de Redis uzantısı
          yerine <code>predis/predis</code> kullanın.
        </p>
      </details>
    </section>
  );
}
