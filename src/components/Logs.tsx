import { useEffect, useState } from "react";
import { RefreshCw } from "lucide-react";
import { call } from "../api";

export function LogPreview({ logs }: { logs: string[] }) {
  return (
    <section aria-labelledby="logs-heading">
      <div className="section-heading">
        <h2 id="logs-heading">Günlükler</h2>
      </div>
      <pre
        className="console preview-console"
        aria-label="Son günlük kayıtları"
      >
        {logs.slice(-3).join("\n") || "Henüz kayıt yok."}
      </pre>
    </section>
  );
}

export default function Logs() {
  const [source, setSource] = useState("f4box");
  const [text, setText] = useState("Yükleniyor…");
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let cancelled = false;
    const update = () =>
      call<string>("read_log", { id: source })
        .then((value) => {
          if (!cancelled) setText(value);
        })
        .catch((error) => {
          if (!cancelled) setText(String(error));
        });
    void update();
    const timer = window.setInterval(() => void update(), 2500);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [source, revision]);
  return (
    <section>
      <div className="log-toolbar">
        <div className="tabs" role="group" aria-label="Günlük kaynağı">
          {["f4box", "php", "mysql", "caddy", "composer"].map((id) => (
            <button
              key={id}
              className={source === id ? "active" : ""}
              onClick={() => {
                setText("Yükleniyor…");
                setSource(id);
              }}
            >
              {id === "f4box"
                ? "F4Box"
                : id === "php"
                  ? "PHP"
                  : id === "mysql"
                    ? "MySQL"
                    : id[0].toUpperCase() + id.slice(1)}
            </button>
          ))}
        </div>
        <button
          className="button secondary small"
          onClick={() => setRevision((x) => x + 1)}
        >
          <RefreshCw size={16} />
          Yenile
        </button>
      </div>
      <pre
        className="console full-console"
        tabIndex={0}
        aria-label={`${source} günlüğü`}
      >
        {text}
      </pre>
    </section>
  );
}
