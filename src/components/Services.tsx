import { useState } from "react";
import { call } from "../api";
import type {
  Settings as Values,
  Run,
  TunnelState,
  MailState,
  PostgresState,
  GithubState,
} from "../types";
import MailActions from "./MailActions";
import PostgresSettings from "./PostgresSettings";
import GithubSettings from "./GithubSettings";
import TunnelSettings from "./TunnelSettings";

const sections = [
  "phpMyAdmin",
  "E-posta",
  "PostgreSQL",
  "GitHub",
  "Tünel",
] as const;

function NumberField({
  label,
  value,
  onChange,
  min = 1,
  max = 65535,
}: {
  label: string;
  value: number;
  onChange: (n: number) => void;
  min?: number;
  max?: number;
}) {
  return (
    <label>
      {label}
      <input
        required
        type="number"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </label>
  );
}

function Toggle({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="setting-toggle">
      <input
        type="checkbox"
        checked={value}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span>{label}</span>
    </label>
  );
}

export default function Services({
  settings,
  busy,
  running,
  run,
  tunnel,
  mail,
  postgres,
  github,
}: {
  settings: Values;
  busy: boolean;
  running: boolean;
  run: Run;
  tunnel: TunnelState;
  mail: MailState;
  postgres: PostgresState;
  github: GithubState;
}) {
  const [values, setValues] = useState(() => structuredClone(settings));
  const [section, setSection] =
    useState<(typeof sections)[number]>("phpMyAdmin");
  const dirty = JSON.stringify(values) !== JSON.stringify(settings);
  const locked = busy || running;
  const canSave = dirty && !busy && !running;
  const change = <K extends keyof Values>(key: K, value: Values[K]) =>
    setValues((v) => ({ ...v, [key]: value }));
  return (
    <div className="settings-layout">
      <nav className="settings-tabs" aria-label="Hizmet bölümleri">
        <div className="settings-tab-row">
          {sections.map((s) => (
            <button
              type="button"
              key={s}
              aria-current={s === section ? "page" : undefined}
              onClick={() => setSection(s)}
            >
              {s}
            </button>
          ))}
        </div>
      </nav>
      <p className="section-note">
        {running
          ? "phpMyAdmin, e-posta ve PostgreSQL ayarlarını kaydetmek için önce ortamı durdurun. GitHub ve tünel jetonu ortam çalışırken de yazılır."
          : "İsteğe bağlı hizmetler ortamın çalışması için gerekli değildir. Kaydedilen portlar sonraki başlangıçta uygulanır."}
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void run("Hizmet ayarları doğrulanıyor ve kaydediliyor…", () =>
            call("save_settings", { settings: values }),
          );
        }}
      >
        <fieldset disabled={locked} className="settings-fields">
          {section === "phpMyAdmin" && (
            <section className="settings-section">
              <h2>phpMyAdmin</h2>
              <Toggle
                label="phpMyAdmin web erişimi etkin"
                value={values.phpmyadmin.enabled}
                onChange={(v) =>
                  change("phpmyadmin", { ...values.phpmyadmin, enabled: v })
                }
              />
              <div className="settings-grid">
                <label>
                  Varsayılan dil
                  <select
                    value={values.phpmyadmin.language}
                    onChange={(e) =>
                      change("phpmyadmin", {
                        ...values.phpmyadmin,
                        language: e.target.value,
                      })
                    }
                  >
                    {Object.entries({
                      tr: "Türkçe",
                      en: "English",
                      de: "Deutsch",
                      fr: "Français",
                      es: "Español",
                      it: "Italiano",
                      pt: "Português",
                      ru: "Русский",
                      ar: "العربية",
                      ja: "日本語",
                      zh_CN: "简体中文",
                    }).map(([v, l]) => (
                      <option key={v} value={v}>
                        {l}
                      </option>
                    ))}
                  </select>
                </label>
                <NumberField
                  label="Sayfa başına satır"
                  value={values.phpmyadmin.rows}
                  min={10}
                  max={1000}
                  onChange={(v) =>
                    change("phpmyadmin", { ...values.phpmyadmin, rows: v })
                  }
                />
                <NumberField
                  label="Oturum süresi (sn)"
                  value={values.phpmyadmin.loginSeconds}
                  min={60}
                  max={86400}
                  onChange={(v) =>
                    change("phpmyadmin", {
                      ...values.phpmyadmin,
                      loginSeconds: v,
                    })
                  }
                />
              </div>
              <p className="section-note">
                Adres:{" "}
                {values.web.https
                  ? `https://phpmyadmin.f4box.localhost:${values.web.httpsPort}`
                  : `http://phpmyadmin.f4box.localhost:${values.webPort}`}
                . Dosyalar bileşen kuruluysa sunulur. Yükleme limitleri
                varsayılan PHP sürümünün profilinden alınır.
              </p>
            </section>
          )}
          {section === "E-posta" && (
            <section className="settings-section">
              <h2>Yerel e-posta yakalama</h2>
              <p className="section-note">
                Mailpit, projelerinizin gönderdiği e-postaları yerel bir SMTP
                sunucusunda tutar ve tarayıcıda gösterir. Hiçbir ileti gerçek
                alıcıya iletilmez.
              </p>
              <div className="settings-grid">
                <NumberField
                  label="SMTP portu"
                  value={values.mail.smtpPort}
                  min={1}
                  max={65535}
                  onChange={(v) =>
                    change("mail", { ...values.mail, smtpPort: v })
                  }
                />
                <NumberField
                  label="Arayüz portu"
                  value={values.mail.webPort}
                  min={1}
                  max={65535}
                  onChange={(v) =>
                    change("mail", { ...values.mail, webPort: v })
                  }
                />
                <NumberField
                  label="Saklanacak e-posta (0: sınırsız)"
                  value={values.mail.maxMessages}
                  min={0}
                  max={100000}
                  onChange={(v) =>
                    change("mail", { ...values.mail, maxMessages: v })
                  }
                />
              </div>
              <Toggle
                label="Ortam başlatıldığında Mailpit'i de başlat"
                value={values.mail.autoStart}
                onChange={(v) =>
                  change("mail", { ...values.mail, autoStart: v })
                }
              />
              <Toggle
                label="PHP mail() çağrılarını Mailpit'e yönlendir"
                value={values.mail.relayPhpMail}
                onChange={(v) =>
                  change("mail", { ...values.mail, relayPhpMail: v })
                }
              />
              <p className="section-note">
                Yönlendirme php.ini içindeki SMTP ayarlarını üretir; bu yüzden
                aynı anahtarlar Ayarlar → PHP ek ayarlar alanına yazılamaz.
                Laravel kendi <code>.env</code> dosyasını okur, bu anahtarları
                kullanmaz. Değişiklik PHP yeniden başladığında geçerli olur.
              </p>
            </section>
          )}
          {section === "PostgreSQL" && (
            <section className="settings-section">
              <h2>İsteğe bağlı PostgreSQL</h2>
              <p className="section-note">
                MySQL varsayılan kalır. İsterseniz aynı Windows makinesinde
                PostgreSQL 17 de kurulur; ortamı bloke etmez. <code>.env</code>{" "}
                yazılmaz.
              </p>
              <div className="settings-grid">
                <NumberField
                  label="Port"
                  value={values.postgres.port}
                  min={1}
                  max={65535}
                  onChange={(v) =>
                    change("postgres", { ...values.postgres, port: v })
                  }
                />
              </div>
              <Toggle
                label="Ortam başlatıldığında PostgreSQL'i de başlat"
                value={values.postgres.autoStart}
                onChange={(v) =>
                  change("postgres", { ...values.postgres, autoStart: v })
                }
              />
            </section>
          )}
        </fieldset>
        {!["Tünel", "GitHub"].includes(section) && (
          <div className="settings-save">
            <span>
              {dirty
                ? running
                  ? "Hizmet ayarlarını kaydetmek için ortamı durdurun"
                  : "Kaydedilmemiş değişiklikler var"
                : "Ayarlar güncel"}
            </span>
            <button
              type="button"
              className="button secondary"
              disabled={!dirty || busy}
              onClick={() => setValues(structuredClone(settings))}
            >
              Vazgeç
            </button>
            <button
              type="submit"
              className="button primary"
              disabled={!canSave}
            >
              Ayarları kaydet
            </button>
          </div>
        )}
      </form>
      {section === "E-posta" && (
        <MailActions mail={mail} busy={busy} run={run} />
      )}
      {section === "PostgreSQL" && (
        <PostgresSettings postgres={postgres} busy={busy} run={run} />
      )}
      {section === "GitHub" && (
        <GithubSettings github={github} busy={busy} run={run} />
      )}
      {section === "Tünel" && (
        <TunnelSettings tunnel={tunnel} busy={busy} run={run} />
      )}
    </div>
  );
}
