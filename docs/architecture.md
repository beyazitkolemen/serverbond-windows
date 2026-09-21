# Mimari

ServerBond üç katmandan oluşur; iş kuralı tek bir yerde, Rust çekirdeğinde yaşar. Masaüstü penceresi, komut satırı ve yerel HTTP API aynı `Manager` nesnesini kullanan ince kabuklardır.

```text
┌──────────────────────────────┐  ┌──────────────────────┐  ┌────────────────────────┐
│ src/  React + TypeScript     │  │ serverbond CLI        │  │ HTTP API 127.0.0.1     │
│ snapshot okur, services/     │  │ bin/serverbond.rs     │  │ api.rs (tiny_http)     │
│ üzerinden komut gönderir     │  │                      │  │ Bearer jeton           │
└──────────────┬───────────────┘  └──────────┬───────────┘  └───────────┬────────────┘
               │ Tauri IPC (invoke)           │ doğrudan                 │ doğrudan
┌──────────────▼───────────────┐             │                          │
│ src-tauri/  komut adaptörleri│             │                          │
│ action string → domain enum  │             │                          │
└──────────────┬───────────────┘             │                          │
               ▼                             ▼                          ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│ crates/serverbond-core  Manager (Arc)                                                    │
│  config: Mutex<Config>  processes: Mutex<HashMap<String, ManagedChild>>              │
│  operation gate  ·  contain()  ·  snapshot()  ·  log()                              │
│  services · projects · project_runtime · jobs · release · github · envfile          │
│  mail · postgres · redis · tunnel · node · https · phpmyadmin · permissions          │
│  install · secrets · storage · resilience · preferences · model · domain · repository│
└─────────────────────────────────────────────────────────────────────────────────────┘
               │ std::process::Command (ManagedChild, job object)
               ▼
   php-cgi.exe · mysqld.exe · caddy.exe · mailpit.exe · postgres.exe · redis-server.exe
   cloudflared.exe · php.exe artisan queue:work / schedule:work · git · composer
```

## Çekirdek: `crates/serverbond-core`

### `Manager`

`Manager` bir `Arc` içinde yaşar ve şu alanları korur:

| Alan | Görev |
| --- | --- |
| `home` | Veri klasörü (`C:\ServerBond`). |
| `config: Mutex<Config>` | Ayarlar ve projeler. Diskteki `config.json` ile eşlenir; her kayıt atomiktir ve önceki geçerli kopya `config.last-good.json` olarak saklanır. |
| `processes: Mutex<HashMap<String, ManagedChild>>` | Çalışan çocuk süreçler. Anahtar süreç kimliğidir: `php`, `mysql`, `caddy`, `mailpit`, `postgres`, `redis`, `cloudflared`, `php-project-<id>`, `queue-<proje>-<işçi>-<n>`, `schedule-<proje>`. |
| `operation: Mutex<()>` | İşlem kapısı. Aynı anda tek mutasyon; ikinci istek "Başka bir işlem devam ediyor" ile reddedilir, beklemez. |
| `faulted`, `shutting_down`, `startup_issue` | Kurtarma durumları (aşağıda). |
| `api: ApiState` | Yerel HTTP dinleyicisi. |
| `_lock: File` | `manager.lock`; ikinci ServerBond süreci aynı klasörü açamaz. |

Her özellik modülü `impl Manager { … }` bloğuyla yöntem ekler. Modüller birbirini doğrudan çağırmaz; ortak alt yapı `storage`, `process`, `secrets`, `install` içindedir.

### İşlem modeli

