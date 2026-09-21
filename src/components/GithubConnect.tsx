import { useEffect, useRef, useState } from "react";
import { Copy, ExternalLink, Github, LoaderCircle } from "lucide-react";
import { githubService } from "../services";
import type { GithubAuthFlow, GithubState } from "../types";

export default function GithubConnect({
  github,
  disabled = false,
}: {
  github: GithubState;
  disabled?: boolean;
}) {
  const [clientId, setClientId] = useState(github.oauthClientId ?? "");
  const [flow, setFlow] = useState<GithubAuthFlow | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const generation = useRef(0);
  const clientDirty = useRef(false);
  useEffect(() => {
    if (!clientDirty.current) setClientId(github.oauthClientId ?? "");
  }, [github.oauthClientId]);
  useEffect(
    () => () => {
      generation.current++;
    },
    [],
  );
  useEffect(() => {
    if (!flow) return;
    let active = true;
    let timer: number;
    const poll = async () => {
      if (!active) return;
      if (Date.now() >= flow.expiresAt * 1000) {
        setFlow(null);
        setError("Doğrulama kodunun süresi doldu. Yeniden giriş yapın.");
        return;
      }
      let interval = flow.interval;
      try {
        const result = await githubService.authPoll(flow.flowId);
        if (!active) return;
        setError("");
        if (result.status === "connected") {
          setFlow(null);
          setMessage(`${result.login} hesabı bağlandı.`);
          return;
        }
        if (result.status !== "pending") {
          setFlow(null);
          setError(
            result.status === "denied"
              ? "GitHub erişim isteği reddedildi."
              : result.status === "expired"
                ? "Doğrulama kodunun süresi doldu."
                : "Giriş isteği iptal edildi.",
          );
          return;
        }
        interval = result.retryAfter;
      } catch (error) {
        if (!active) return;
        setError(String(error));
        interval = Math.max(interval, 10);
      }
      if (active)
        timer = window.setTimeout(
          () => void poll(),
          Math.max(1, interval) * 1000,
        );
    };
    timer = window.setTimeout(() => void poll(), flow.interval * 1000);
    return () => {
      active = false;
      window.clearTimeout(timer);
    };
  }, [flow]);
  async function start() {
    const attempt = ++generation.current;
    setWorking(true);
    setError("");
    setMessage("");
    try {
      if (clientId.trim() !== (github.oauthClientId ?? ""))
        await githubService.saveClientId(clientId.trim());
      const result = await githubService.authStart();
      if (generation.current !== attempt) {
        await githubService.authCancel(result.flowId);
        return;
      }
      clientDirty.current = false;
      setFlow(result);
      await githubService.authOpen();
    } catch (error) {
      if (generation.current === attempt) setError(String(error));
    } finally {
      if (generation.current === attempt) setWorking(false);
    }
  }
  async function cancel() {
    if (!flow) return;
    generation.current++;
    setWorking(true);
    try {
      await githubService.authCancel(flow.flowId);
      setFlow(null);
      setError("");
    } catch (error) {
      setError(String(error));
    } finally {
      setWorking(false);
    }
  }
  return (
    <div className="github-connect">
      {!flow ? (
        <>
          <div className="github-account-row">
            <div>
              <strong>
                {github.tokenSaved
                  ? github.login || "GitHub hesabı bağlı"
                  : "GitHub hesabınızı bağlayın"}
              </strong>
              <p className="section-note">
                {github.tokenSaved
                  ? "Depolarınızı ve dallarınızı doğrudan seçin."
                  : "Tarayıcıda onay verin; jeton otomatik ve şifreli kaydedilir."}
              </p>
            </div>
            <button
              type="button"
              className="button primary"
              disabled={disabled || working || !clientId.trim()}
              onClick={() => void start()}
            >
              {working ? (
                <LoaderCircle size={17} className="spin" />
              ) : (
                <Github size={17} />
              )}
              {github.tokenSaved ? "Hesabı değiştir" : "GitHub ile giriş yap"}
            </button>
          </div>
          <details
            className="github-client-settings"
            open={!github.oauthClientId || undefined}
          >
            <summary>OAuth uygulama ayarı</summary>
            <label>
              Client ID
              <input
                value={clientId}
                autoComplete="off"
                spellCheck={false}
                disabled={working || disabled}
                onChange={(e) => {
                  clientDirty.current = true;
                  setClientId(e.target.value);
                }}
                placeholder="ServerBond OAuth App Client ID"
              />
            </label>
            <p className="field-hint">
              GitHub OAuth App’te Device Flow etkin olmalı. Client Secret
              kullanılmaz.
            </p>
          </details>
        </>
      ) : (
        <div className="github-device-flow" role="status">
          <strong>GitHub’da bu kodu onaylayın</strong>
          <div className="github-device-code">
            <code>{flow.userCode}</code>
            <button
              type="button"
              className="button secondary small"
              onClick={() =>
                void navigator.clipboard
                  .writeText(flow.userCode)
                  .then(() => setMessage("Kod kopyalandı."))
                  .catch((error) => setError(String(error)))
              }
            >
              <Copy size={16} /> Kodu kopyala
            </button>
          </div>
          <p className="section-note">
            Onay bekleniyor · Son geçerlilik{" "}
            {new Date(flow.expiresAt * 1000).toLocaleTimeString("tr-TR", {
              hour: "2-digit",
              minute: "2-digit",
            })}
          </p>
          <div className="settings-actions">
            <button
              type="button"
              className="button primary small"
              onClick={() =>
                void githubService
                  .authOpen()
                  .catch((error) => setError(String(error)))
              }
            >
              <ExternalLink size={16} /> GitHub’ı aç
            </button>
            <button
              type="button"
              className="button secondary small"
              disabled={working}
              onClick={() => void cancel()}
            >
              Girişi iptal et
            </button>
          </div>
        </div>
      )}
      {message ? (
        <p className="section-note" role="status">
          {message}
        </p>
      ) : null}
      {error ? (
        <p className="field-error" role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}
