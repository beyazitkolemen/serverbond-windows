# Herd, Laragon, Forge ve F4Box

İnceleme tarihi: 20 Eylül 2026. Kapsam: Laravel Herd, Laragon ve Laravel Forge’un kamuya açık belgelerindeki roller ile F4Box’ın Windows üzerindeki konumu. Üç ürünün arayüzü bu incelemede tek tek tıklanarak doğrulanmamıştır; belgelerde yazılmayan davranış varsayılmamıştır. Laragon satırları için ayrıntı: [laragon-incelemesi.md](laragon-incelemesi.md).

F4Box bir yerel geliştirme kopyası (Herd / Laragon) değildir. Amaç, Windows x64 sunucuda Laravel uygulamasını **üretim olarak** çalıştırmaktır: sabit PHP/MySQL/Caddy, proje PHP’si, kuyruk, zamanlayıcı ve yapılandırılmış sürüm (git + Composer + Artisan). Laravel Forge’un uzak Linux VPS, SSH ve push-to-deploy katmanı yoktur; üretim bu Windows makinesindedir.

## Ürün rolleri

| Ürün | Rol | F4Box karşılığı |
| --- | --- | --- |
| **Laravel Herd** | macOS/Windows **geliştirme**: yerel PHP/nginx, site isolate, Pro’da dump / Xdebug / günlük, Expose | PHP 7.4–8.5, proje PHP, Caddy, HTTPS, Mailpit, Cloudflare tüneli. Dump, Xdebug ve Expose yok; bunlar geliştirme aracıdır. |
| **Laragon** | Taşınabilir **geliştirme** WAMP; Quick-add, Quick-app, Mailpit, Auto SSL, Procfile | Caddy + MySQL 8.4, isteğe bağlı PostgreSQL 17, Mailpit, HTTPS, kuyruk/zamanlayıcı. Redis/Apache ve serbest Procfile yok. |
| **Laravel Forge** | Uzak Linux VPS üretimi: git push-to-deploy, script, zero-downtime, Supervisor, Let’s Encrypt | **Aynı iş, bu Windows makinede**: sürüm tarifi, kuyruk/zamanlayıcı, tünel. Uzak SSH ve sunucu provision yok. |

```mermaid
flowchart LR
  subgraph dev [Geliştirme araçları]
    Herd
    Laragon
  end
  subgraph windows [Windows üretimi]
    F4Box
  end
  subgraph remote [Uzak Linux VPS]
    Forge
  end
  F4Box -->|"Sürüm"| Recipe[git_composer_artisan]
  Recipe --> Restart[kuyruk_ve_zamanlama]
```

## Forge Deployments’ın yerel karşılığı

Forge’da sürüm uzak sunucuda çalışır: depo çekilir, isteğe bağlı script çalışır, kuyruk işçileri Supervisor üzerinden yenilenir. F4Box aynı sırayı **proje klasöründe** uygular; kabuk scripti yoktur.

| Forge (uzak) | F4Box (yerel, bu makine) |
| --- | --- |
| Git pull / seçilen dal | `git pull`; isteğe bağlı dal. `git` PATH’te yoksa Türkçe hata. |
| `composer install --no-dev` (üretim) | Katalogdaki Composer + projenin PHP’si; isteğe bağlı `--no-dev`. |
| `php artisan migrate --force` | Aynı sabit bayraklar. |
| Optimizasyon / cache | `php artisan optimize:clear --no-interaction --no-ansi` |
| Serbest deploy scripti | Yalnızca `a-z0-9:_-` Artisan komutu + `--bayrak` / `--bayrak=değer` |
| Supervisor restart | Kayıtlı ve etkin `queue:work` işçileri + `schedule:work` |
| 10 dk civarı tavan | Ortam meşgul kilidi, birleşik çıktı (`--- git ---`), 10 dakika |
| Deploy geçmişi | Son 20 kayıt: `logs/release-{proje}.jsonl` |

`.env` yazılmaz. Serbest PowerShell, uzak SSH ve “Forge-lite” sunucu yönetimi yoktur.

## Bilinçli olarak eklenmeyenler

Ürün kuralı ve önceki kapsam: Apache/Nginx seçimi, Redis, Horizon, uzak SSH, Expose/ngrok, dump penceresi, Xdebug uzantısı, serbest PowerShell scripti, zero-downtime symlink sürümü ve genel sertifika otoritesi. PostgreSQL isteğe bağlı bir araçtır; MySQL varsayılan kalır.

Herd’in site isolate ve paylaşım katmanı Cloudflare tüneli ile kısmen karşılanır; genel adres eşlemesi Cloudflare panelindedir. Laragon Quick-add paketleri F4Box katalog doğrulamasıyla sınırlıdır.

## Doğrulama

Sürüm tarifinin enjeksiyon reddi, adım sırası, `git` yokken hata, `migrate --force --no-ansi` ve eski `config.json` yüklemesi çekirdek testlerindedir. Yeni kurulumda `display_errors` kapalıdır; hatalar günlüğe yazılır. Arayüz: proje detayı → **Sürüm**. CLI: `f4box project release <ad>`.
