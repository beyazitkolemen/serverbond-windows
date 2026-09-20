import { useState } from "react";
import { ChevronLeft, Settings } from "lucide-react";
import { call } from "../api";
import type {
  Settings as Values,
  Run,
  TunnelState,
  MailState,
  PostgresState,
  RedisState,
  GithubState,
} from "../types";
import MailActions from "./MailActions";
import PostgresSettings from "./PostgresSettings";
import RedisActions from "./RedisActions";
import GithubSettings from "./GithubSettings";
import TunnelSettings from "./TunnelSettings";

const catalog = [
  {
    id: "pma",
    title: "phpMyAdmin",
    copy: "MySQL veritabanlarını tarayıcıdan yönetin.",
  },
  {
    id: "mail",
    title: "E-posta",
    copy: "Mailpit yerel SMTP yakalayıcı ve gelen kutusu.",
  },
  {
    id: "postgres",
    title: "PostgreSQL",
    copy: "İsteğe bağlı PostgreSQL 17 · Laravel pgsql.",
  },
  {
    id: "redis",
    title: "Redis",
    copy: "Kuyruk, önbellek ve oturum için Redis 8.",
  },
  {
    id: "github",
    title: "GitHub",
    copy: "Özel depolar için bir kez jeton kaydı.",
  },
  {
    id: "tunnel",
    title: "Tünel",
    copy: "Cloudflare Tunnel ile dışarı açın.",
  },
] as const;

