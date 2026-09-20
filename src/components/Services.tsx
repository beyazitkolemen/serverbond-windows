import { useState } from "react";
import { ChevronLeft, Settings } from "lucide-react";
import {
  ComponentId,
  ToolAction,
  ToolCommand,
  WorkspaceService,
} from "../domain";
import type {
  ToolCommand as ToolCommandName,
  WorkspaceService as ServiceId,
} from "../domain";
import {
  mailService,
  packagesService,
  runTool,
  settingsService,
  tunnelService,
} from "../services";
import type {
  Settings as Values,
  Run,
  TunnelState,
  MailState,
  PostgresState,
  RedisState,
  GithubState,
  PackageStatus,
} from "../types";
import ServiceRepair from "./ServiceRepair";
import ServiceConsole, {
  type ConsoleAction,
  type ConsoleKind,
} from "./ServiceConsole";
import MailActions from "./MailActions";
import PostgresSettings from "./PostgresSettings";
import RedisActions from "./RedisActions";
import GithubSettings from "./GithubSettings";
import TunnelSettings from "./TunnelSettings";

const catalog = [
  {
    id: WorkspaceService.PhpMyAdmin,
    title: "phpMyAdmin",
    copy: "MySQL veritabanlarını tarayıcıdan yönetin.",
  },
  {
    id: WorkspaceService.Mail,
    title: "E-posta",
    copy: "Mailpit yerel SMTP yakalayıcı ve gelen kutusu.",
  },
  {
    id: WorkspaceService.Postgres,
    title: "PostgreSQL",
    copy: "İsteğe bağlı PostgreSQL 17 · Laravel pgsql.",
  },
  {
    id: WorkspaceService.Redis,
    title: "Redis",
    copy: "Kuyruk, önbellek ve oturum için Redis 8.",
  },
  {
    id: WorkspaceService.Github,
    title: "GitHub",
    copy: "Özel depolar için bir kez jeton kaydı.",
  },
  {
    id: WorkspaceService.Tunnel,
    title: "Tünel",
    copy: "Cloudflare Tunnel ile dışarı açın.",
  },
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
  redis,
  github,
  phpmyadmin,
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
  phpmyadmin?: PackageStatus;
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
      case WorkspaceService.PhpMyAdmin:
        return {
          on: settings.phpmyadmin.enabled,
          label: settings.phpmyadmin.enabled ? "Açık" : "Kapalı",
        };
      case WorkspaceService.Mail:
        return {
          on: mail.running,
          label: mail.running
            ? "Çalışıyor"
            : mail.installed
              ? "Kurulu"
              : mail.repairable
                ? "Onarım gerekir"
                : "Kurulmadı",
        };
      case WorkspaceService.Postgres:
        return {
          on: postgres.running,
          label: postgres.running
            ? "Çalışıyor"
            : postgres.installed
              ? "Kurulu"
              : postgres.repairable
                ? "Onarım gerekir"
                : "Kurulmadı",
        };
      case WorkspaceService.Redis:
        return {
          on: redis.running,
          label: redis.running
            ? "Çalışıyor"
            : redis.installed
              ? "Kurulu"
              : redis.repairable
                ? "Onarım gerekir"
                : "Kurulmadı",
        };
      case WorkspaceService.Github:
        return {
          on: github.tokenSaved,
          label: github.tokenSaved
            ? github.login
              ? github.login
              : "Jeton kayıtlı"
            : "Jeton yok",
        };
      case WorkspaceService.Tunnel:
        return {
          on: tunnel.running,
          label: tunnel.running
            ? "Çalışıyor"
            : tunnel.installed
              ? "Kurulu"
              : tunnel.repairable
                ? "Onarım gerekir"
                : "Kurulmadı",
        };
    }
  };
  const serviceConsole = (
    id: ServiceId,
  ): {
    kind: ConsoleKind;
    title: string;
    detail: string;
    issue: string | null;
    actions: ConsoleAction[];
  } => {
    const power = (
      installed: boolean,
      repairable: boolean,
      isRunning: boolean,
      command: ToolCommandName,
      start: string,
      stop: string,
    ): ConsoleAction[] => {
      if (!installed && !repairable) {
        return [
          {
            id: ToolAction.Install,
            label: "Kur",
            tone: "primary",
            icon: "install",
            disabled: busy,
            onClick: () =>
              void run(`${start} indiriliyor…`, () =>
                runTool(command, ToolAction.Install),
              ),
          },
        ];
      }
      if (!installed) return [];
      return [
        {
          id: isRunning ? ToolAction.Stop : ToolAction.Start,
          label: isRunning ? "Durdur" : "Başlat",
          tone: isRunning ? "secondary" : "primary",
          icon: isRunning ? "stop" : "play",
          disabled: busy,
          onClick: () =>
            void run(isRunning ? `${stop}…` : `${start} başlatılıyor…`, () =>
              runTool(command, isRunning ? ToolAction.Stop : ToolAction.Start),
            ),
        },
      ];
    };
    const kind = (
      installed: boolean,
      repairable: boolean,
      isRunning: boolean,
    ): ConsoleKind =>
      isRunning
        ? "running"
        : installed
          ? "stopped"
          : repairable
            ? "repair"
            : "missing";
    const title = (
      name: string,
      installed: boolean,
      repairable: boolean,
      isRunning: boolean,
      pid: number | null,
    ) =>
      isRunning
        ? `${name} çalışıyor · PID ${pid ?? "-"}`
        : installed
          ? `${name} durdu`
          : repairable
            ? "Kurulum eksik"
            : `${name} kurulu değil`;
    switch (id) {
      case WorkspaceService.PhpMyAdmin: {
        const enabled = settings.phpmyadmin.enabled;
        const url = settings.web.https
          ? `https://phpmyadmin.f4box.localhost:${settings.web.httpsPort}`
          : `http://phpmyadmin.f4box.localhost:${settings.webPort}`;
        return {
          kind: (enabled
            ? running
              ? "ready"
              : "stopped"
            : "off") satisfies ConsoleKind,
          title: enabled ? "Web erişimi açık" : "Web erişimi kapalı",
          detail: url,
          issue: phpmyadmin?.installed ? phpmyadmin.issue : null,
          actions: [
            {
              id: "open",
              label: "Aç",
              tone: "primary" as const,
              icon: "open" as const,
              disabled: busy || !enabled || !running,
              title: !enabled
                ? "Ayarlar’dan phpMyAdmin erişimini açın"
                : running
                  ? "phpMyAdmin'i tarayıcıda aç"
                  : "Önce ortamı başlatın",
              onClick: () =>
                void run("phpMyAdmin açılıyor…", () =>
                  packagesService.openPhpMyAdmin(),
                ),
            },
          ],
        };
      }
      case WorkspaceService.Mail:
        return {
          kind: kind(mail.installed, mail.repairable, mail.running),
          title: title(
            "Mailpit",
            mail.installed,
            mail.repairable,
            mail.running,
            mail.pid,
          ),
          detail: `Mailpit ${mail.version} · SMTP 127.0.0.1:${mail.smtpPort} · http://127.0.0.1:${mail.webPort}`,
          issue: mail.installed ? mail.issue : null,
          actions: [
            ...power(
              mail.installed,
              mail.repairable,
              mail.running,
              ToolCommand.Mail,
              "Mailpit",
              "Mailpit durduruluyor",
            ),
            ...(mail.installed
              ? [
                  {
                    id: ToolAction.Open,
                    label: "Gelen kutusu",
                    tone: "secondary" as const,
                    icon: "open" as const,
                    disabled: busy || !mail.running,
                    title: mail.running
                      ? "Gelen kutusunu tarayıcıda aç"
                      : "Önce Mailpit'i başlatın",
                    onClick: () =>
                      void run("Gelen kutusu açılıyor…", () =>
                        mailService.open(),
                      ),
                  },
                ]
              : []),
          ],
        };
      case WorkspaceService.Postgres:
        return {
          kind: kind(postgres.installed, postgres.repairable, postgres.running),
          title: title(
            "PostgreSQL",
            postgres.installed,
            postgres.repairable,
            postgres.running,
            postgres.pid,
          ),
          detail: `PostgreSQL ${postgres.version} · 127.0.0.1:${postgres.port} · kullanıcı postgres`,
          issue: postgres.installed ? postgres.issue : null,
          actions: power(
            postgres.installed,
            postgres.repairable,
            postgres.running,
            ToolCommand.Postgres,
            "PostgreSQL",
            "PostgreSQL durduruluyor",
          ),
        };
      case WorkspaceService.Redis:
        return {
          kind: kind(redis.installed, redis.repairable, redis.running),
          title: title(
            "Redis",
            redis.installed,
            redis.repairable,
            redis.running,
            redis.pid,
          ),
          detail: `Redis ${redis.version} · 127.0.0.1:${redis.port} · loopback, parola yok`,
          issue: redis.installed ? redis.issue : null,
          actions: power(
            redis.installed,
            redis.repairable,
            redis.running,
            ToolCommand.Redis,
            "Redis",
            "Redis durduruluyor",
          ),
        };
      case WorkspaceService.Github:
        return {
          kind: github.tokenSaved ? "ready" : "off",
          title: github.tokenSaved
            ? github.login
              ? `Bağlı: ${github.login}`
              : "GitHub jetonu kayıtlı"
            : "GitHub jetonu yok",
          detail: github.tokenSaved
            ? "Özel depolar için bir kez kaydedilir. Jeton Windows hesabınıza bağlıdır."
            : "Jeton kaydedilmedi. Ayarlar’dan kişisel erişim jetonunu yapıştırın.",
          issue: null,
          actions: [] as ConsoleAction[],
        };
      case WorkspaceService.Tunnel:
        return {
          kind: kind(tunnel.installed, tunnel.repairable, tunnel.running),
          title: title(
            "Tünel",
            tunnel.installed,
            tunnel.repairable,
            tunnel.running,
            tunnel.pid,
          ),
          detail: tunnel.tokenSaved
            ? `Cloudflared ${tunnel.version} · jeton kayıtlı`
            : `Cloudflared ${tunnel.version} · jeton yok`,
          issue: tunnel.installed ? tunnel.issue : null,
          actions: [
            ...(!tunnel.installed && !tunnel.repairable
              ? power(
                  false,
                  false,
                  false,
                  ToolCommand.Tunnel,
                  "Cloudflared",
                  "Tünel durduruluyor",
                )
              : tunnel.installed
                ? [
                    {
                      id: tunnel.running ? ToolAction.Stop : ToolAction.Start,
                      label: tunnel.running ? "Durdur" : "Başlat",
                      tone: tunnel.running
                        ? ("secondary" as const)
                        : ("primary" as const),
                      icon: tunnel.running
                        ? ("stop" as const)
                        : ("play" as const),
                      disabled: busy || !tunnel.tokenSaved,
                      title: tunnel.tokenSaved
                        ? undefined
                        : "Önce jetonu Ayarlar’dan kaydedin.",
                      onClick: () =>
                        void run(
                          tunnel.running
                            ? "Tünel durduruluyor…"
                            : "Tünel açılıyor…",
                          () =>
                            tunnel.running
                              ? tunnelService.stop()
                              : tunnelService.start(),
                        ),
                    },
                  ]
                : []),
          ],
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
          {!settingsOpen ? (
            <ServiceConsole {...serviceConsole(service as ServiceId)} />
          ) : null}
          {settingsOpen &&
          (service === WorkspaceService.PhpMyAdmin ||
            service === WorkspaceService.Mail ||
            service === WorkspaceService.Postgres ||
            service === WorkspaceService.Redis) ? (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void run("Hizmet ayarları doğrulanıyor ve kaydediliyor…", () =>
                  settingsService.save(values),
                );
              }}
            >
              <p className="section-note">
                {running
                  ? "Bu ayarları kaydetmek için önce ortamı durdurun."
                  : "Kaydedilen portlar sonraki başlangıçta uygulanır."}
              </p>
              <fieldset disabled={locked} className="settings-fields">
                {service === WorkspaceService.PhpMyAdmin ? (
                  <PmaForm values={values} change={change} />
                ) : null}
                {service === WorkspaceService.Mail ? (
                  <MailForm values={values} change={change} />
                ) : null}
                {service === WorkspaceService.Postgres ? (
                  <PostgresForm values={values} change={change} />
                ) : null}
                {service === WorkspaceService.Redis ? (
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
          ) : settingsOpen && service === WorkspaceService.Github ? (
            <GithubSettings
              github={github}
              busy={busy}
              run={run}
              pane="settings"
            />
          ) : settingsOpen && service === WorkspaceService.Tunnel ? (
            <TunnelSettings
              tunnel={tunnel}
              busy={busy}
              run={run}
              pane="settings"
            />
          ) : (
            <>
              {service === WorkspaceService.PhpMyAdmin ? (
                <PmaActions
                  phpmyadmin={phpmyadmin}
                  busy={busy}
                  running={running}
                  run={run}
                />
              ) : null}
              {service === WorkspaceService.Mail ? (
                <MailActions mail={mail} busy={busy} run={run} />
              ) : null}
              {service === WorkspaceService.Postgres ? (
                <PostgresSettings postgres={postgres} busy={busy} run={run} />
              ) : null}
              {service === WorkspaceService.Redis ? (
                <RedisActions redis={redis} busy={busy} run={run} />
              ) : null}
              {service === WorkspaceService.Github ? (
                <GithubSettings
                  github={github}
                  busy={busy}
                  run={run}
                  pane="ops"
                />
              ) : null}
              {service === WorkspaceService.Tunnel ? (
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
  phpmyadmin,
  busy,
  running,
  run,
}: {
  phpmyadmin?: PackageStatus;
  busy: boolean;
  running: boolean;
  run: Run;
}) {
  return (
    <section className="settings-section">
      <ServiceRepair
        name="phpMyAdmin"
        installed={Boolean(phpmyadmin?.installed)}
        repairable={Boolean(phpmyadmin?.repairable)}
        issue={phpmyadmin?.issue ?? null}
        busy={busy}
        run={run}
        keeps={[
          "Oturum dosyaları (data/phpmyadmin)",
          "Dil, satır ve oturum ayarları",
          "MySQL verileri ve proje .env dosyaları",
        ]}
        action={() => packagesService.repair(ComponentId.PhpMyAdmin)}
        blocked={running}
        blockedReason="Onarmak için ortamı durdurun. phpMyAdmin dosyaları web sunucusu açıkken kilitlenebilir."
      />
      <p className="section-note">
        Dil, satır sayısı ve oturum süresi Ayarlar düğmesindedir. Giriş:{" "}
        <strong>root</strong>; MySQL parolası Ayarlar → Sistem’de. Onarım ortam
        çalışırken kilitlenir; önce ortamı durdurun.
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
