import { useEffect, useId, useMemo, useRef, useState } from "react";
import { Copy, Check, RefreshCw, Pause, Play } from "lucide-react";
import SectionTabs from "./SectionTabs";
import SearchField from "./SearchField";
import { searchText } from "../search";

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
  const [text, setText] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [live, setLive] = useState(true);
  const [level, setLevel] = useState<"all" | "error" | "warn">("all");
  const [copyError, setCopyError] = useState("");
  const tabsId = useId();
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
    setText("");
    setError("");
    setCopied(false);
    setCopyError("");
  }, [source]);
  useEffect(() => {
    let cancelled = false;
    const current = source || first;
    if (!current) {
      setText("");
      setLoading(false);
      return;
    }
    const update = async () => {
      setLoading(true);
      try {
        const value = await loadRef.current(current);
        if (!cancelled) {
          setText(value);
          setError("");
        }
      } catch (reason) {
        if (!cancelled)
          setError(reason instanceof Error ? reason.message : String(reason));
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };
    void update();
    return () => {
      cancelled = true;
    };
  }, [first, revision, source]);
  useEffect(() => {
    if (!live || loading || !source) return;
    // Pause cancels the next refresh without discarding an in-flight read.
    const timer = window.setTimeout(
      () => setRevision((value) => value + 1),
      2500,
    );
    return () => clearTimeout(timer);
  }, [live, loading, revision, source]);
  const lines = useMemo(() => splitLines(text), [text]);
  const filtered = useMemo(() => {
    const needle = searchText(query);
    return lines.filter(
      (line) =>
        (level === "all" || line.level === level) &&
        searchText(line.text).includes(needle),
    );
  }, [lines, query, level]);
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
      {sources.length > 1 ? (
        <SectionTabs
          id={tabsId}
          label={`${label} kaynakları`}
          items={sources}
          value={source}
          onChange={(value) => {
            if (value === source) return;
            setCopyError("");
            setCopied(false);
            setText("");
            setError("");
            setLoading(true);
            setSource(value);
          }}
        />
      ) : (
        <strong className="log-viewer-title">
          {sources[0]?.label ?? label}
        </strong>
      )}
      <div
        id={`${tabsId}-panel`}
        className="log-content"
        role={sources.length > 1 ? "tabpanel" : undefined}
        aria-labelledby={sources.length > 1 ? `${tabsId}-${source}` : undefined}
      >
        <div className="log-viewer-toolbar">
          <SearchField
            value={query}
            onChange={setQuery}
            label={`${label} içinde ara`}
            placeholder="Kayıtlarda ara"
          />
          <select
            className="log-level-filter"
            aria-label="Kayıt seviyesi"
            value={level}
            onChange={(e) => setLevel(e.target.value as typeof level)}
          >
            <option value="all">Tüm seviyeler</option>
            <option value="error">Hatalar</option>
            <option value="warn">Uyarılar</option>
          </select>
          <div className="log-viewer-actions">
            <button
              type="button"
              className="button secondary small"
              aria-pressed={!live}
              onClick={() => setLive((value) => !value)}
            >
              {live ? <Pause size={14} /> : <Play size={14} />}
              {live ? "Akışı duraklat" : "Akışı sürdür"}
            </button>
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
              disabled={!filtered.length || empty}
              onClick={async () => {
                setCopyError("");
                try {
                  await navigator.clipboard.writeText(
                    filtered.map((line) => line.text).join("\n"),
                  );
                  setCopied(true);
                  window.setTimeout(() => setCopied(false), 1500);
                } catch {
                  setCopyError(
                    "Panoya kopyalanamadı. Kayıtları seçip Ctrl+C kullanın.",
                  );
                }
              }}
            >
              {copied ? <Check size={14} /> : <Copy size={14} />}
              {copied ? "Kopyalandı" : "Görüneni kopyala"}
            </button>
            <button
              type="button"
              className="button secondary small"
              disabled={loading}
              onClick={() => setRevision((value) => value + 1)}
            >
              <RefreshCw size={14} className={loading ? "spin" : undefined} />
              Yenile
            </button>
          </div>
        </div>
        {error && (
          <div className="log-feedback" role="alert">
            <strong>Günlük okunamadı</strong>
            <span>{error}</span>
            <button
              type="button"
              className="section-link"
              disabled={loading}
              onClick={() => setRevision((value) => value + 1)}
            >
              Yeniden dene
            </button>
          </div>
        )}
        {copyError && (
          <p role="alert" className="field-error">
            {copyError}
          </p>
        )}
        <div
          ref={scroller}
          className="log-viewer-body"
          tabIndex={0}
          role="log"
          aria-live="off"
          aria-busy={loading && !text}
          aria-label={label}
          onScroll={(e) => {
            const node = e.currentTarget;
            const atEnd =
              node.scrollHeight - node.scrollTop - node.clientHeight < 24;
            if (follow !== atEnd) setFollow(atEnd);
          }}
        >
          {loading && !text ? (
            <p role="status" className="log-viewer-empty">
              Günlük yükleniyor…
            </p>
          ) : empty || !filtered.length ? (
            <div className="log-viewer-empty">
              <strong>
                {query.trim() || level !== "all"
                  ? "Filtreyle eşleşen kayıt yok"
                  : error
                    ? "Günlük verisi alınamadı"
                    : "Henüz günlük kaydı yok"}
              </strong>
              {(query.trim() || level !== "all") && (
                <button
                  type="button"
                  className="section-link"
                  onClick={() => {
                    setQuery("");
                    setLevel("all");
                  }}
                >
                  Filtreleri temizle
                </button>
              )}
            </div>
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
          {query.trim() || level !== "all"
            ? `${filtered.length} / ${lines.length} satır`
            : `${empty ? 0 : lines.length} satır`}
          {errors ? ` · ${errors} hata` : ""}
          <span className={live ? "log-stream-live" : ""}>
            {live ? "Canlı akış" : "Akış duraklatıldı"}
          </span>
        </p>
      </div>
    </div>
  );
}
