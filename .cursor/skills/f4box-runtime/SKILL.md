---
name: f4box-runtime
description: PHP FastCGI, MySQL, Caddy, proje işçileri, onarım ve ayar kaydı. Servis başlatma, port, Laravel proje, terminal veya phpMyAdmin değişince kullan.
---

# Çalışma zamanı

Giriş noktası `Manager` (`crates/f4box-core/src/lib.rs`). Uzun işlemler `gate()` ile tekil; panik `contain()` ile yakalanır ve `faulted` yeni yazmayı kilitler.

## Servisler

- `start("all")` sıra: MySQL → varsayılan PHP → proje PHP → Caddy
- Başarısızlıkta yalnızca bu işlemde açılan süreçler geri alınır (`rollback_new_services`)
- Beklenmedik kapanış otomatik yeniden başlamaz; `service_errors` dolar
- Kapanışta Job Object + MySQL için `mysqladmin shutdown`

Proje PHP’si ayrı `php-cgi` ve otomatik loopback port kullanır. Caddy `php_fastcgi 127.0.0.1:<port>`.

## Kuyruk ve zamanlama

`crates/f4box-core/src/jobs.rs`. İşçi: `queue-{projectUuid}-{workerUuid}-{n}` → `php artisan queue:work` (`--no-ansi`; `maxJobs`/`maxTime` 0 değilse `--max-jobs`/`--max-time`). Zamanlayıcı: `schedule-{projectUuid}` → `schedule:work`. En fazla 8 işçi, işçi başına 8 süreç. `artisan` yoksa veya işçi/zamanlayıcı `enabled` değilse başlatma reddedilir. `.env` yazılmaz. Ortam `start("php"|"all")` sonrası `autoStart` olanlar açılır; kayıt sırasında PHP veya bir iş süreci çalışıyorsa yeni/yeni etkin `autoStart` işçiler ve zamanlayıcı da açılır. `stop` hepsini kapatır. Beklenmedik çıkışta otomatik yeniden başlama yok; `restart_*` durdurup yeniden açar. `queue:failed` / `queue:retry` / `queue:flush` ve işçi/zamanlayıcı günlükleri (`read_project_*_log`) proje kartından ve CLI’dan okunur. `read_project_log` `php` / `schedule` / `worker:<uuid>` kaynaklarını doğrular. `schedule:list` önce `--next` dener.

## Cloudflare tüneli, Mailpit ve Windows izinleri

`tunnel.rs`: servis `cloudflared`, isteğe bağlı paket `tools.json` içinde. Jeton `config/cloudflared-token.dpapi` (DPAPI), sürece `TUNNEL_TOKEN` ortam değişkeniyle geçer; komut satırına yazılmaz. `normalize_token` dashboard komut satırından jetonu ayıklar (`service install`, `--token`). `apply_tunnel` kaydeder, yoksa kurar, başlatır. Hazır olma ölçütü günlükteki `Registered tunnel connection` (`spawn_watched`). `start("all")` sonunda `autoStart` açıksa denenir; hata ortamı düşürmez, `service_errors` dolar.

İsteğe bağlı hizmetlerde `repairable` kurulum klasörünün varlığıdır; eksik `installed.json` veya yürütülebilir `issue` olarak döner. `repair_*` çalışan süreci durdurur, SHA-256 doğrular, program klasörünü atomik değiştirir, `before-repair` kopyasını saklar. `data/`, jeton, parola ve `.env` yazılmaz. Hizmetler kartındaki Onarım onayı aynı sözleşmeyi gösterir.

`mail.rs`: servis `mailpit`, isteğe bağlı paket `tools.json` içinde. Portlar `settings.mail` altında; `Settings::validate` bunları web/mysql/php portlarıyla birlikte tekilleştirir. Hazır olma ölçütü arayüz portunun dinlenmesi (`spawn_service`). `relayPhpMail` açıkken `PhpSettings::render` üretilen php.ini’ye `SMTP`/`smtp_port`/`sendmail_from` yazar; bu anahtarlar `extra_ini` içinde reddedilir. `start("all")` sonunda `autoStart` açıksa denenir; hata ortamı düşürmez.

`redis.rs`: servis `redis`, isteğe bağlı paket `tools.json` içinde. Port `settings.redis` (varsayılan 16379); `Settings::validate` diğer bileşen portlarıyla tekilleştirir. Veri dizini `data/redis`; `redis-server` 127.0.0.1’e bağlanır, parola yok. `start("all")` sonunda `autoStart` açıksa denenir; hata ortamı düşürmez. `.env` yazılmaz. PHP `redis` uzantısı resmî NTS pakette yoktur; Laravel `predis/predis` kullanır.