type ServiceId = (typeof catalog)[number]["id"];

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
  redis,
  github,
}: {
  settings: Values;
  busy: boolean;
  running: boolean;
  run: Run;
  tunnel: TunnelState;
  mail: MailState;
  postgres: PostgresState;
  redis: RedisState;
  github: GithubState;
}) {
  const [values, setValues] = useState(() => structuredClone(settings));
  const [service, setService] = useState<ServiceId | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const dirty = JSON.stringify(values) !== JSON.stringify(settings);
  const locked = busy || running;
  const canSave = dirty && !busy && !running;
  const change = <K extends keyof Values>(key: K, value: Values[K]) =>
    setValues((v) => ({ ...v, [key]: value }));
  const selected = catalog.find((item) => item.id === service);
  const openService = (id: ServiceId) => {
    setValues(structuredClone(settings));
    setService(id);
    setSettingsOpen(false);
  };
  const closeSettings = () => {
    setValues(structuredClone(settings));
    setSettingsOpen(false);
  };
  const tileState = (id: ServiceId) => {
    switch (id) {
      case "pma":
        return {
          on: settings.phpmyadmin.enabled,
          label: settings.phpmyadmin.enabled ? "Açık" : "Kapalı",
        };
      case "mail":
        return {
          on: mail.running,
          label: mail.running
            ? "Çalışıyor"
            : mail.installed
              ? "Kurulu"
              : "Kurulmadı",
        };
      case "postgres":
        return {
          on: postgres.running,
          label: postgres.running
            ? "Çalışıyor"
            : postgres.installed
              ? "Kurulu"
              : "Kurulmadı",
        };
      case "redis":
        return {
          on: redis.running,
          label: redis.running
            ? "Çalışıyor"
            : redis.installed
              ? "Kurulu"
              : "Kurulmadı",
        };
      case "github":
        return {
          on: github.tokenSaved,
          label: github.tokenSaved
            ? github.login
              ? github.login
              : "Jeton kayıtlı"
            : "Jeton yok",
        };
      case "tunnel":
        return {
          on: tunnel.running,
          label: tunnel.running
            ? "Çalışıyor"
            : tunnel.installed
              ? "Kurulu"
              : "Kurulmadı",
        };
    }
  };
  return (
    <div className="settings-layout">
      {!selected ? (
        <>
          <p className="section-note">
            İsteğe bağlı hizmetler ortamın çalışması için gerekli değildir. Bir
            hizmete girin; port ve jeton Ayarlar düğmesindedir.
          </p>
          <div className="service-grid">
            {catalog.map((item) => {
              const state = tileState(item.id);
              return (
                <button
                  type="button"
                  key={item.id}
                  className="service-tile"
                  data-service={item.title}
                  aria-label={`${item.title} hizmetini aç`}
                  onClick={() => openService(item.id)}
                >
                  <span className="service-tile-head">
                    <strong>{item.title}</strong>
                    <span
                      className={`service-status ${state.on ? "running" : ""}`}
                    >
                      <span
                        className={`status-dot ${state.on ? "green" : ""}`}
                      />
                      {state.label}
                    </span>
                  </span>
                  <span className="service-tile-copy">{item.copy}</span>
                </button>
              );
            })}
          </div>
        </>
      ) : (
        <>
          <div className="service-toolbar">
            <button
              type="button"
              className="button secondary small"
              onClick={() => {
                if (settingsOpen) {
                  closeSettings();
                  return;
                }
                setService(null);
                setSettingsOpen(false);
              }}
            >
              <ChevronLeft size={16} />
              {settingsOpen ? selected.title : "Hizmetler"}
            </button>
            <h2 className="service-toolbar-title">
              {settingsOpen ? `${selected.title} ayarları` : selected.title}
            </h2>
            {!settingsOpen ? (
              <button
                type="button"
                className="button secondary small service-gear"
                onClick={() => {
                  setValues(structuredClone(settings));
                  setSettingsOpen(true);
                }}
              >
                <Settings size={16} />
                Ayarlar
              </button>
            ) : null}
          </div>
          {settingsOpen &&
          (service === "pma" ||
            service === "mail" ||
            service === "postgres" ||
            service === "redis") ? (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void run("Hizmet ayarları doğrulanıyor ve kaydediliyor…", () =>
                  call("save_settings", { settings: values }),
                );
              }}
            >
              <p className="section-note">
                {running
                  ? "Bu ayarları kaydetmek için önce ortamı durdurun."
                  : "Kaydedilen portlar sonraki başlangıçta uygulanır."}
              </p>
              <fieldset disabled={locked} className="settings-fields">
                {service === "pma" ? (
                  <PmaForm values={values} change={change} />
                ) : null}
                {service === "mail" ? (
                  <MailForm values={values} change={change} />
                ) : null}
                {service === "postgres" ? (
                  <PostgresForm values={values} change={change} />
                ) : null}
                {service === "redis" ? (
                  <RedisForm values={values} change={change} />
                ) : null}
              </fieldset>
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
            </form>
          ) : settingsOpen && service === "github" ? (
            <GithubSettings
              github={github}
              busy={busy}
              run={run}
              pane="settings"
            />
          ) : settingsOpen && service === "tunnel" ? (
            <TunnelSettings
              tunnel={tunnel}
              busy={busy}
              run={run}
              pane="settings"
            />
          ) : (
            <>
              {service === "pma" ? (
                <PmaActions
                  settings={settings}
                  busy={busy}
                  running={running}
                  run={run}
                />
              ) : null}
              {service === "mail" ? (
                <MailActions mail={mail} busy={busy} run={run} />
              ) : null}
              {service === "postgres" ? (
                <PostgresSettings postgres={postgres} busy={busy} run={run} />
              ) : null}
              {service === "redis" ? (
                <RedisActions redis={redis} busy={busy} run={run} />
              ) : null}
              {service === "github" ? (
                <GithubSettings
                  github={github}
                  busy={busy}
                  run={run}
                  pane="ops"
                />
              ) : null}
              {service === "tunnel" ? (
                <TunnelSettings
                  tunnel={tunnel}
                  busy={busy}
                  run={run}
                  pane="ops"
                />
              ) : null}
            </>
          )}
        </>
      )}
    </div>
  );
}

