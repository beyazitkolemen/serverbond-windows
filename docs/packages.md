# Paket kataloğu

`crates/f4box-core/catalog.json` indirilecek kesin URL, sürüm, SHA-256, arşiv kökü ve çalıştırılabilir dosyayı içerir. Kaynaklar 20 Eylül 2026 tarihinde kontrol edildi.

Seçilebilir PHP paketleri `crates/f4box-core/php-versions.json` içindedir. Her 7.4, 8.0, 8.1, 8.2, 8.3, 8.4 ve 8.5 serisi için bir Windows x64 NTS derlemesi sabitlenir. Tüm PHP özetleri resmî JSON listesindeki ilgili `nts-<derleyici>-x64.zip.sha256` alanından alınır. 7.4–8.1 paketleri resmî `archives/` dizininden, 8.2–8.5 paketleri `releases/` dizininden indirilir.

| Paket | Sürüm / hash kaynağı |
| --- | --- |
| PHP | `https://downloads.php.net/~windows/releases/releases.json`, her serinin `nts-*-x64.zip.sha256` alanı |
| Caddy | `https://api.github.com/repos/caddyserver/caddy/releases/latest`, Windows amd64 ZIP `digest` alanı |
| Composer | `https://getcomposer.org/download/2.10.3/composer.phar.sha256sum` |
| phpMyAdmin | [Resmî 5.2.3 all-languages ZIP SHA-256](https://files.phpmyadmin.net/phpMyAdmin/5.2.3/phpMyAdmin-5.2.3-all-languages.zip.sha256) |
| MySQL | `https://cdn.mysql.com/Downloads/MySQL-8.4/mysql-8.4.10-winx64.zip`, resmî HTTPS kaynağından indirilen 280672277 baytlık dosyanın yerelde hesaplanan SHA-256 özeti |

İsteğe bağlı araçlar `crates/f4box-core/tools.json` içindedir ve ortamın çalışması için gerekmez.

| Araç | Dosya / sürüm | Özet kaynağı | Kurulum yeri |
| --- | --- | --- | --- |
| Cloudflare Tunnel bağlayıcısı | `cloudflared-windows-amd64.exe` 2026.9.1, 54976432 bayt | yerelde hesaplandı | Hizmetler → Tünel |
| Mailpit | `mailpit-windows-amd64.zip` 1.31.2, 10479883 bayt | yerelde hesaplandı | Hizmetler → E-posta |
| Node.js | `node-v24.21.0-win-x64.zip` | resmî [`SHASUMS256.txt`](https://nodejs.org/dist/v24.21.0/SHASUMS256.txt) | Ayarlar → Sistem |
| PostgreSQL | `postgresql-17.11-1-windows-x64-binaries.zip`, 340719294 bayt | yerelde hesaplandı | Hizmetler → PostgreSQL |
| Redis | `Redis-8.10.2-Windows-x64-msys2.zip`, 13938657 bayt | GitHub release `digest` `7c8cebd5…ee85` | Hizmetler → Redis |

Cloudflared, Mailpit ve PostgreSQL yayımlanmış bir özet dosyası sunmadığı için bu kayıtların SHA-256'sı indirilen dosyadan yerelde hesaplanmıştır. Redis özeti GitHub release `digest` alanından alınmıştır. MySQL, cloudflared, Mailpit ve PostgreSQL özetleri bağımsız imza doğrulaması değildir; resmî HTTPS indirmesini sabitler. Uygulama her kurulumda önbellek dahil dosyayı katalog özetiyle karşılaştırır. Geçici dosyalar tamamlanıp doğrulanmadan kurulum klasörüne taşınmaz. Arşiv yolları hedef klasörü aşamaz; sembolik bağlantılar ve Windows alternatif veri akışı yolları reddedilir.

## Sürüm güncelleme

1. Paketin resmî kaynağını ve Windows x64 desteğini doğrulayın.
2. Tam sürümü, URL'yi ve güvenilir kaynaktaki SHA-256'yı kataloğa birlikte işleyin. `latest` gibi hareketli URL kullanmayın.
3. Arşiv kökü ve çalıştırılabilir dosya yolunu doğrulayın.
4. Ayrı `SERVERBOND_HOME` altında bütünleşme testini çalıştırın.
5. MySQL için veri yükseltme akışını ayrıca tasarlamadan mevcut veri dizininde sürüm değiştirmeyin.

PHP güncellemelerinde `cargo test -p f4box-core --test php_matrix -- --ignored --nocapture` çalıştırın. PHP varsayılanı değiştirilirse `catalog.json` girdisi de PHP kataloğundaki aynı paketle eşleşmelidir. Önceki kayıtlı sürümleri katalogdan çıkarmayın; uygulama mevcut kullanıcının seçimini çözerken bu girdilere ihtiyaç duyar.

Paketlerin lisansları kendilerine aittir. ServerBond internetten resmî paketleri indirir; paketleri kaynak deposunda yeniden dağıtmaz.

## Kaynaklar

- [Tauri Windows gereksinimleri](https://v2.tauri.app/start/prerequisites/)
- [PHP Windows kurulumu](https://www.php.net/manual/en/install.windows.php)
- [PHP desteklenen sürümler ve destek takvimi](https://www.php.net/supported-versions.php)
- [MySQL Windows ZIP kurulumu](https://dev.mysql.com/doc/refman/8.4/en/windows-install-archive.html)
- [PostgreSQL Windows ikili arşivi](https://www.enterprisedb.com/download-postgresql-binaries)
- [Caddy PHP FastCGI](https://caddyserver.com/docs/caddyfile/directives/php_fastcgi)