1. Kabuk (Tauri komutu, CLI, API) `Manager::contain(|| …)` çağırır. `contain` paniği yakalar, `faulted` bayrağını kaldırır ve "yeniden başlatma gerekli" hatası döner; süreç çökmez.
2. Mutasyon yapan yöntemler ilk satırda `self.gate()?` ile işlem kilidini alır. Okumalar (`snapshot`, `read_log`) kilit almaz.
3. Süreç başlatan yöntemler `ManagedChild::spawn` kullanır: çıktı `logs/<id>.log` dosyasına yönlendirilir ve boyutla sınırlanır; Windows'ta iş nesnesi (job object) ServerBond kapandığında çocukların da kapanmasını garanti eder.
4. `snapshot()` her çağrıda `reap_exited_children` ile ölmüş çocukları listeden düşürür. Kuyruk işçileri sıfır çıkış koduyla ve yeterli çalışma süresinden sonra kapanmışsa bu planlı bir çıkıştır (`--max-jobs`, `queue:restart`) ve `respawn_planned_workers` onları yeniden başlatır; diğer çıkışlar `service_errors` içine hata olarak yazılır.

### Kurtarma

| Durum | Ne olur |
| --- | --- |
| `config.json` bozuk | Açılışta `config.last-good.json` denenir. O da yoksa `startup_issue` dolar; arayüz "Yapılandırmayı kurtar" teklif eder ve mutasyonlar kilitlenir. |
| Panik | `faulted` kalıcıdır; sadece durdurma ve yeniden başlatma serbesttir. |
| Kilitli/yer kalmayan disk | `storage::atomic_write` ve `require_space` erken hata verir; yarım dosya yazılmaz. |
| Beklenmeyen süreç ölümü | `service_errors` içine yazılır, kart üzerinde görünür; kullanıcı günlüğe bakıp yeniden başlatır. |

### Sırlar

MySQL/PostgreSQL parolaları, GitHub ve Cloudflare jetonları `secrets.rs` üzerinden DPAPI ile mevcut Windows kullanıcısına bağlanarak şifrelenir. Parola değişiminde yeni sır önce geçici dosyaya yazılıp geri okunur, veritabanı güncellenir, sonra atomik olarak yerine geçer; böylece kayıt başarısız olursa kullanıcı kilitlenmez. API jetonu için yalnızca SHA-256 özeti saklanır.

### Doğrulama katmanı

Komut satırına veya yapılandırma dosyasına giden her değer `model.rs` / `preferences.rs` içinde önce doğrulanır: proje adı `validate_slug`, parola `validate_mysql_password`, git adresi `validate_git_url`, işçi alanları `validate_worker` (tire ile başlayan değerler `--help` gibi argüman karışıklığını önlemek için reddedilir), PowerShell literal'leri `terminal::literal`. Doğrulama hatası Türkçe metindir ve aynen kullanıcıya gösterilir.

## Masaüstü: `src-tauri`

`main.rs` içindeki her `#[tauri::command]` üç şey yapar: parametreleri alır, eylem dizesini `domain` enum'una parse eder, `blocking(state, || …)` ile çekirdek yöntemini bloklayan iş parçacığında `contain` altında çalıştırır. Hata `format!("{e:#}")` ile zincirli Türkçe metne çevrilir.

`desktop.rs` pencere/tepsi tercihlerini, `tray.rs` tepsi menüsünü, `startup.rs` Windows başlangıç kaydını yönetir. Tauri katmanında iş kuralı yoktur; bir davranış hem pencerede hem CLI'da gerekiyorsa çekirdeğe yazılır.

## Arayüz: `src`

| Yol | Görev |
| --- | --- |
| `api.ts` | `call()` — Tauri `invoke` sarmalayıcısı. Okumalar tekilleştirilir (aynı komut için bekleyen çağrı paylaşılır) ve 10 sn'de zaman aşımına düşer; tarayıcı önizlemesinde örnek veri döner. |
| `domain/` | `Page`, `ComponentId`, `ToolAction`, `WorkspaceService` sabitleri. |
| `services/` | IPC çağrılarını adlandıran katman (`environmentService.start()`, `apiService.createToken()`). Bileşenler ham `call("mail")` yazmaz. |
| `repositories/snapshot.ts` | Durum okuma. |
| `hooks/useDraft` | Anlık görüntüden gelen değerin yerel taslağı: kirli değilken kaynağı izler, kirliyken kullanıcı girişini korur. `Services`, `Settings`, `ProjectRelease` bunu kullanır. |
| `hooks/useTheme` | Sistem/açık/koyu tema; `<html data-theme>` üzerinden CSS token setini değiştirir. |
| `components/` | Sayfalar ve paylaşılan yapı taşları: `StatusBadge`, `Toggle`, `NumberField`, `SegmentedControl`, `EmptyState`, `ErrorBoundary`, `LogViewer`, `ServiceConsole`, `ServiceRepair`. |

