import { useNavigationGuard } from "../hooks/useNavigationGuard";
import { useEffect, useRef, useState } from "react";
import { FolderOpen, Eye, EyeOff } from "lucide-react";
import { call, chooseFolder } from "../api";
import type {
  Settings as Values,
  PhpSettings,
  PackageStatus,
  PermissionState,
  Run,
  NodeState,
} from "../types";
import Requirements from "./Requirements";
import DesktopSettings from "./DesktopSettings";
import UpdateSettings from "./UpdateSettings";
import AppearanceSettings from "./AppearanceSettings";
import NodeSettings from "./NodeSettings";
import PermissionSettings from "./PermissionSettings";
import type { UpdateInfo } from "../updates";
import NumberField from "./NumberField";
import Toggle from "./Toggle";
import { useDraft } from "../hooks/useDraft";
import SaveBar from "./SaveBar";
import CloudSettings from "./CloudSettings";

const sections = [
  "Genel",
  "Görünüm",
  "Windows",
  "PHP",
  "MySQL",
  "Web sunucusu",
  "Yedek ve aktarım",
  "Sistem",
  "Güncellemeler",
  "Cloud",
] as const;
const sectionGroups = [
  { label: "Çalışma alanı", items: ["Genel", "Görünüm", "Windows"] },
  {
    label: "Sunucu",
    items: ["PHP", "MySQL", "Web sunucusu"],
  },
  {
    label: "Yönetim",
    items: ["Yedek ve aktarım", "Sistem", "Güncellemeler", "Cloud"],
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

function runtimeSlice(settings: Values) {
  return {
    ...settings,
    projectsDir: "",
    backupsDir: "",
    startOnLaunch: false,
    api: undefined,
  };
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
  node,
  permissions,
  appUpdate,
  onAppUpdate,
  openUpdates,
  onUpdatesOpened,
}: {
  settings: Values;
  versions: PackageStatus[];
  phpVersion: string;
  mysqlRunning: boolean;
  home: string;
  busy: boolean;
  running: boolean;
  run: Run;
  node: NodeState;
  permissions: PermissionState;
  appUpdate: UpdateInfo | null;
  onAppUpdate: (update: UpdateInfo | null) => void;
  openUpdates: number;
  onUpdatesOpened: () => void;
}) {
  const navigate = useNavigationGuard();
  const { values, setValues, dirty, reset } = useDraft(
    settings,
    "Sunucu ayarları",
  );
  const [section, setSection] = useState<(typeof sections)[number]>("Genel");
  const [updateRequest, setUpdateRequest] = useState(0);
  useEffect(() => {
    if (openUpdates) {
      setSection("Güncellemeler");
      setUpdateRequest((value) => value + 1);
      onUpdatesOpened();
    }
  }, [openUpdates, onUpdatesOpened]);
  const [version, setVersion] = useState("");
  const [password, setPassword] = useState("");
  const [nextPassword, setNextPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showNext, setShowNext] = useState(false);
  const [note, setNote] = useState("");
  const input = useRef<HTMLInputElement>(null);
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
          aria-label={label}
          value={values[key]}
          placeholder={fallback}
          onChange={(e) => change(key, e.target.value)}
        />
        <button
          type="button"
          className="button secondary"
          disabled={busy}
          onClick={async () => {
            // The native picker is not a Rust operation: no busy lock, no
            // "İşlem tamamlandı." when the user simply cancels it.
            try {
              const path = await chooseFolder();
              if (path) change(key, path);
            } catch (e) {
              setNote(e instanceof Error ? e.message : String(e));
            }
          }}
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
    <div className="settings-layout settings-workspace">
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
                  aria-controls="settings-content"
                  onClick={() => {
                    if (s !== section)
                      navigate(
                        () => setSection(s),
                        ["Windows tercihleri", "Cloud eşleştirmesi"],
                      );
                  }}
                >
                  {s}
                </button>
              ))}
            </div>
          </div>
        ))}
      </nav>
      <div className="settings-content" id="settings-content">
        <header className="inner-page-heading">
          <div>
            <span className="eyebrow">Yapılandırma</span>
            <h2>{section === "Genel" ? "Genel ayarlar" : section}</h2>
          </div>
          {dirty && <span className="draft-indicator">Taslak</span>}
        </header>
        {["PHP", "MySQL", "Web sunucusu"].includes(section) && (
          <p className="context-note">
            {running
              ? "Port ve servis ayarlarını değiştirmek için sunucuyu durdurun."
              : "Kaydettikten sonra servisleri ve açık terminalleri yeniden başlatın."}
          </p>
        )}
        {note && (
          <p role="status" className="settings-feedback">
            {note}
          </p>
        )}
        {section === "Windows" && <DesktopSettings busy={busy} run={run} />}
        {section === "Cloud" && <CloudSettings />}
        {section === "Görünüm" && <AppearanceSettings />}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            if (!canSave) return;
            void run("Ayarlar doğrulanıyor ve kaydediliyor…", () =>
              call("save_settings", { settings: values }),
            );
          }}
        >
          <fieldset disabled={busy} className="settings-fields">
            {section === "Genel" && (
              <section className="settings-section">
                <h2>Çalışma alanı</h2>
                {folder("projectsDir", "Proje çalışma alanı", `${home}\\www`)}
                <p className="section-note">
                  Yeni projelerin konumu. Boşsa www kullanılır. Mevcut projeler
                  taşınmaz.
                </p>
                <Toggle
                  label="ServerBond açıldığında sunucuyu otomatik başlat"
                  value={values.startOnLaunch}
                  onChange={(v) => change("startOnLaunch", v)}
                />
                <p className="section-note">
                  Uygulama açıldığında kurulu servisleri başlatır.
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
                  Boşsa backups kullanılır. Mevcut yedekler taşınmaz.
                </p>
              </section>
            )}
          </fieldset>
          <fieldset disabled={locked} className="settings-fields">
            {section === "Genel" && (
              <section className="settings-section">
                <h2>Bağlantı portları</h2>
                {running && (
                  <p className="section-note">
                    Portları değiştirmek için sunucuyu durdurun.
                  </p>
                )}
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
                  Yeni projelerin varsayılan sürümü. Mevcut projeler ayrı
                  seçilir.
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
                    ? "Bu sürümü kullanan tüm projelere uygulanır."
                    : "Özel profili olmayan PHP sürümlerine uygulanır."}
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
                  Gerekli uzantıları kapatmak projeleri ve phpMyAdmin’i
                  etkileyebilir.
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
                  anahtar=değer biçimi kullanın. Formdaki ayarları
                  tekrarlamayın.
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
                      [
                        "waitSeconds",
                        "Boşta bağlantı süresi (sn)",
                        1,
                        31536000,
                      ],
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
                  Karşılaştırma düzeni yalnızca yeni veritabanlarına uygulanır.
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
                  Virgülle ayırın. Boşsa isteğe bağlı SQL modları kapanır.
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
                    "Örnek: {name}.localhost. Kayıtlı proje adresleri de değişir; APP_URL değerini ayrıca güncelleyin."
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
                  onChange={(v) =>
                    change("web", { ...values.web, accessLog: v })
                  }
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
                  HTTP, HTTPS’e yönlendirilir. Yerel sertifika Windows
                  kullanıcınızın güven deposuna eklenir.
                </p>
                <p className="section-note">
                  Yalnızca yerel erişim. Kayıtlar: Günlükler → Caddy.
                </p>
              </section>
            )}
            {section === "Yedek ve aktarım" && (
              <section className="settings-section">
                <h2>Ayarları aktar</h2>
                <p className="section-note">
                  Yalnızca ayarlar aktarılır; parolalar, veritabanları ve
                  projeler dahil değildir. İçe aktardıktan sonra Kaydet’i seçin.
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
                      a.download = "serverbond-settings.json";
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
                HTTPS açıkken sunucuyu başlatın, ardından yerel sertifikaya
                güvenin.
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
          <SaveBar
            dirty={dirty}
            busy={busy}
            canSave={canSave}
            label="Ayarları kaydet"
            blocked={
              running && runtimeDirty
                ? "Kaydetmek için sunucuyu durdurun."
                : undefined
            }
            onReset={() => {
              reset();
              setNote("");
            }}
          />
        </form>
        {section === "Sistem" && (
          <>
            <NodeSettings node={node} busy={busy} run={run} />
            <PermissionSettings
              permissions={permissions}
              busy={busy}
              run={run}
            />
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
                Parola şifreli saklanır. Proje .env dosyaları otomatik
                güncellenmez.
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
                      aria-label={
                        showNext ? "Parolayı gizle" : "Parolayı göster"
                      }
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
              <h2>ServerBond veri klasörü</h2>
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
                Servis ayarlarını bu ekrandan değiştirin. Önceki ayarlar
                yedeklenir.
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
            openUpdates={updateRequest}
          />
        )}
      </div>
    </div>
  );
}
