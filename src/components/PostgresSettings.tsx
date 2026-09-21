import { useState } from "react";
import { Eye, EyeOff, Copy } from "lucide-react";
import { postgresService } from "../services";
import type { PostgresState, Run } from "../types";
import ServiceRepair from "./ServiceRepair";

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
  return (
    <section className="settings-section">
      <h3>Parola</h3>
      <div className="settings-actions">
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
              setPassword(await postgresService.credentials()),
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
              const value = password || (await postgresService.credentials());
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
                  await postgresService.changePassword(nextPassword);
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
      <ServiceRepair
        name="PostgreSQL"
        installed={postgres.installed}
        repairable={postgres.repairable}
        running={postgres.running}
        issue={postgres.issue}
        busy={busy}
        run={run}
        keeps={[
          "Veri dizini (data/postgresql-17)",
          "postgres kullanıcısının parolası",
          "Port ve otomatik başlatma",
          "Proje .env dosyaları",
        ]}
        action={() => postgresService.repair()}
      />
      <details className="connection-help">
        <summary>Laravel bağlantısı</summary>
        <p className="section-note">
          <code>.env</code>: <code>DB_CONNECTION=pgsql</code>,{" "}
          <code>DB_HOST=127.0.0.1</code>, <code>DB_PORT={postgres.port}</code>,{" "}
          <code>DB_USERNAME=postgres</code> değerlerini girin. PHP ayarlarından{" "}
          <code>pgsql</code> / <code>pdo_pgsql</code> uzantılarını açın.
        </p>
      </details>
    </section>
  );
}
