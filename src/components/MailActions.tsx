import { mailService } from "../services";
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
  return (
    <section className="settings-section">
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
        action={() => mailService.repair()}
      />
      <details className="connection-help">
        <summary>Laravel bağlantısı</summary>
        <p className="section-note">
          <code>.env</code>: <code>MAIL_MAILER=smtp</code>,{" "}
          <code>MAIL_HOST=127.0.0.1</code> ve{" "}
          <code>MAIL_PORT={mail.smtpPort}</code> yazın; kullanıcı ve parola
          gerekmez. Yakalanan e-postalar dışarı gönderilmez.
        </p>
      </details>
    </section>
  );
}
