import { useEffect, useRef, useState } from "react";
import { FolderOpen, Eye, EyeOff } from "lucide-react";
import { call, chooseFolder } from "../api";
import type {
  Settings as Values,
  PhpSettings,
  PackageStatus,
  PermissionState,
  Run,
  TunnelState,
  MailState,
  NodeState,
  PostgresState,
  GithubState,
} from "../types";
import Requirements from "./Requirements";
import DesktopSettings from "./DesktopSettings";
import UpdateSettings from "./UpdateSettings";
import TunnelSettings from "./TunnelSettings";
import MailActions from "./MailActions";
import PostgresSettings from "./PostgresSettings";
import GithubSettings from "./GithubSettings";
import NodeSettings from "./NodeSettings";
import PermissionSettings from "./PermissionSettings";
import type { UpdateInfo } from "../updates";

const sections = [
  "Genel",
  "PHP",
  "MySQL",
  "Web sunucusu",
  "phpMyAdmin",
  "E-posta",
  "PostgreSQL",
  "GitHub",
  "Tünel",
  "Yedek ve aktarım",
  "Sistem",
  "Güncellemeler",
] as const;
const sectionGroups = [
  {
    label: "Ortam",
    items: ["Genel", "PHP", "MySQL", "Web sunucusu"],
  },
  {
    label: "Hizmetler",
    items: ["phpMyAdmin", "E-posta", "PostgreSQL", "GitHub", "Tünel"],
  },
  {
    label: "Yönetim",
    items: ["Yedek ve aktarım", "Sistem", "Güncellemeler"],
  },
] as const;
const extensions = [
  "curl",
  "fileinfo",
  "mbstring",
  "openssl",
  "pdo_mysql",
  "mysqli",
  "sodium",
  "pdo_sqlite",
  "sqlite3",
  "intl",
  "zip",
  "bcmath",
  "gd",
  "exif",
  "soap",
  "sockets",
  "ftp",
  "gettext",
  "imap",
  "ldap",
  "odbc",
  "pdo_odbc",
  "pgsql",
  "pdo_pgsql",
  "shmop",
  "tidy",
  "xsl",
];
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

function runtimeSlice(settings: Values) {
  return { ...settings, projectsDir: "", backupsDir: "", startOnLaunch: false };
}

