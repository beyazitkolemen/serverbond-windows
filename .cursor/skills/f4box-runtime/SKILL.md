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

`crates/f4box-core/src/jobs.rs`. İşçi: `queue-{projectUuid}-{workerUuid}-{n}` → `php artisan queue:work`. Zamanlayıcı: `schedule-{projectUuid}` → `schedule:work` (crontab’daki dakikalık `schedule:run`). En fazla 8 işçi, işçi başına 8 süreç. `artisan` yoksa başlatma reddedilir. `.env` yazılmaz. Ortam `start("php"|"all")` sonrası `autoStart` olanlar açılır; `stop` hepsini kapatır. Beklenmedik çıkışta otomatik yeniden başlama yok.

## Cloudflare tüneli, Mailpit ve Windows izinleri

`tunnel.rs`: servis `cloudflared`, isteğe bağlı paket `tools.json` içinde. Jeton `config/cloudflared-token.dpapi` (DPAPI), sürece `TUNNEL_TOKEN` ortam değişkeniyle geçer; komut satırına yazılmaz. Hazır olma ölçütü günlükteki `Registered tunnel connection` (`spawn_watched`). `start("all")` sonunda `autoStart` açıksa denenir; hata ortamı düşürmez, `service_errors` dolar.

`mail.rs`: servis `mailpit`, isteğe bağlı paket `tools.json` içinde. Portlar `settings.mail` altında; `Settings::validate` bunları web/mysql/php portlarıyla birlikte tekilleştirir. Hazır olma ölçütü arayüz portunun dinlenmesi (`spawn_service`). `relayPhpMail` açıkken `PhpSettings::render` üretilen php.ini’ye `SMTP`/`smtp_port`/`sendmail_from` yazar; bu anahtarlar `extra_ini` içinde reddedilir. `start("all")` sonunda `autoStart` açıksa denenir; hata ortamı düşürmez.

`permissions.rs`: açılışta `ensure_permissions` bir kez UAC açar (reddedilirse `declined` yazılır, bir daha sorulmaz). Betik netsh, `icacls`, isteğe bağlı Defender ve `F4Box Permissions` zamanlanmış görevini kurar (`RunLevel Highest`). Sonraki paket kurulumları `refresh_permissions_quietly` ile görevi çalıştırır; yeni UAC yoktur. Betik metni saf fonksiyonla üretilir ve testlidir. Uygulamanın kendisi `requireAdministrator` değildir.

## Projeler

- Ad: 1–48 karakter, `[a-z0-9-]` (kenarda tire yok), Windows ayrılmış adları yok (`con`, `com0`, `lpt0`, …)
- Yeni Laravel 12: `php_supports_laravel12` → PHP ≥ 8.2
- `public/index.php` şart; kaldırmak klasörü/SQL’i silmez
- Terminal, projenin PHP’sini `PATH`/`php`/`composer` sarmalayıcısıyla verir; `node` kuruluysa dizini de `PATH`’e girer (`node.rs`, servis değil)

## Ayarlar

`save_settings` ortam çalışırken port/php/mysql/web tercihlerini reddeder. Masaüstü tepsi tercihleri `config/desktop.json` ayrıdır ve çalışırken kaydedilebilir.

## phpMyAdmin

Cookie auth, parola yapıya yazılmaz. Kök `http://phpmyadmin.f4box.localhost:<web>/`. `config`/`vendor` HTTP’den 404.

## Güvenlik alışkanlıkları

- İndirme: HTTPS, SHA-256, evre klasörü, sonra atomik taşıma
- Arşiv: `enclosed_name`, `:` / `\` / symlink yok, boyut ve dosya sayısı sınırlı
- Yazma: `storage::atomic_write` (aynı birimde temp + persist)
