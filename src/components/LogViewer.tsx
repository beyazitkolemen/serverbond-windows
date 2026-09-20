import { useEffect, useMemo, useRef, useState } from "react";
import { Copy, Check, RefreshCw, Search } from "lucide-react";

export type LogSource = { id: string; label: string };

type Level = "error" | "warn" | "info" | "plain";

function classify(line: string): Level {
  const text = line.toLowerCase();
  if (
    /(fatal|exception|panic|traceback|failed|failure|error|başarısız|hata)/.test(
      text,
    )
  ) {
    return "error";
  }
  if (/(warning|deprecated|notice|warn)/.test(text)) {
    return "warn";
  }
  if (/(info|processed|processing|çalışıyor|running)/.test(text)) {
    return "info";
  }
  return "plain";
}

function splitLines(text: string) {
  const rows = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n");
  while (rows.length && rows[rows.length - 1] === "") {
    rows.pop();
  }
  return rows.map((line, index) => ({
    n: index + 1,
    text: line,
    level: classify(line),
  }));
}

export default function LogViewer({
  sources,
  load,
  compact,
  tall,
  label,
  initialSource,
}: {
  sources: LogSource[];
  load: (source: string) => Promise<string>;
  compact?: boolean;
  tall?: boolean;
  label: string;
  initialSource?: string;
}) {
  const first = sources[0]?.id ?? "";
  const [source, setSource] = useState(initialSource ?? first);
  const [text, setText] = useState("Yükleniyor…");
  const [query, setQuery] = useState("");
  const [follow, setFollow] = useState(true);
  const [copied, setCopied] = useState(false);
  const [revision, setRevision] = useState(0);
  const scroller = useRef<HTMLDivElement>(null);
  const loadRef = useRef(load);
  loadRef.current = load;
  useEffect(() => {
    if (!sources.some((item) => item.id === source)) {
      setSource(first);
    }
  }, [first, source, sources]);
  useEffect(() => {
    let cancelled = false;
    const current = source || first;
    if (!current) {
      setText("Gösterilecek günlük yok.");
      return;
    }
    const update = () =>
      loadRef
        .current(current)
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
  }, [first, revision, source]);
  const lines = useMemo(() => splitLines(text), [text]);
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return lines;
    return lines.filter((line) => line.text.toLowerCase().includes(needle));
  }, [lines, query]);
  const errors = filtered.filter((line) => line.level === "error").length;
  useEffect(() => {
    if (!follow) return;
    const node = scroller.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [filtered, follow]);
  const empty =
    !text.trim() ||
    text === "Henüz günlük kaydı yok." ||
    text === "Gösterilecek günlük yok.";
  return (
    <div
      className={`log-viewer${compact ? " is-compact" : ""}${tall ? " is-tall" : ""}`}
    >
      <div className="log-viewer-toolbar">
        {sources.length > 1 ? (
          <div
            className="tabs"
            role="tablist"
            aria-label={`${label} kaynakları`}
          >
            {sources.map((item) => (
              <button
                key={item.id}
                type="button"
                role="tab"
                aria-selected={item.id === source}
                className={item.id === source ? "active" : ""}
                onClick={() => {
                  setText("Yükleniyor…");
                  setSource(item.id);
                }}
              >
                {item.label}
              </button>
            ))}
          </div>
        ) : (
          <strong className="log-viewer-title">
            {sources[0]?.label ?? label}
          </strong>
        )}
        <label className="log-viewer-search">
          <Search size={14} aria-hidden />
          <input
            type="search"
            value={query}
            placeholder="Kayıtlarda ara"
            aria-label={`${label} içinde ara`}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <div className="log-viewer-actions">
          <label className="setting-toggle">
            <input
              type="checkbox"
              checked={follow}
              onChange={(e) => setFollow(e.target.checked)}
            />
            <span>Sona kaydır</span>
          </label>
          <button
            type="button"
            className="button secondary small"
            onClick={() => {
              void navigator.clipboard
                ?.writeText(text)
                .then(() => {
                  setCopied(true);
                  window.setTimeout(() => setCopied(false), 1500);
                })
                .catch(() => {});
            }}
          >
            {copied ? <Check size={14} /> : <Copy size={14} />}
            {copied ? "Kopyalandı" : "Kopyala"}
          </button>
          <button
            type="button"
            className="button secondary small"
            onClick={() => setRevision((value) => value + 1)}
          >
            <RefreshCw size={14} />
            Yenile
          </button>
        </div>
      </div>
      <div
        ref={scroller}
        className="log-viewer-body"
        tabIndex={0}
        role="log"
        aria-label={label}
        onScroll={(e) => {
          const node = e.currentTarget;
          const atEnd =
            node.scrollHeight - node.scrollTop - node.clientHeight < 24;
          if (follow !== atEnd) setFollow(atEnd);
        }}
      >
        {empty ? (
          <p className="log-viewer-empty">
            {query.trim()
              ? "Aramayla eşleşen kayıt yok."
              : "Henüz günlük kaydı yok."}
          </p>
        ) : (
          <ol className="log-viewer-lines">
            {filtered.map((line) => (
              <li key={line.n} className={`is-${line.level}`}>
                <span className="log-line-no">{line.n}</span>
                <span className="log-line-text">{line.text || " "}</span>
              </li>
            ))}
          </ol>
        )}
      </div>
      <p className="log-viewer-meta">
        {query.trim()
          ? `${filtered.length} / ${lines.length} satır`
          : `${lines.length} satır`}
        {errors ? ` · ${errors} hata` : ""}
      </p>
    </div>
  );
}
