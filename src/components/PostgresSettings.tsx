import { useState } from "react";
import {
  Play,
  Square,
  Download,
  Wrench,
  Eye,
  EyeOff,
  Copy,
} from "lucide-react";
import { call } from "../api";
import type { PostgresState, Run } from "../types";

export default function PostgresSettings({
  postgres,
  busy,
  run,
}: {
  postgres: PostgresState;
  busy: boolean;
  run: Run;
}) {
  const [password, setPassword] = useState("");
  const [nextPassword, setNextPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showNext, setShowNext] = useState(false);
  const action = (name: string, message: string) =>
    void run(message, () => call("postgres", { action: name }));
  return (
    <section className="settings-section">
      <h2>PostgreSQL servisi</h2>
      <div className="tunnel-status">
        <span className={`status-dot ${postgres.running ? "on" : "off"}`} />
        <div>
          <strong>
            {postgres.running
              ? `PostgreSQL çalışıyor · PID ${postgres.pid ?? "-"}`
              : postgres.installed
                ? "PostgreSQL durdu"
                : "PostgreSQL kurulu değil"}
          </strong>
          <p className="section-note">
            PostgreSQL {postgres.version} · 127.0.0.1:{postgres.port} ·
            kullanıcı <code>postgres</code>
          </p>
        </div>
      </div>
      {postgres.issue ? (
        <p role="alert" className="settings-feedback">
          {postgres.issue}
        </p>
      ) : null}
      <div className="settings-actions">
        {!postgres.installed ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() => action("install", "PostgreSQL indiriliyor…")}
          >
            <Download size={16} />
            PostgreSQL kur
          </button>
        ) : (
          <>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() =>
                action(
                  postgres.running ? "stop" : "start",
                  postgres.running
                    ? "PostgreSQL durduruluyor…"
                    : "PostgreSQL başlatılıyor…",
                )
              }
            >
              {postgres.running ? <Square size={16} /> : <Play size={16} />}
              {postgres.running ? "Durdur" : "Başlat"}
            </button>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => action("repair", "PostgreSQL onarılıyor…")}
            >
              <Wrench size={16} />
              Onar
            </button>
          </>
        )}
        <button
          type="button"
          className="button secondary"
          disabled={busy || !postgres.passwordSaved}
          aria-label={password ? "Parolayı gizle" : "Parolayı göster"}
          onClick={() => {
            if (password) {
              setPassword("");
              return;
            }
            void run("Parola açılıyor…", async () =>
              setPassword(
                await call<string>("postgres", { action: "credentials" }),
              ),
            );
          }}
        >
          {password ? <EyeOff size={16} /> : <Eye size={16} />}
          {password ? "Gizle" : "Parolayı göster"}
        </button>
        <button
          type="button"
          className="button secondary"
          disabled={busy || !postgres.passwordSaved}
          onClick={() =>
            void run("Parola kopyalanıyor…", async () => {
              const value =
                password ||
                (await call<string>("postgres", { action: "credentials" }));
              await navigator.clipboard.writeText(value);
              return "Parola panoya kopyalandı.";
            })
          }
        >
          <Copy size={16} />
          Kopyala
        </button>
      </div>
      {password ? (
        <p className="section-note">
          <code>{password}</code>
        </p>
      ) : null}
      {postgres.installed ? (
        <>
          <h3>Parolayı değiştir</h3>
          <p className="section-note">
            PostgreSQL çalışırken <code>postgres</code> kullanıcısının
            parolasını buradan değiştirin. 8–128 karakter; boşluk ve tırnak
            kullanmayın.
          </p>
          <div className="settings-grid">
            <label>
              Yeni parola
              <div className="input-with-button">
                <input
                  type={showNext ? "text" : "password"}
                  autoComplete="new-password"
                  value={nextPassword}
                  disabled={busy}
                  onChange={(e) => setNextPassword(e.target.value)}
                />
                <button
                  type="button"
                  className="icon-button"
                  disabled={busy}
                  aria-label={showNext ? "Parolayı gizle" : "Parolayı göster"}
                  onClick={() => setShowNext((v) => !v)}
                >
                  {showNext ? <EyeOff size={18} /> : <Eye size={18} />}
                </button>
              </div>
            </label>
            <label>
              Yeni parolayı doğrula
              <input
                type={showNext ? "text" : "password"}
                autoComplete="new-password"
                value={confirmPassword}
                disabled={busy}
                onChange={(e) => setConfirmPassword(e.target.value)}
              />
            </label>
          </div>
          <div className="settings-actions">
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() => {
                const generated = Array.from(
                  crypto.getRandomValues(new Uint8Array(16)),
                )
                  .map((n) => n.toString(16).padStart(2, "0"))
                  .join("");
                setNextPassword(generated);
                setConfirmPassword(generated);
                setShowNext(true);
              }}
            >
              Rastgele üret
            </button>
            <button
              type="button"
              className="button primary"
              disabled={
                busy || !nextPassword || nextPassword !== confirmPassword
              }
              onClick={() =>
                void run("PostgreSQL parolası güncelleniyor…", async () => {
                  if (!postgres.running)
                    throw new Error(
                      "Parolayı değiştirmek için önce PostgreSQL'i başlatın.",
                    );
                  if (nextPassword !== confirmPassword)
                    throw new Error("Parola doğrulaması eşleşmiyor.");
                  await call("postgres", {
                    action: "password",
                    password: nextPassword,
                  });
                  setPassword(nextPassword);
                  setNextPassword("");
                  setConfirmPassword("");
                  setShowNext(false);
                  return "PostgreSQL parolası güncellendi. Proje .env dosyası yazılmadı.";
                })
              }
            >
              Parolayı kaydet
            </button>
          </div>
        </>
      ) : null}
      <p className="section-note">
        MySQL varsayılan kalır; PostgreSQL isteğe bağlıdır ve ortamı bloke
        etmez. Laravel <code>DB_CONNECTION=pgsql</code>,{" "}
        <code>DB_HOST=127.0.0.1</code>, <code>DB_PORT={postgres.port}</code>,{" "}
        <code>DB_USERNAME=postgres</code> değerlerini kendi <code>.env</code>{" "}
        dosyanızda tanımlarsınız; F4Box yazmaz. PHP <code>pgsql</code> /{" "}
        <code>pdo_pgsql</code> uzantılarını PHP sekmesinden açın. Komut:{" "}
        <code>f4box postgres install|start|stop</code>.
      </p>
    </section>
  );
}
