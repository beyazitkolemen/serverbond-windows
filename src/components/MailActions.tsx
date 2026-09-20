import { Play, Square, Download, ExternalLink } from "lucide-react";
import { call } from "../api";
import type { MailState, Run } from "../types";
import ServiceRepair from "./ServiceRepair";

export default function MailActions({
  mail,
  busy,
  run,
}: {
  mail: MailState;
  busy: boolean;
  run: Run;
}) {
  const action = (name: string, message: string) =>
    void run(message, () => call("mail", { action: name }));
  return (
    <section className="settings-section">
      <h2>Mailpit servisi</h2>
      <div className="tunnel-status">
        <span className={`status-dot ${mail.running ? "on" : "off"}`} />
        <div>
          <strong>
            {mail.running
              ? `Mailpit çalışıyor · PID ${mail.pid ?? "-"}`
              : mail.installed
                ? "Mailpit durdu"
                : mail.repairable
                  ? "Kurulum eksik"
                  : "Mailpit kurulu değil"}
          </strong>
          <p className="section-note">
            Mailpit {mail.version} · SMTP 127.0.0.1:{mail.smtpPort} · arayüz
            http://127.0.0.1:{mail.webPort}
          </p>
        </div>
      </div>
      {mail.issue && mail.installed ? (
        <p role="alert" className="settings-feedback">
          {mail.issue}
        </p>
      ) : null}
      <div className="settings-actions">
        {!mail.installed && !mail.repairable ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() => action("install", "Mailpit indiriliyor…")}
          >
            <Download size={16} />
            Mailpit kur
          </button>
        ) : mail.installed ? (
          <>
            <button
              type="button"
              className="button secondary"
              disabled={busy}
              onClick={() =>
                action(
                  mail.running ? "stop" : "start",
                  mail.running
                    ? "Mailpit durduruluyor…"
                    : "Mailpit başlatılıyor…",
                )
              }
            >
              {mail.running ? <Square size={16} /> : <Play size={16} />}
              {mail.running ? "Mailpit'i durdur" : "Mailpit'i başlat"}
            </button>
            <button
              type="button"
              className="button secondary"
              disabled={busy || !mail.running}
              title={
                mail.running
                  ? "Gelen kutusunu tarayıcıda aç"
                  : "Önce Mailpit'i başlatın"
              }
              onClick={() => action("open", "Gelen kutusu açılıyor…")}
            >
              <ExternalLink size={16} />
              Gelen kutusu
            </button>
          </>
        ) : null}
      </div>
      <ServiceRepair
        name="Mailpit"
        installed={mail.installed}
        repairable={mail.repairable}
        running={mail.running}
        issue={mail.issue}
        busy={busy}
        run={run}
        keeps={[
          "Yakalanan e-postalar (data/mailpit)",
          "SMTP, arayüz portu ve otomatik başlatma",
          "Proje .env dosyaları",
        ]}
        action={() => call("mail", { action: "repair" })}
      />
      <p className="section-note">
        SMTP, arayüz portu ve otomatik başlatma Ayarlar düğmesindedir. Laravel
        için <code>.env</code> dosyasına <code>MAIL_MAILER=smtp</code>,{" "}
        <code>MAIL_HOST=127.0.0.1</code> ve{" "}
        <code>MAIL_PORT={mail.smtpPort}</code> yazın; kullanıcı ve parola
        gerekmez. Yakalanan e-postalar dışarı gönderilmez.
      </p>
    </section>
  );
}