function PmaActions({
  settings,
  busy,
  running,
  run,
}: {
  settings: Values;
  busy: boolean;
  running: boolean;
  run: Run;
}) {
  const url = settings.web.https
    ? `https://phpmyadmin.f4box.localhost:${settings.web.httpsPort}`
    : `http://phpmyadmin.f4box.localhost:${settings.webPort}`;
  return (
    <section className="settings-section">
      <h2>phpMyAdmin</h2>
      <div className="tunnel-status">
        <span
          className={`status-dot ${settings.phpmyadmin.enabled ? "on" : "off"}`}
        />
        <div>
          <strong>
            {settings.phpmyadmin.enabled
              ? "Web erişimi açık"
              : "Web erişimi kapalı"}
          </strong>
          <p className="section-note">{url}</p>
        </div>
      </div>
      <div className="settings-actions">
        <button
          type="button"
          className="button secondary"
          disabled={busy || !settings.phpmyadmin.enabled || !running}
          title={
            !settings.phpmyadmin.enabled
              ? "Ayarlar’dan phpMyAdmin erişimini açın"
              : running
                ? "phpMyAdmin'i tarayıcıda aç"
                : "Önce ortamı başlatın"
          }
          onClick={() =>
            void run("phpMyAdmin açılıyor…", () => call("open_phpmyadmin"))
          }
        >
          Aç
        </button>
      </div>
      <p className="section-note">
        Dil, satır sayısı ve oturum süresi Ayarlar düğmesindedir. Giriş:{" "}
        <strong>root</strong>; MySQL parolası Ayarlar → Sistem’de.
      </p>
    </section>
  );
}

function PmaForm({
  values,
  change,
}: {
  values: Values;
  change: <K extends keyof Values>(key: K, value: Values[K]) => void;
}) {
  return (
    <section className="settings-section">
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
    </section>
  );
}

function MailForm({
  values,
  change,
}: {
  values: Values;
  change: <K extends keyof Values>(key: K, value: Values[K]) => void;
}) {
  return (
    <section className="settings-section">
      <div className="settings-grid">
        <NumberField
          label="SMTP portu"
          value={values.mail.smtpPort}
          min={1}
          max={65535}
          onChange={(v) => change("mail", { ...values.mail, smtpPort: v })}
        />
        <NumberField
          label="Arayüz portu"
          value={values.mail.webPort}
          min={1}
          max={65535}
          onChange={(v) => change("mail", { ...values.mail, webPort: v })}
        />
        <NumberField
          label="Saklanacak e-posta (0: sınırsız)"
          value={values.mail.maxMessages}
          min={0}
          max={100000}
          onChange={(v) => change("mail", { ...values.mail, maxMessages: v })}
        />
      </div>
      <Toggle
        label="Ortam başlatıldığında Mailpit'i de başlat"
        value={values.mail.autoStart}
        onChange={(v) => change("mail", { ...values.mail, autoStart: v })}
      />
      <Toggle
        label="PHP mail() çağrılarını Mailpit'e yönlendir"
        value={values.mail.relayPhpMail}
        onChange={(v) => change("mail", { ...values.mail, relayPhpMail: v })}
      />
    </section>
  );
}

function PostgresForm({
  values,
  change,
}: {
  values: Values;
  change: <K extends keyof Values>(key: K, value: Values[K]) => void;
}) {
  return (
    <section className="settings-section">
      <div className="settings-grid">
        <NumberField
          label="Port"
          value={values.postgres.port}
          min={1}
          max={65535}
          onChange={(v) => change("postgres", { ...values.postgres, port: v })}
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
  );
}

function RedisForm({
  values,
  change,
}: {
  values: Values;
  change: <K extends keyof Values>(key: K, value: Values[K]) => void;
}) {
  return (
    <section className="settings-section">
      <div className="settings-grid">
        <NumberField
          label="Port"
          value={values.redis.port}
          min={1}
          max={65535}
          onChange={(v) => change("redis", { ...values.redis, port: v })}
        />
      </div>
      <Toggle
        label="Ortam başlatıldığında Redis'i de başlat"
        value={values.redis.autoStart}
        onChange={(v) => change("redis", { ...values.redis, autoStart: v })}
      />
    </section>
  );
}
