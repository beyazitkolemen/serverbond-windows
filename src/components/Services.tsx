import { useId, useState } from "react";
import NumberField from "./NumberField";
import Toggle from "./Toggle";
import { useDraft } from "../hooks/useDraft";
import {
  ChevronLeft,
  ChevronRight,
  Database,
  Github,
  Globe,
  Mail,
  Zap,
} from "lucide-react";
import {
  ComponentId,
  ToolAction,
  ToolCommand,
  WorkspaceService,
} from "../domain";
import { PMA_HOST } from "../product";
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
import StatusBadge from "./StatusBadge";
import SearchField from "./SearchField";
import { searchText } from "../search";
import SectionTabs from "./SectionTabs";
import SaveBar from "./SaveBar";

const catalog = [
  {
    id: WorkspaceService.PhpMyAdmin,
    title: "phpMyAdmin",
    icon: Database,
    copy: "MySQL yönetimi",
  },
  {
    id: WorkspaceService.Mail,
    title: "E-posta",
    icon: Mail,
    copy: "Yerel e-posta kutusu",
  },
  {
    id: WorkspaceService.Postgres,
    title: "PostgreSQL",
    icon: Database,
    copy: "PostgreSQL 17 veritabanı",
  },
  {
    id: WorkspaceService.Redis,
    title: "Redis",
    icon: Zap,
    copy: "Kuyruk ve önbellek",
  },
  {
    id: WorkspaceService.Github,
    title: "GitHub",
    icon: Github,
    copy: "Özel depolara erişim",
  },
  {
    id: WorkspaceService.Tunnel,
    title: "Tünel",
    icon: Globe,
    copy: "İnternetten erişim",
  },
] as const;

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
  const { values, setValues, dirty, reset } = useDraft(settings);
  const [service, setService] = useState<ServiceId | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [activeOnly, setActiveOnly] = useState(false);
  const tabsId = useId();
  const locked = busy || running;
  const canSave = dirty && !busy && !running;
  const change = <K extends keyof Values>(key: K, value: Values[K]) =>
    setValues((v) => ({ ...v, [key]: value }));
  const selected = catalog.find((item) => item.id === service);
  const openService = (id: ServiceId) => {
    setService(id);
    setSettingsOpen(false);
  };
  const closeSettings = () => {
    setSettingsOpen(false);
  };
  const serviceState = (id: ServiceId) => {
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
          ? `https://${PMA_HOST}:${settings.web.httpsPort}`
          : `http://${PMA_HOST}:${settings.webPort}`;
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
                  : "Önce sunucuyu başlatın",
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
  const visibleServices = catalog.filter(
    (item) =>
      searchText(`${item.title} ${item.copy}`).includes(searchText(query)) &&
      (!activeOnly || serviceState(item.id).on),
  );
  return (
    <div className="settings-layout services-workspace">
      {!selected ? (
        <>
          <div className="list-toolbar">
            <div
              className="list-filters"
              role="group"
              aria-label="Hizmet filtresi"
            >
              <button
                type="button"
                aria-pressed={!activeOnly}
                onClick={() => setActiveOnly(false)}
              >
                Tümü <span>{catalog.length}</span>
              </button>
              <button
                type="button"
                aria-pressed={activeOnly}
                onClick={() => setActiveOnly(true)}
              >
                Etkin{" "}
                <span>
                  {catalog.filter((item) => serviceState(item.id).on).length}
                </span>
              </button>
            </div>
            <SearchField label="Hizmet ara" value={query} onChange={setQuery} />
          </div>
          <div className="service-list-heading" aria-hidden="true">
            <span>Uygulama</span>
            <span>Açıklama</span>
            <span>Durum</span>
          </div>
          <ul className="service-list" aria-label="Hizmetler">
            {visibleServices.map((item) => {
              const state = serviceState(item.id);
              const Icon = item.icon;
              return (
                <li key={item.id}>
                  <button
                    type="button"
                    className="service-row"
                    data-service={item.title}
                    aria-label={`${item.title} hizmetini aç`}
                    aria-describedby={`service-${item.id}-copy service-${item.id}-state`}
                    onClick={() => openService(item.id)}
                  >
                    <span className="service-row-name">
                      <span className="service-row-icon" aria-hidden="true">
                        <Icon size={20} strokeWidth={1.6} />
                      </span>
                      <strong>{item.title}</strong>
                    </span>
                    <span
                      className="service-row-copy"
                      id={`service-${item.id}-copy`}
                    >
                      {item.copy}
                    </span>
                    <span
                      className="service-row-state"
                      id={`service-${item.id}-state`}
                    >
                      <StatusBadge tone={state.on ? "running" : "stopped"}>
                        {state.label}
                      </StatusBadge>
                    </span>
                    <ChevronRight
                      className="service-row-arrow"
                      size={16}
                      aria-hidden="true"
                    />
                  </button>
                </li>
              );
            })}
          </ul>
          {!visibleServices.length ? (
            <div className="search-empty" role="status">
              <strong>Eşleşen hizmet yok</strong>
              <button
                className="section-link"
                type="button"
                onClick={() => {
                  setQuery("");
                  setActiveOnly(false);
                }}
              >
                Filtreleri temizle
              </button>
            </div>
          ) : null}
        </>
      ) : (
        <>
          <div className="service-toolbar">
            <button
              type="button"
              className="button secondary small"
              onClick={() => {
                setService(null);
                setSettingsOpen(false);
              }}
            >
              <ChevronLeft size={16} />
              Hizmetler
            </button>
            <ChevronRight
              size={14}
              aria-hidden
              className="breadcrumb-separator"
            />
            <h2 className="service-toolbar-title">{selected.title}</h2>
          </div>
          <SectionTabs
            id={tabsId}
            label={`${selected.title} bölümleri`}
            value={settingsOpen ? "settings" : "overview"}
            onChange={(value) =>
              value === "overview" ? closeSettings() : setSettingsOpen(true)
            }
            items={[
              { id: "overview", label: "Durum ve işlemler" },
              {
                id: "settings",
                label: "Ayarlar",
                indicator: dirty ? (
                  <span
                    className="draft-dot"
                    aria-label="Kaydedilmemiş değişiklikler"
                  />
                ) : null,
              },
            ]}
          />
          <div
            id={`${tabsId}-panel`}
            role="tabpanel"
            aria-labelledby={`${tabsId}-${settingsOpen ? "settings" : "overview"}`}
            tabIndex={0}
          >
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
                  void run(
                    "Hizmet ayarları doğrulanıyor ve kaydediliyor…",
                    () => settingsService.save(values),
                  );
                }}
              >
                <p className="section-note">
                  {running
                    ? "Kaydetmek için sunucuyu durdurun."
                    : "Değişiklikler sonraki başlangıçta uygulanır."}
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
                <SaveBar
                  dirty={dirty}
                  busy={busy}
                  canSave={canSave}
                  onReset={reset}
                  label="Ayarları kaydet"
                  blocked={
                    running ? "Kaydetmek için sunucuyu durdurun." : undefined
                  }
                />
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
          </div>
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
        blockedReason="Onarmak için sunucuyu durdurun. phpMyAdmin dosyaları web sunucusu açıkken kilitlenebilir."
      />
      <p className="section-note">
        Kullanıcı: <strong>root</strong> · Parola: Ayarlar → Sistem.
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
        label="Sunucu başlatıldığında Mailpit'i de başlat"
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
        label="Sunucu başlatıldığında PostgreSQL'i de başlat"
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
        label="Sunucu başlatıldığında Redis'i de başlat"
        value={values.redis.autoStart}
        onChange={(v) => change("redis", { ...values.redis, autoStart: v })}
      />
    </section>
  );
}