`App.tsx` 2,5 saniyede bir `snapshot` okur ve `run(label, action)` ile tek işlem yürütür: meşgul etiketi, hata ve başarı bildirimi buradan yönetilir. `Shell` sayfa çerçevesidir ve her sayfayı kapsamlı bir `ErrorBoundary` içine alır; bir sekme çökerse kenar çubuğu ve "Ortamı durdur" erişilebilir kalır.

## Veri klasörü

```text
C:\ServerBond\
  config.json               ayarlar + projeler (2 MB, ≤1000 proje)
  config.last-good.json     son geçerli kopya
  manager.lock              tek süreç kilidi
  bin/<id>/<version>/       indirilen paketler (php, mysql, caddy, composer, phpmyadmin, mailpit, postgres, redis, cloudflared, node)
  cache/                    indirme arşivleri
  config/                   üretilen php.ini / my.ini / Caddyfile / phpmyadmin, DPAPI sırları, api-token.sha256, desktop.json, settings.previous.json
  data/                     mysql, postgres, redis verisi
  backups/                  SQL yedekleri
  logs/<id>.log             her süreç için çıktı
  logs/release-<id>.jsonl   proje sürüm geçmişi
  www/                      yeni Laravel köklerinin varsayılan çalışma alanı
```

Yeni kurulumlarda `C:\ServerBond` ve `SERVERBOND_HOME` kullanılır. Önceki kurulumların veri yolu ve okuma takma adları `legacy.rs` ile korunur; [adlandırma ve uyumluluk](naming.md) belgesine bakın.

## Ağ

Her şey loopback'tir. Varsayılan portlar: web `8088`, HTTPS `8443`, MySQL `13306`, PHP FastCGI `19000` (proje başına otomatik ek port), Mailpit `1025/8025`, PostgreSQL `15432`, Redis `16379`, API `18800`. Proje adresleri `{ad}.localhost`; hosts dosyası düzenlenmez. `Settings::reserved_ports` tüm portların benzersiz olmasını zorlar.

## Yeni özellik eklerken

1. İş kuralını `crates/serverbond-core/src/<modül>.rs` içinde `impl Manager` olarak yazın; girdi doğrulamasını `model.rs`'e koyun.
2. Testini `crates/serverbond-core/tests/` altına ekleyin (Windows'a bağımlıysa `#[cfg(windows)]` veya `#[ignore]`).
3. `src-tauri/src/main.rs` içinde ince bir komut, `generate_handler!` listesinde kayıt.
4. `api.rs` `route()`/`project_route()` içine yol ve `routes()` listesine satır.
5. `bin/serverbond.rs` içine CLI alt komutu.
6. `src/services/` içine sarmalayıcı, `src/types.ts` içine tip, bileşende kullanım. `api.ts` tarayıcı önizlemesine örnek veri ekleyin.
7. Ekran değiştiyse `npm run build && node scripts/ai/screenshots.mjs`.
8. `bash scripts/ai/test.sh` (derleme, fmt, test, clippy) ve `cargo clippy --target x86_64-pc-windows-gnu` temiz olmalı.

Bkz. [api.md](api.md), [design/design-system.md](design/design-system.md), [packages.md](packages.md), [updates.md](updates.md), [ai-environment.md](ai-environment.md).
