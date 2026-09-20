---
name: f4box-catalog
description: PHP, MySQL, Caddy, Composer veya phpMyAdmin sürüm/SHA-256 kataloğunu güncelleme. Paket URL, hash, arşiv kökü veya indirme onarımı sorulduğunda kullan.
---

# Paket kataloğu

Kaynak: `crates/f4box-core/catalog.json`, `crates/f4box-core/php-versions.json`, `crates/f4box-core/tools.json` (isteğe bağlı araçlar; ortamı bloke etmez), [docs/packages.md](../../../docs/packages.md).

## Kurallar

1. Resmî Windows x64 kaynağını doğrula. `latest` URL yasak.
2. Sürüm, URL, SHA-256, `archive`/`prefix`/`executable` aynı commit’te değişir.
3. PHP varsayılanı değişirse her iki JSON’daki `php` kaydı aynı paket olur.
4. Kayıtlı kullanıcı sürümünü çözmek için eski yama satırını silme.
5. MySQL sürümünü, veri taşıma akışı yokken `data/mysql-8.4` ile bağlama.

## Hash kaynakları

| Paket | Kaynak |
| --- | --- |
| PHP | `https://downloads.php.net/~windows/releases/releases.json` → `nts-*-x64.zip.sha256` |
| Caddy | GitHub release Windows amd64 ZIP `digest` |
| Composer | `https://getcomposer.org/download/<sürüm>/composer.phar.sha256sum` |
| phpMyAdmin | files.phpmyadmin.net all-languages ZIP SHA-256 |
| MySQL | Resmî HTTPS ZIP’in yerelde hesaplanan SHA-256’sı (imza değil) |
| Cloudflared | GitHub release `cloudflared-windows-amd64.exe` dosyasının yerelde hesaplanan SHA-256’sı |
| Mailpit | GitHub release `mailpit-windows-amd64.zip` dosyasının yerelde hesaplanan SHA-256’sı |
| Node.js | `https://nodejs.org/dist/v<sürüm>/SHASUMS256.txt` → `node-v<sürüm>-win-x64.zip` |
| PostgreSQL | Resmî EDB Windows x64 ZIP’in yerelde hesaplanan SHA-256’sı (`postgres.exe` ile çalıştır; `pg_ctl` başlatmaz) |

7.4–8.1 arşiv dizininden, 8.2–8.5 `releases/` dizininden iner. 404/410 olursa `install::archive_fallback` aynı dosya adını `archives/` altında arar.

## Doğrulama

Windows, ayrı `F4BOX_HOME`:

```powershell
cargo test -p f4box-core --test php_matrix -- --ignored --nocapture
```

Linux asistan yalnızca katalog biçimini ve birim testlerini çalıştırır; paketi indirmez.
