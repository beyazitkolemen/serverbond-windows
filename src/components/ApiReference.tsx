import { useEffect, useMemo, useState } from "react";
import { Copy, Download, Search } from "lucide-react";
import { apiService } from "../services";
import { searchText } from "../search";
import type { ApiDocumentation, ApiRoute, Run } from "../types";
import ApiRequestSchema from "./ApiRequestSchema";

function expand(route: ApiRoute): ApiRoute[] {
  const [path, query] = route.path.split("?");
  const choice = path.match(/\{([^{}]*\|[^{}]*)\}/);
  if (choice)
    return choice[1].split("|").flatMap((value) =>
      expand({
        ...route,
        path: path.replace(choice[0], value) + (query ? `?${query}` : ""),
      }),
    );
  const exampleQuery = query.startsWith("source=")
    ? "source=php"
    : query
      ? query
          .replace(/\{page\}/g, "1")
          .replace(/\{repository\}/g, "owner/repo")
          .replace(/\{[^}]+\}/g, "php")
      : "";
  return [{ ...route, path: path + (exampleQuery ? `?${exampleQuery}` : "") }];
}

export default function ApiReference({ run }: { run: Run }) {
  const [info, setInfo] = useState<ApiDocumentation | null>(null);
  const [error, setError] = useState("");
  const [attempt, setAttempt] = useState(0);
  const [query, setQuery] = useState("");
  const [method, setMethod] = useState("");
  useEffect(() => {
    let active = true;
    setError("");
    void apiService
      .documentation()
      .then((value) => {
        if (active) setInfo(value);
      })
      .catch((error) => {
        if (active) setError(String(error));
      });
    return () => {
      active = false;
    };
  }, [attempt]);
  const routes = useMemo(() => info?.routes.flatMap(expand) ?? [], [info]);
  const shown = routes.filter(
    (route) =>
      (!method || route.method === method) &&
      searchText(`${route.method} ${route.path} ${route.description}`).includes(
        searchText(query),
      ),
  );
  const download = () => {
    if (!info?.document) return;
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(info.document, null, 2)], {
        type: "application/json",
      }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = "serverbond-openapi.json";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 30000);
  };
  return (
    <section
      className="settings-section api-reference"
      aria-labelledby="api-reference-title"
    >
      <div className="api-reference-header">
        <div>
          <h2 id="api-reference-title">Uç noktalar</h2>
          <p className="section-note">
            Bearer kimlik doğrulaması · JSON · /api/v1
          </p>
        </div>
        <button
          type="button"
          className="button secondary small"
          disabled={!info?.document}
          onClick={download}
        >
          <Download size={16} />
          OpenAPI indir
        </button>
      </div>
      {error ? (
        <div role="alert">
          <p>{error}</p>
          <button
            type="button"
            className="button secondary small"
            onClick={() => setAttempt((value) => value + 1)}
          >
            Yeniden dene
          </button>
        </div>
      ) : !info ? (
        <p role="status">API sözleşmesi yükleniyor…</p>
      ) : (
        <>
          {!info.document ? (
            <p className="section-note">
              OpenAPI dosyası masaüstü uygulamasından indirilebilir.
            </p>
          ) : null}
          <div className="api-reference-filters">
            <label className="api-route-search">
              <Search size={16} />
              <input
                type="search"
                aria-label="API uç noktası ara"
                placeholder="Proje, hizmet veya yol ara"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
            <select
              aria-label="HTTP yöntemi"
              value={method}
              onChange={(event) => setMethod(event.target.value)}
            >
              <option value="">Tüm yöntemler</option>
              {["GET", "POST", "PUT", "DELETE"].map((value) => (
                <option key={value}>{value}</option>
              ))}
            </select>
            <span role="status">
              {shown.length} / {routes.length}
            </span>
          </div>
          <div className="api-route-list">
            {shown.map((route) => (
              <details
                className="api-route"
                key={`${route.method} ${route.path}`}
              >
                <summary>
                  <span
                    className={`api-method method-${route.method.toLowerCase()}`}
                  >
                    {route.method}
                  </span>
                  <code>{route.path}</code>
                </summary>
                <div className="api-route-detail">
                  <p>{route.description}</p>
                  <ApiRequestSchema document={info.document} route={route} />
                  <button
                    type="button"
                    className="button secondary small"
                    onClick={() =>
                      void run("API yolu kopyalanıyor…", () =>
                        navigator.clipboard.writeText(route.path),
                      )
                    }
                  >
                    <Copy size={14} />
                    Yolu kopyala
                  </button>
                </div>
              </details>
            ))}
          </div>
          {!shown.length ? (
            <div className="search-empty">
              <p>Eşleşen uç nokta yok.</p>
              <button
                type="button"
                className="button secondary small"
                onClick={() => {
                  setQuery("");
                  setMethod("");
                }}
              >
                Filtreleri temizle
              </button>
            </div>
          ) : null}
        </>
      )}
    </section>
  );
}