`postgres.rs`: servis `postgres`, isteğe bağlı paket `tools.json` içinde. Port `settings.postgres` (varsayılan 15432); `Settings::validate` diğer bileşen portlarıyla tekilleştirir. Veri dizini `data/postgresql-17`; parola `config/postgres-password.dpapi` (DPAPI). İlk açılışta `initdb` + `postgres.exe -D -p -h 127.0.0.1` (`pg_ctl` başlatılmaz; süreç hemen çıkar). Kapatırken `pg_ctl stop -m fast`. `start("all")` sonunda `autoStart` açıksa denenir; hata ortamı düşürmez. `.env` yazılmaz. PHP `pgsql` / `pdo_pgsql` uzantıları Ayarlar → PHP’den açılır.

`permissions.rs`: açılışta `ensure_permissions` bir kez UAC açar (reddedilirse `declined` yazılır, bir daha sorulmaz). Betik netsh, `icacls`, isteğe bağlı Defender ve `ServerBond Permissions` zamanlanmış görevini kurar (`RunLevel Highest`). Sonraki paket kurulumları `refresh_permissions_quietly` ile görevi çalıştırır; yeni UAC yoktur. Betik metni saf fonksiyonla üretilir ve testlidir. Uygulamanın kendisi `requireAdministrator` değildir.

## Projeler

- Ad: 1–48 karakter, `[a-z0-9-]` (kenarda tire yok), Windows ayrılmış adları yok (`con`, `com0`, `lpt0`, …)
- Yeni Laravel 12: `php_supports_laravel12` → PHP ≥ 8.2
- `public/index.php` şart; kaldırmak klasörü/SQL’i silmez
- MySQL çalışırken ekleme/oluşturma/tarama `CREATE DATABASE IF NOT EXISTS` yapar (tire → `_`). `.env` yazılmaz
- `discover_projects` `projects_dir` veya varsayılan `projects/` (+ varsa `www/`) altında bir ve iki seviye Laravel köklerini listeler (`musteri/magaza`). `vendor`, `node_modules`, `storage` ve benzeri klasörler atlanır. Çakışan klasör adı `musteri-magaza` olur. `import_projects` tek `apply_project_config` ile ekler
- SQL geri yükleme: `.sql`, 512 MB, Unicode için geçici ASCII kopya, `mysql --one-database`
- GitHub: jeton `config/github-token.dpapi` (DPAPI), hesap `config/github-account.json`. Hizmetler → GitHub bir kez kaydeder. `import_github_project` `git clone https://github.com/owner/repo.git` ile `projects_dir` altına yazar; özel depo için jeton `GIT_CONFIG_*` ortamında (`http.extraHeader`), komut satırına yazılmaz. `.env` yazılmaz. Sürüm `git pull` aynı jetonu kullanır.
- `.env` editörü: `read_project_env` / `save_project_env` yalnızca proje kökü `.env` (256 KB, UTF-8, NUL yok). Otomatik yazılmaz; kullanıcı Kaydet der. İçerik günlüğe yazılmaz. `.env.example` taslak olarak okunabilir.
- Terminal, projenin PHP’sini `PATH`/`php`/`composer` sarmalayıcısıyla verir; `node` kuruluysa dizini de `PATH`’e girer (`node.rs`, servis değil)

## Yerel HTTPS

`settings.web.https` ve `https_port`. Caddyfile `auto_https off` kalır; HTTPS siteleri `tls internal` kullanır. HTTP `redir https://{host}:{https_port}{uri}`. Sertifika `data/caddy` veya `config/Caddy` altında. `trust_https` Windows’ta `certutil -user -addstore Root` (UAC yok). Ortam başlarken denenir; başarısızlık Caddy’yi düşürmez.

## Ayarlar

`save_settings` ortam çalışırken port/php/mysql/web tercihlerini reddeder. `projects_dir`, `backups_dir` ve `start_on_launch` çalışırken kaydedilir. Varsayılan PHP Ayarlar ve Bileşenler’den `select_php` ile seçilir. MySQL root parolası Ayarlar → Sistem’den gösterilir/kopyalanır/`change_mysql_password` ile değiştirilir (MySQL açık olmalı; `.env` yazılmaz). Masaüstü tepsi tercihleri `config/desktop.json` ayrıdır ve çalışırken kaydedilebilir.

## phpMyAdmin

Cookie auth, parola yapıya yazılmaz. Kök `http://phpmyadmin.serverbond.localhost:<web>/`. `config`/`vendor` HTTP’den 404.

## Güvenlik alışkanlıkları

- İndirme: HTTPS, SHA-256, evre klasörü, sonra atomik taşıma
- Arşiv: `enclosed_name`, `:` / `\` / symlink yok, boyut ve dosya sayısı sınırlı
- Yazma: `storage::atomic_write` (aynı birimde temp + persist)
