import { useState } from "react";
import { Eye, EyeOff, Trash2 } from "lucide-react";
import { call } from "../api";
import type { GithubState, Run } from "../types";

export default function GithubSettings({
  github,
  busy,
  run,
}: {
  github: GithubState;
  busy: boolean;
  run: Run;
}) {
  const [token, setToken] = useState("");
  const [visible, setVisible] = useState(false);
  return (
    <section className="settings-section">
      <h2>GitHub hesabı</h2>
      <div className="tunnel-status">
        <span className={`status-dot ${github.tokenSaved ? "on" : "off"}`} />
        <div>
          <strong>
            {github.tokenSaved
              ? github.login
                ? `Bağlı: ${github.login}`
                : "GitHub jetonu kayıtlı"
              : "GitHub jetonu yok"}
          </strong>
          <p className="section-note">
            Özel depolar için bir kez kaydedilir. Jeton Windows hesabınıza bağlı
            olarak şifrelenir; komut satırına yazılmaz.
          </p>
        </div>
      </div>
      <label>
        Kişisel erişim jetonu
        <div className="input-with-button">
          <input
            type={visible ? "text" : "password"}
            autoComplete="off"
            spellCheck={false}
            placeholder="ghp_… veya github_pat_…"
            value={token}
            disabled={busy}
            onChange={(e) => setToken(e.target.value)}
          />
          <button
            type="button"
            className="icon-button"
            disabled={busy}
            aria-label={visible ? "Jetonu gizle" : "Jetonu göster"}
            onClick={() => setVisible((v) => !v)}
          >
            {visible ? <EyeOff size={18} /> : <Eye size={18} />}
          </button>
        </div>
      </label>
      <div className="settings-actions">
        <button
          type="button"
          className="button primary"
          disabled={busy || !token.trim()}
          onClick={() =>
            void run("GitHub jetonu kaydediliyor…", async () => {
              await call("github", { action: "save", token });
              setToken("");
              return "GitHub hesabı kaydedildi. Proje ekle → GitHub ile klonlayabilirsiniz.";
            })
          }
        >
          Jetonu kaydet
        </button>
        <button
          type="button"
          className="button secondary"
          disabled={busy || !github.tokenSaved}
          onClick={() =>
            void run("GitHub jetonu siliniyor…", () =>
              call("github", { action: "forget" }),
            )
          }
        >
          <Trash2 size={16} />
          Unut
        </button>
      </div>
      <p className="section-note">
        GitHub → Settings → Developer settings → Personal access tokens. Klasik
        jeton için <code>repo</code> yetkisi; ince ayarlı jeton için Contents
        okuma yeterlidir. Genel depolar jeton olmadan da klonlanır. Komut:{" "}
        <code>f4box github token|forget|import</code>.
      </p>
    </section>
  );
}
