import { useEffect, useRef, useState } from "react";
import { FolderOpen, Eye, EyeOff } from "lucide-react";
import { call, chooseFolder } from "../api";
import type {
  Settings as Values,
  PhpSettings,
  PackageStatus,
  Run,
} from "../types";
import Requirements from "./Requirements";
import DesktopSettings from "./DesktopSettings";
import UpdateSettings from "./UpdateSettings";
import type { UpdateInfo } from "../updates";

const sections = [
  "Genel",
  "PHP",
  "MySQL",
  "Web sunucusu",
  "phpMyAdmin",
  "Yedek ve aktarım",
  "Sistem",
  "Güncellemeler",
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

export default function Settings({
  settings,
  versions,
  home,
  busy,
  running,
  run,
  appUpdate,
  onAppUpdate,
  openUpdates,
}: {
  settings: Values;
  versions: PackageStatus[];
  home: string;
  busy: boolean;
  running: boolean;
  run: Run;
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
  const [note, setNote] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const dirty = JSON.stringify(values) !== JSON.stringify(settings);
  const locked = busy || running;
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
      </nav>
      <p className="section-note">
        {running
          ? "PHP ve sunucu ayarlarını değiştirmek için önce ortamı durdurun. Masaüstü tercihlerini aşağıdan değiştirebilirsiniz."
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
        <fieldset disabled={locked} className="settings-fields">
          {section === "Genel" && (
            <section className="settings-section">
              <h2>Çalışma alanı</h2>
              {folder(
                "projectsDir",
                "Yeni projeler için klasör",
                `${home}\\www`,
              )}
              <p className="section-note">
                Boş değer varsayılan www klasörünü kullanır. Klasör mevcut
                olmalı; kayıtlı projeler taşınmaz.
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
                    ["displayErrors", "Hataları sayfada göster"],
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
                Adres: http://phpmyadmin.f4box.localhost:{values.webPort}.
                Dosyalar bileşen kuruluysa sunulur. Yükleme limitleri varsayılan
                PHP sürümünün profilinden alınır.
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
        {section !== "Sistem" && section !== "Güncellemeler" && (
          <div className="settings-save">
            <span>
              {dirty ? "Kaydedilmemiş değişiklikler var" : "Ayarlar güncel"}
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
              disabled={locked || !dirty}
            >
              Ayarları kaydet
            </button>
          </div>
        )}
      </form>
      {section === "Sistem" && (
        <>
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
            </div>
            <p className="section-note">
              Rastgele parola Windows hesabınıza bağlı olarak şifrelenir.
            </p>
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
              <FolderOpen size={17} />
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