export default function Settings({
  settings,
  versions,
  phpVersion,
  mysqlRunning,
  home,
  busy,
  running,
  run,
  tunnel,
  mail,
  postgres,
  github,
  node,
  permissions,
  appUpdate,
  onAppUpdate,
  openUpdates,
}: {
  settings: Values;
  versions: PackageStatus[];
  phpVersion: string;
  mysqlRunning: boolean;
  home: string;
  busy: boolean;
  running: boolean;
  run: Run;
  tunnel: TunnelState;
  mail: MailState;
  postgres: PostgresState;
  github: GithubState;
  node: NodeState;
  permissions: PermissionState;
  appUpdate: UpdateInfo | null;
  onAppUpdate: (update: UpdateInfo | null) => void;
  openUpdates: number;
}) {
  const [values, setValues] = useState(() => structuredClone(settings));
  const [section, setSection] = useState<(typeof sections)[number]>("Genel");
  useEffect(() => {
    if (openUpdates) setSection("Güncellemeler");
  }, [openUpdates]);
  const [version, setVersion] = useState("");
  const [password, setPassword] = useState("");
  const [nextPassword, setNextPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showNext, setShowNext] = useState(false);
  const [note, setNote] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const dirty = JSON.stringify(values) !== JSON.stringify(settings);
  const runtimeDirty =
    JSON.stringify(runtimeSlice(values)) !==
    JSON.stringify(runtimeSlice(settings));
  const locked = busy || running;
  const canSave = dirty && !busy && (!running || !runtimeDirty);
  const php = (version && values.phpVersions[version]) || values.php;
  const setPhp = (patch: Partial<PhpSettings>) =>
    setValues((v) =>
      version
        ? {
            ...v,
            phpVersions: { ...v.phpVersions, [version]: { ...php, ...patch } },
          }
        : { ...v, php: { ...v.php, ...patch } },
    );
  const change = <K extends keyof Values>(key: K, value: Values[K]) =>
    setValues((v) => ({ ...v, [key]: value }));
  const folder = (
    key: "projectsDir" | "backupsDir",
    label: string,
    fallback: string,
  ) => (
    <label>
      {label}
      <div className="input-with-button">
        <input
          value={values[key]}
          placeholder={fallback}
          onChange={(e) => change(key, e.target.value)}
        />
        <button
          type="button"
          className="button secondary"
          onClick={() =>
            void run("Klasör seçiliyor…", async () => {
              const path = await chooseFolder();
              if (path) change(key, path);
            })
          }
        >
          <FolderOpen size={16} />
          Seç
        </button>
      </div>
    </label>
  );
  const draft = (command: string, message: string) =>
    void run("Ayarlar okunuyor…", async () => {
      setValues(await call<Values>(command));
      setNote(message);
    });
  return (
    <div className="settings-layout">
      <nav className="settings-tabs" aria-label="Ayar bölümleri">
        {sectionGroups.map((group) => (
          <div
            key={group.label}
            className="settings-tab-group"
            role="group"
            aria-label={group.label}
          >
            <span className="settings-tab-caption">{group.label}</span>
            <div className="settings-tab-row">
              {group.items.map((s) => (
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
          </div>
        ))}
      </nav>
      <p className="section-note">
        {running
          ? "Çalışma alanı, yedek klasörü ve açılış tercihi ortam çalışırken kaydedilir. Port, PHP, MySQL ve web ayarları için önce ortamı durdurun."
          : "Kaydedilen ayarlar sonraki servis başlangıcında uygulanır. Açık terminalleri yeniden açın."}
      </p>
      {note && (
        <p role="status" className="settings-feedback">
          {note}
        </p>
      )}
      {section === "Genel" && <DesktopSettings busy={busy} run={run} />}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void run("Ayarlar doğrulanıyor ve kaydediliyor…", () =>
            call("save_settings", { settings: values }),
          );
        }}
      >
        <fieldset disabled={busy} className="settings-fields">
          {section === "Genel" && (
            <section className="settings-section">
              <h2>Çalışma alanı</h2>
              {folder(
                "projectsDir",
                "Proje çalışma alanı",
                `${home}\\projects`,
              )}
              <p className="section-note">
                Yeni Laravel projeleri buraya yazılır. Klasör tara bir ve iki
                seviye kökleri okur: magaza veya musteri/magaza. vendor ve
                node_modules atlanır. Boş değer projects klasörünü kullanır;
                eski www klasörü de taranır. Kayıtlı projeler taşınmaz.
              </p>
              <Toggle
                label="F4Box açıldığında ortamı otomatik başlat"
                value={values.startOnLaunch}
                onChange={(v) => change("startOnLaunch", v)}
              />
              <p className="section-note">
                Kurulu PHP, MySQL ve web sunucusunu başlatır. Windows başlangıcı
                için yukarıdaki masaüstü tercihini de açın.
              </p>
            </section>
          )}
          {section === "Yedek ve aktarım" && (
            <section className="settings-section">
              <h2>Veritabanı yedekleri</h2>
              {folder(
                "backupsDir",
                "SQL yedeklerinin klasörü",
                `${home}\\backups`,
              )}
              <p className="section-note">
                Boş değer varsayılan backups klasörünü kullanır. Bundan sonraki
                yedekler bu klasöre yazılır; eski yedekler taşınmaz.
              </p>
            </section>
          )}
        </fieldset>
        <fieldset disabled={locked} className="settings-fields">
          {section === "Genel" && (
            <section className="settings-section">
              <h2>Bağlantı portları</h2>
              <div className="settings-grid">
                {(
                  [
                    ["webPort", "Web sunucusu portu"],
                    ["mysqlPort", "MySQL portu"],
                    ["phpPort", "PHP FastCGI portu"],
                  ] as const
                ).map(([key, label]) => (
                  <NumberField
                    key={key}
                    label={label}
                    value={values[key]}
                    min={1}
                    onChange={(v) => change(key, v)}
                  />
                ))}
              </div>
            </section>
          )}
          {section === "PHP" && (
            <section className="settings-section">
              <h2>PHP çalışma ayarları</h2>
              <label>
                Varsayılan PHP sürümü
                <select
                  value={phpVersion}
                  disabled={busy || running}
                  onChange={(e) => {
                    const version = e.target.value;
                    if (version && version !== phpVersion)
                      void run("Varsayılan PHP sürümü uygulanıyor…", () =>
                        call("select_php", { version }),
                      );
                  }}
                >
                  {versions.map((p) => (
                    <option key={p.version} value={p.version}>
                      PHP {p.version}
                      {p.installed ? " · Kurulu" : " · Kurulu değil"}
                    </option>
                  ))}
                </select>
              </label>
              <p className="section-note">
                Yeni projeler ve ortak FastCGI bu sürümü kullanır. Kurulu
                değilse indirilir. Ortam çalışırken değiştirilemez; proje
                kartından ayrı sürüm seçilebilir.
              </p>
              <label>
                Ayar kapsamı
                <select
                  value={version}
                  onChange={(e) => setVersion(e.target.value)}
                >
                  <option value="">Ortak varsayılanlar</option>
                  {versions.map((p) => (
                    <option key={p.version} value={p.version}>
                      PHP {p.version}
                      {p.installed ? " · Kurulu" : " · Kurulu değil"}
                      {values.phpVersions[p.version] ? " · Özel ayar" : ""}
                    </option>
                  ))}
                </select>
              </label>
              <p className="section-note">
                {version
                  ? "Değişiklik yaptığınızda bu sürüm için ayrı profil oluşur. Aynı sürümü kullanan tüm projelere uygulanır."
                  : "Özel profili olmayan tüm PHP sürümleri bu ayarları kullanır."}{" "}
                Kurulu sürümler kaydetmeden önce çalıştırılarak doğrulanır.
                Üretim varsayılanı: sayfada hata kapalı, günlük ve OPcache açık.
              </p>
              {version && values.phpVersions[version] && (
                <button
                  type="button"
                  className="button secondary small"
                  onClick={() => {
                    const profiles = { ...values.phpVersions };
                    delete profiles[version];
                    change("phpVersions", profiles);
                  }}
                >
                  Bu sürümde ortak ayarlara dön
                </button>
              )}
              <div className="settings-grid">
                <label>
                  Saat dilimi
                  <input
                    required
                    value={php.timezone}
                    onChange={(e) => setPhp({ timezone: e.target.value })}
                  />
                </label>
                {(
                  [
                    ["memoryMb", "Bellek (MB, -1: sınırsız)", -1, 32768],
                    ["uploadMb", "Dosya yükleme limiti (MB)", 1, 8192],
                    ["postMb", "POST limiti (MB)", php.uploadMb, 8192],
                    [
                      "executionSeconds",
                      "Çalışma süresi (sn, 0: sınırsız)",
                      0,
                      86400,
                    ],
                    [
                      "inputSeconds",
                      "Giriş süresi (sn, -1: çalışma süresi)",
                      -1,
                      86400,
                    ],
                    ["inputVars", "Giriş değişkeni limiti", 100, 100000],
                    ["opcacheMb", "OPcache belleği (MB)", 8, 4096],
                  ] as const
                ).map(([key, label, min, max]) => (
                  <NumberField
                    key={key}
                    label={label}
                    value={php[key]}
                    min={min}
                    max={max}
                    onChange={(v) => setPhp({ [key]: v })}
                  />
                ))}
              </div>
              <div className="settings-grid">
                {(
                  [
                    [
                      "displayErrors",
                      "Hataları sayfada göster (üretimde kapalı)",
                    ],
                    ["logErrors", "Hataları günlüğe yaz"],
                    ["opcache", "OPcache etkin"],
                  ] as const
                ).map(([key, label]) => (
                  <Toggle
                    key={key}
                    label={label}
                    value={php[key]}
                    onChange={(v) => setPhp({ [key]: v })}
                  />
                ))}
              </div>
              <h3>Uzantılar</h3>
              <p className="section-note">
                Seçilen uzantı PHP paketinde bulunmalı. Gerekli uzantıları
                kapatmak phpMyAdmin veya projelerinizi etkileyebilir.
              </p>
              <div className="extension-grid">
                {extensions.map((ext) => (
                  <Toggle
                    key={ext}
                    label={ext}
                    value={php.extensions.includes(ext)}
                    onChange={(enabled) =>
                      setPhp({
                        extensions: enabled
                          ? [...php.extensions, ext]
                          : php.extensions.filter((s) => s !== ext),
                      })
                    }
                  />
                ))}
              </div>
              <label>
                Ek php.ini ayarları
                <textarea
                  rows={5}
                  spellCheck={false}
                  placeholder={
                    "; Her satıra bir ayar\nserialize_precision=-1\nshort_open_tag=Off"
                  }
                  value={php.extraIni}
                  onChange={(e) => setPhp({ extraIni: e.target.value })}
                />
              </label>
              <p className="section-note">
                anahtar=değer biçimi kullanın. Form değerlerini burada
                tekrarlamayın. Uzantı yolu ve FastCGI bağlantı ayarları F4Box
                tarafından yönetilir.
              </p>
            </section>
          )}
          {section === "MySQL" && (
            <section className="settings-section">
              <h2>MySQL 8.4</h2>
              <div className="settings-grid">
                {(
                  [
                    ["bufferPoolMb", "InnoDB belleği (MB)", 8, 32768],
                    ["maxConnections", "En fazla bağlantı", 10, 5000],
                    ["maxPacketMb", "En büyük paket (MB)", 1, 1024],
                    ["waitSeconds", "Boşta bağlantı süresi (sn)", 1, 31536000],
                    ["longQuerySeconds", "Yavaş sorgu eşiği (sn)", 0, 3600],
                  ] as const
                ).map(([key, label, min, max]) => (
                  <NumberField
                    key={key}
                    label={label}
                    value={values.mysql[key]}
                    min={min}
                    max={max}
                    onChange={(v) =>
                      change("mysql", { ...values.mysql, [key]: v })
                    }
                  />
                ))}
                <label>
                  Varsayılan karşılaştırma düzeni
                  <select
                    value={values.mysql.collation}
                    onChange={(e) =>
                      change("mysql", {
                        ...values.mysql,
                        collation: e.target.value,
                      })
                    }
                  >
                    {[
                      "utf8mb4_unicode_ci",
                      "utf8mb4_0900_ai_ci",
                      "utf8mb4_general_ci",
                      "utf8mb4_bin",
                      "utf8mb4_turkish_ci",
                    ].map((s) => (
                      <option key={s}>{s}</option>
                    ))}
                  </select>
                </label>
              </div>
              <Toggle
                label="Yavaş sorgu günlüğü"
                value={values.mysql.slowQueryLog}
                onChange={(v) =>
                  change("mysql", { ...values.mysql, slowQueryLog: v })
                }
              />
              <p className="section-note">
                Günlük MySQL veri klasörüne yazılır. Karşılaştırma düzeni yeni
                veritabanlarını etkiler; mevcut tablolar dönüştürülmez.
              </p>
              <label>
                SQL modları
                <textarea
                  rows={3}
                  spellCheck={false}
                  value={values.mysql.sqlMode}
                  onChange={(e) =>
                    change("mysql", {
                      ...values.mysql,
                      sqlMode: e.target.value,
                    })
                  }
                />
              </label>
              <p className="section-note">
                Modları virgülle ayırın. Boş değer isteğe bağlı SQL modlarını
                kapatır.
              </p>
            </section>
          )}
          {section === "Web sunucusu" && (
            <section className="settings-section">
              <h2>Adresler ve Caddy</h2>
              <label>
                Proje adres kalıbı
                <input
                  required
                  value={values.web.hostPattern}
                  onChange={(e) =>
                    change("web", {
                      ...values.web,
                      hostPattern: e.target.value,
                    })
                  }
                />
              </label>
              <p className="section-note">
                {
                  "Örnek: {name}.dev.localhost. .localhost adresleri hosts değişikliği gerektirmez. Kaydetmek kayıtlı proje adreslerini de günceller; projelerinizdeki APP_URL değerini ayrıca düzenleyin."
                }
              </p>
              <div className="settings-grid">
                <NumberField
                  label="FastCGI bağlantı süresi (sn)"
                  value={values.web.connectSeconds}
                  max={120}
                  onChange={(v) =>
                    change("web", { ...values.web, connectSeconds: v })
                  }
                />
                <NumberField
                  label="PHP yanıt süresi (sn)"
                  value={values.web.readSeconds}
                  max={86400}
                  onChange={(v) =>
                    change("web", { ...values.web, readSeconds: v })
                  }
                />
              </div>
              <Toggle
                label="Gzip / Zstandard sıkıştırma"
                value={values.web.compression}
                onChange={(v) =>
                  change("web", { ...values.web, compression: v })
                }
              />
              <Toggle
                label="HTTP erişim günlüğü"
                value={values.web.accessLog}
                onChange={(v) => change("web", { ...values.web, accessLog: v })}
              />
              <Toggle
                label="Yerel HTTPS (Auto SSL)"
                value={values.web.https}
                onChange={(v) => change("web", { ...values.web, https: v })}
              />
              {values.web.https ? (
                <NumberField
                  label="HTTPS portu"
                  value={values.web.httpsPort}
                  onChange={(v) =>
                    change("web", { ...values.web, httpsPort: v })
                  }
                />
              ) : null}
              <p className="section-note">
                Caddy dahili bir CA ile {`{name}.localhost`} adreslerine
                sertifika verir. HTTP istekleri HTTPS’e yönlendirilir. Sertifika
                bu Windows kullanıcısının güven deposuna yazılır; yönetici onayı
                gerekmez. Ortam çalışırken port değiştirilemez. Kaydettikten
                sonra ortamı başlatın; sertifika güveni aşağıdaki düğmelerle
                yönetilir.
              </p>
              <p className="section-note">
                Erişim kayıtları Günlükler → Caddy bölümünde görünür. Sunucu
                yalnızca bu bilgisayardan bağlantı kabul eder.
              </p>
            </section>
          )}
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
                aynı anahtarlar PHP sekmesindeki ek ayarlar alanına yazılamaz.
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
          {section === "Yedek ve aktarım" && (
            <section className="settings-section">
              <h2>Ayarları aktar</h2>
              <p className="section-note">
                JSON yalnızca tercihleri içerir; MySQL yönetici parolası,
                veritabanları ve proje listesi içermez. İçe aktarılan ayarları
                gözden geçirip Kaydet ile uygulayın.
              </p>
              <div className="settings-actions">
                <button
                  type="button"
                  className="button secondary"
                  onClick={() => {
                    const url = URL.createObjectURL(
                      new Blob([JSON.stringify(values, null, 2)], {
                        type: "application/json",
                      }),
                    );
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = "f4box-settings.json";
                    a.click();
                    setTimeout(() => URL.revokeObjectURL(url), 30000);
                  }}
                >
                  JSON dışa aktar
                </button>
                <button
                  type="button"
                  className="button secondary"
                  onClick={() => input.current?.click()}
                >
                  JSON içe aktar
                </button>
                <input
                  ref={input}
                  type="file"
                  accept=".json,application/json"
                  hidden
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    e.target.value = "";
                    if (file)
                      void run("Ayarlar okunuyor…", async () => {
                        if (file.size > 262144)
                          throw new Error(
                            "Ayar dosyası 256 KB sınırını aşıyor.",
                          );
                        setValues(
                          await call<Values>("settings_validate_import", {
                            json: await file.text(),
                          }),
                        );
                        setNote(
                          "Ayarlar taslağa yüklendi. Gözden geçirip kaydedin.",
                        );
                      });
                  }}
                />
                <button
                  type="button"
                  className="button secondary"
                  onClick={() =>
                    draft(
                      "settings_previous",
                      "Önceki ayarlar taslağa alındı. Uygulamak için kaydedin.",
                    )
                  }
                >
                  Önceki ayarları getir
                </button>
                <button
                  type="button"
                  className="button secondary"
                  onClick={() =>
                    draft(
                      "settings_defaults",
                      "Varsayılanlar taslağa alındı. Portlar korundu. Uygulamak için kaydedin.",
                    )
                  }
                >
                  Varsayılanlara dön
                </button>
              </div>
            </section>
          )}
        </fieldset>
        {section === "Web sunucusu" && (
          <section className="settings-section">
            <h2>Sertifika güveni</h2>
            <p className="section-note">
              HTTPS açıkken ortamı bir kez başlatın. Sertifika bu Windows
              kullanıcısının deposuna yazılır; tarayıcı uyarısı kaybolur.
            </p>
            <div className="settings-grid">
              <button
                type="button"
                className="button secondary"
                disabled={busy || !settings.web.https}
                onClick={() =>
                  void run("Sertifika güven deposuna ekleniyor…", () =>
                    call("https_trust", { action: "trust" }),
                  )
                }
              >
                Sertifikayı güven deposuna ekle
              </button>
              <button
                type="button"
                className="button secondary"
                disabled={busy}
                onClick={() =>
                  void run("Sertifika güven deposundan kaldırılıyor…", () =>
                    call("https_trust", { action: "untrust" }),
                  )
                }
              >
                Güveni kaldır
              </button>
            </div>
          </section>
        )}
        {!["Sistem", "Güncellemeler", "Tünel", "GitHub"].includes(section) && (
          <div className="settings-save">
            <span>
              {dirty
                ? running && runtimeDirty
                  ? "Sunucu ayarlarını kaydetmek için ortamı durdurun"
                  : "Kaydedilmemiş değişiklikler var"
                : "Ayarlar güncel"}
            </span>
            <button
              type="button"
              className="button secondary"
              disabled={!dirty || busy}
              onClick={() => {
                setValues(structuredClone(settings));
                setNote("");
              }}
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
      {section === "Sistem" && (
        <>
          <NodeSettings node={node} busy={busy} run={run} />
          <PermissionSettings permissions={permissions} busy={busy} run={run} />
          <Requirements busy={busy} run={run} />
          <section className="settings-section">
            <h2>MySQL bağlantısı</h2>
            <p>127.0.0.1:{settings.mysqlPort} · root</p>
            <div className="settings-actions">
              <code>{password || "••••••••••••"}</code>
              <button
                type="button"
                className="icon-button"
                disabled={busy}
                aria-label={password ? "Parolayı gizle" : "Parolayı göster"}
                onClick={() => {
                  if (password) setPassword("");
                  else
                    void run("Parola açılıyor…", async () =>
                      setPassword(await call<string>("credentials")),
                    );
                }}
              >
                {password ? <EyeOff size={18} /> : <Eye size={18} />}
              </button>
              <button
                type="button"
                className="button secondary small"
                disabled={busy || !password}
                onClick={() =>
                  void run("Parola kopyalanıyor…", async () => {
                    await navigator.clipboard.writeText(password);
                    return "Parola panoya kopyalandı.";
                  })
                }
              >
                Kopyala
              </button>
            </div>
            <p className="section-note">
              Rastgele parola Windows hesabınıza bağlı olarak şifrelenir. Proje
              .env dosyaları yazılmaz.
            </p>
            <h3>Parolayı değiştir</h3>
            <p className="section-note">
              MySQL çalışırken kök parolasını buradan değiştirin. 8–128
              karakter; boşluk ve tırnak kullanmayın.
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
                  void run("MySQL parolası güncelleniyor…", async () => {
                    if (!mysqlRunning)
                      throw new Error(
                        "Parolayı değiştirmek için önce MySQL'i başlatın.",
                      );
                    if (nextPassword !== confirmPassword)
                      throw new Error("Parola doğrulaması eşleşmiyor.");
                    await call("change_mysql_password", {
                      password: nextPassword,
                    });
                    setPassword(nextPassword);
                    setNextPassword("");
                    setConfirmPassword("");
                    setShowNext(false);
                    return "MySQL parolası güncellendi. Proje .env dosyaları yazılmadı.";
                  })
                }
              >
                Parolayı kaydet
              </button>
            </div>
          </section>
          <section className="settings-section">
            <h2>F4Box veri klasörü</h2>
            <p className="data-path">{home}</p>
            <button
              type="button"
              className="button secondary"
              onClick={() =>
                void run("Klasör açılıyor…", () => call("open_home"))
              }
            >
              <FolderOpen size={16} />
              Klasörü aç
            </button>
            <p className="section-note">
              Kalıcı ayarlar config.json içinde; önceki tercihler
              config/settings.previous.json dosyasında tutulur. Üretilen
              php.ini, my.ini ve Caddyfile dosyalarını doğrudan düzenlemek
              yerine bu ekranı kullanın.
            </p>
          </section>
        </>
      )}
      {section === "Güncellemeler" && (
        <UpdateSettings
          busy={busy}
          running={running}
          run={run}
          available={appUpdate}
          onAvailable={onAppUpdate}
          openUpdates={openUpdates}
        />
      )}
    </div>
  );
}
