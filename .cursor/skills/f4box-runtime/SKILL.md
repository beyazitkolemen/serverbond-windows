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

## Projeler

- Ad: 1–48 karakter, `[a-z0-9-]` (kenarda tire yok), Windows ayrılmış adları yok (`con`, `com0`, `lpt0`, …)
- Yeni Laravel 12: `php_supports_laravel12` → PHP ≥ 8.2
- `public/index.php` şart; kaldırmak klasörü/SQL’i silmez
- Terminal, projenin PHP’sini `PATH`/`php`/`composer` sarmalayıcısıyla verir

## Ayarlar

`save_settings` ortam çalışırken port/php/mysql/web tercihlerini reddeder. Masaüstü tepsi tercihleri `config/desktop.json` ayrıdır ve çalışırken kaydedilebilir.

## phpMyAdmin

Cookie auth, parola yapıya yazılmaz. Kök `http://phpmyadmin.f4box.localhost:<web>/`. `config`/`vendor` HTTP’den 404.

## Güvenlik alışkanlıkları

- İndirme: HTTPS, SHA-256, evre klasörü, sonra atomik taşıma
- Arşiv: `enclosed_name`, `:` / `\` / symlink yok, boyut ve dosya sayısı sınırlı
- Yazma: `storage::atomic_write` (aynı birimde temp + persist)
