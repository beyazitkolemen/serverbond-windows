import { useEffect, useRef, useState } from "react";
import { Check, GitBranch, LockKeyhole, RefreshCw, Search } from "lucide-react";
import { githubService } from "../services";
import type { GithubBranchPage, GithubRepository, GithubState } from "../types";
import GithubConnect from "./GithubConnect";

export default function GithubRepositoryPicker({
  github,
  repository,
  branch,
  disabled,
  onRepository,
  onBranch,
}: {
  github: GithubState;
  repository: string;
  branch: string;
  disabled: boolean;
  onRepository: (repository: string, defaultBranch?: string) => void;
  onBranch: (branch: string) => void;
}) {
  const [mode, setMode] = useState<"list" | "manual">(
    github.tokenSaved ? "list" : "manual",
  );
  const [repos, setRepos] = useState<GithubRepository[]>([]);
  const [next, setNext] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [query, setQuery] = useState("");
  const [owner, setOwner] = useState("");
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  const wasConnected = useRef(github.tokenSaved);
  const account = `${github.tokenSaved}:${github.login ?? ""}`;
  const previousAccount = useRef(account);
  const chooseRepository = useRef(onRepository);
  chooseRepository.current = onRepository;
  useEffect(() => {
    if (previousAccount.current !== account) {
      chooseRepository.current("");
      setQuery("");
      previousAccount.current = account;
    }
  }, [account]);
  useEffect(() => {
    if (github.tokenSaved && !wasConnected.current) setMode("list");
    wasConnected.current = github.tokenSaved;
  }, [github.tokenSaved]);
  useEffect(() => {
    const id = ++generation.current;
    setRepos([]);
    setNext(null);
    setError("");
    setOwner("");
    setLoading(false);
    if (!github.tokenSaved || mode !== "list") return;
    setLoading(true);
    void githubService
      .repositories()
      .then((result) => {
        if (generation.current !== id) return;
        setRepos(result.repositories);
        setNext(result.nextPage);
      })
      .catch((error) => {
        if (generation.current === id) setError(String(error));
      })
      .finally(() => {
        if (generation.current === id) setLoading(false);
      });
    return () => {
      generation.current++;
    };
  }, [github.login, github.tokenSaved, mode, revision]);
  async function loadMore() {
    if (!next || loading) return;
    const id = generation.current;
    setLoading(true);
    setError("");
    try {
      const result = await githubService.repositories(next);
      if (generation.current !== id) return;
      setRepos((current) => [
        ...new Map(
          [...current, ...result.repositories].map((repo) => [
            repo.fullName,
            repo,
          ]),
        ).values(),
      ]);
      setNext(result.nextPage);
    } catch (error) {
      if (generation.current === id) setError(String(error));
    } finally {
      if (generation.current === id) setLoading(false);
    }
  }
  const shown = repos.filter(
    (repo) =>
      (!owner || repo.owner === owner) &&
      `${repo.fullName} ${repo.description ?? ""}`
        .toLocaleLowerCase()
        .includes(query.toLocaleLowerCase()),
  );
  const owners = [...new Set(repos.map((repo) => repo.owner))].sort();
  return (
    <div className="github-picker">
      {!github.tokenSaved ? (
        <GithubConnect github={github} disabled={disabled} />
      ) : null}
      <div className="github-picker-heading">
        <strong>
          {github.login ? `${github.login} / Depolar` : "Git deposu"}
        </strong>
        <div className="tabs compact" role="group" aria-label="Depo kaynağı">
          <button
            type="button"
            className={mode === "list" ? "active" : ""}
            disabled={disabled || !github.tokenSaved}
            onClick={() => setMode("list")}
          >
            Hesabımdaki depolar
          </button>
          <button
            type="button"
            className={mode === "manual" ? "active" : ""}
            disabled={disabled}
            onClick={() => setMode("manual")}
          >
            Adres ile ekle
          </button>
        </div>
      </div>
      {mode === "list" ? (
        <>
          <div className="github-repo-filters">
            <label className="github-search">
              <Search size={16} />
              <input
                aria-label="Yüklü depolarda ara"
                placeholder="Yüklü depolarda ara…"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </label>
            <select
              aria-label="Hesap veya organizasyon"
              value={owner}
              onChange={(e) => setOwner(e.target.value)}
            >
              <option value="">Tüm hesaplar</option>
              {owners.map((owner) => (
                <option key={owner} value={owner}>
                  {owner}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="icon-button"
              aria-label="Depoları yenile"
              disabled={loading || disabled}
              onClick={() => setRevision((v) => v + 1)}
            >
              <RefreshCw size={17} />
            </button>
          </div>
          <div
            className="github-repo-list"
            role="group"
            aria-label="GitHub depoları"
            aria-busy={loading}
          >
            {shown.map((repo) => (
              <button
                type="button"
                key={repo.fullName}
                className={`github-repo-row ${repository === repo.fullName ? "selected" : ""}`}
                aria-pressed={repository === repo.fullName}
                disabled={disabled}
                onClick={() => onRepository(repo.fullName, repo.defaultBranch)}
              >
                <span className="github-repo-main">
                  <strong>{repo.fullName}</strong>
                  {repo.description ? (
                    <span className="github-repo-description">
                      {repo.description}
                    </span>
                  ) : null}
                  <span className="github-repo-meta">
                    {repo.private ? (
                      <>
                        <LockKeyhole size={12} /> Özel
                      </>
                    ) : (
                      "Genel"
                    )}
                    {repo.language ? ` · ${repo.language}` : ""}
                    {repo.archived ? " · Arşiv" : ""}
                    {repo.fork ? " · Fork" : ""} ·{" "}
                    {new Date(repo.updatedAt).toLocaleDateString("tr-TR")}
                  </span>
                </span>
                {repository === repo.fullName ? (
                  <Check size={17} />
                ) : (
                  <GitBranch size={16} />
                )}
              </button>
            ))}
            {!shown.length && !loading ? (
              <p className="section-note">
                {error
                  ? "Depo listesi alınamadı."
                  : repos.length
                    ? "Yüklü depolarda eşleşme yok."
                    : "Bu hesap için erişilebilir depo bulunamadı."}
              </p>
            ) : null}
          </div>
          <div className="github-list-footer">
            <span role="status">
              {loading
                ? "Depolar yükleniyor…"
                : `${repos.length} depo yüklendi`}
            </span>
            {next ? (
              <button
                type="button"
                className="button secondary small"
                disabled={loading || disabled}
                onClick={() => void loadMore()}
              >
                Daha fazla depo
              </button>
            ) : null}
          </div>
          {error ? (
            <p className="field-error" role="alert">
              {error}
            </p>
          ) : null}
          {repository ? (
            <p className="field-hint">
              Seçili depo: <strong>{repository}</strong>
            </p>
          ) : null}
        </>
      ) : (
        <label>
          Git deposu
          <input
            name="repository"
            placeholder="owner/repo veya https://… git adresi"
            value={repository}
            disabled={disabled}
            onChange={(e) => onRepository(e.target.value)}
            required
          />
        </label>
      )}
      {repository ? (
        <GithubBranches
          key={`${github.login}:${repository}`}
          repository={repository}
          branch={branch}
          onChange={onBranch}
          disabled={disabled}
          automatic={mode === "list"}
        />
      ) : null}
    </div>
  );
}

function GithubBranches({
  repository,
  branch,
  onChange,
  disabled,
  automatic,
}: {
  repository: string;
  branch: string;
  onChange: (branch: string) => void;
  disabled: boolean;
  automatic: boolean;
}) {
  const [data, setData] = useState<GithubBranchPage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [filter, setFilter] = useState("");
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  const branchRef = useRef(branch);
  branchRef.current = branch;
  const changeRef = useRef(onChange);
  changeRef.current = onChange;
  useEffect(() => {
    const id = ++generation.current;
    if (!automatic && !revision) return;
    setLoading(true);
    setError("");
    setData(null);
    void githubService
      .branches(repository)
      .then((result) => {
        if (generation.current !== id) return;
        setData(result);
        if (!branchRef.current && result.branches.length)
          changeRef.current(result.defaultBranch);
      })
      .catch((error) => {
        if (generation.current === id) setError(String(error));
      })
      .finally(() => {
        if (generation.current === id) setLoading(false);
      });
    return () => {
      generation.current++;
    };
  }, [repository, automatic, revision]);
  async function more() {
    if (!data?.nextPage || loading) return;
    const id = generation.current;
    setLoading(true);
    setError("");
    try {
      const result = await githubService.branches(repository, data.nextPage);
      if (generation.current !== id) return;
      setData((current) => ({
        ...result,
        branches: [
          ...new Map(
            [...(current?.branches ?? []), ...result.branches].map((branch) => [
              branch.name,
              branch,
            ]),
          ).values(),
        ],
      }));
    } catch (error) {
      if (generation.current === id) setError(String(error));
    } finally {
      if (generation.current === id) setLoading(false);
    }
  }
  const branches =
    data?.branches.filter((item) =>
      item.name.toLocaleLowerCase().includes(filter.toLocaleLowerCase()),
    ) ?? [];
  return (
    <div className="github-branches">
      <div className="github-picker-heading">
        <strong>
          <GitBranch size={16} /> Dal seçimi
        </strong>
        <button
          type="button"
          className="button secondary small"
          disabled={loading || disabled}
          onClick={() => setRevision((v) => v + 1)}
        >
          {data ? "Dalları yenile" : "Dalları getir"}
        </button>
      </div>
      {data ? (
        <>
          <label>
            Dal ara
            <input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Dal adı…"
            />
          </label>
          <label>
            Dal
            <select
              name="branch"
              value={branch}
              disabled={disabled || loading}
              onChange={(e) => onChange(e.target.value)}
            >
              <option value="">Varsayılan dal ({data.defaultBranch})</option>
              {branch && !branches.some((item) => item.name === branch) ? (
                <option value={branch}>
                  {branch}
                  {branch === data.defaultBranch
                    ? " · Varsayılan"
                    : " · Seçili"}
                </option>
              ) : null}
              {branches.map((item) => (
                <option key={item.name} value={item.name}>
                  {item.name}
                  {item.name === data.defaultBranch ? " · Varsayılan" : ""}
                  {item.protected ? " · Korumalı" : ""}
                </option>
              ))}
            </select>
          </label>
          <div className="github-list-footer">
            <span>
              {data.branches.length
                ? `${data.branches.length} dal yüklendi`
                : "Bu depoda henüz dal yok."}
            </span>
            {data.nextPage ? (
              <button
                type="button"
                className="button secondary small"
                disabled={loading || disabled}
                onClick={() => void more()}
              >
                Daha fazla dal
              </button>
            ) : null}
          </div>
        </>
      ) : (
        <label>
          Dal (isteğe bağlı)
          <input
            name="branch"
            value={branch}
            disabled={disabled}
            placeholder="varsayılan dal"
            onChange={(e) => onChange(e.target.value)}
          />
        </label>
      )}
      {loading ? (
        <p className="section-note" role="status">
          Dallar yükleniyor…
        </p>
      ) : null}
      {error ? (
        <p className="field-error" role="alert">
          {error}
          {!data
            ? " Dal adını elle de girebilirsiniz."
            : " Yeniden deneyebilirsiniz."}
        </p>
      ) : null}
    </div>
  );
}
