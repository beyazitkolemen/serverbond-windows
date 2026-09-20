import { useCallback, useEffect, useState } from "react";
import { CheckCircle2, AlertTriangle, RefreshCw } from "lucide-react";
import { call, desktop } from "../api";
import type { Requirement, Run } from "../types";

export default function Requirements({
  busy,
  run,
}: {
  busy: boolean;
  run: Run;
}) {
  const [checks, setChecks] = useState<Requirement[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const refresh = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setChecks(await call<Requirement[]>("requirements"));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => {
    if (desktop) void refresh();
  }, [refresh]);
  return (
    <section
      className="settings-section requirements"
      aria-labelledby="requirements-heading"
    >
      <div className="section-heading">
        <h2 id="requirements-heading">Kurulum gereksinimleri</h2>
        <button
          className="button secondary small"
          disabled={busy || loading || !desktop}
          onClick={() => void refresh()}
        >
          <RefreshCw size={15} className={loading ? "spin" : ""} /> Yeniden
          denetle
        </button>
      </div>
      <p className="section-note">
        Kontroller bilgisayarınızda yapılır. Eksik Visual C++ çalışma zamanı
        indirme bağlantısından kurulabilir; ardından yeniden denetleyin.
      </p>
      {!desktop ? (
        <p>Gereksinimler masaüstü uygulamasında denetlenir.</p>
      ) : null}
      {error ? (
        <p className="field-error" role="alert">
          {error}
        </p>
      ) : null}
      {loading ? <p role="status">Gereksinimler denetleniyor…</p> : null}
      <div className="requirement-list">
        {checks.map((check) => (
          <div className={`requirement-item ${check.status}`} key={check.id}>
            {check.status === "ok" ? (
              <CheckCircle2 size={19} />
            ) : (
              <AlertTriangle size={19} />
            )}
            <div>
              <strong>{check.label}</strong>
              <span className="requirement-state">
                {check.status === "ok"
                  ? "Hazır"
                  : check.status === "warning"
                    ? "Kontrol edin"
                    : "İşlem gerekli"}
              </span>
              <p>{check.detail}</p>
              {check.helpUrl && check.status !== "ok" ? (
                <button
                  className="button secondary small"
                  disabled={busy}
                  onClick={() =>
                    void run("Microsoft indirme sayfası açılıyor…", () =>
                      call("open_runtime_download"),
                    )
                  }
                >
                  Microsoft'tan indir
                </button>
              ) : null}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
