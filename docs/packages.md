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

MySQL özeti bağımsız imza doğrulaması değildir; resmî HTTPS indirmesini sabitler. Uygulama her kurulumda önbellek dahil dosyayı katalog özetiyle karşılaştırır. Geçici dosyalar tamamlanıp doğrulanmadan kurulum klasörüne taşınmaz. Arşiv yolları hedef klasörü aşamaz; sembolik bağlantılar ve Windows alternatif veri akışı yolları reddedilir.

## Sürüm güncelleme

1. Paketin resmî kaynağını ve Windows x64 desteğini doğrulayın.
2. Tam sürümü, URL'yi ve güvenilir kaynaktaki SHA-256'yı kataloğa birlikte işleyin. `latest` gibi hareketli URL kullanmayın.
3. Arşiv kökü ve çalıştırılabilir dosya yolunu doğrulayın.
4. Ayrı `F4BOX_HOME` altında bütünleşme testini çalıştırın.
5. MySQL için veri yükseltme akışını ayrıca tasarlamadan mevcut veri dizininde sürüm değiştirmeyin.

PHP güncellemelerinde `cargo test -p f4box-core --test php_matrix -- --ignored --nocapture` çalıştırın. PHP varsayılanı değiştirilirse `catalog.json` girdisi de PHP kataloğundaki aynı paketle eşleşmelidir. Önceki kayıtlı sürümleri katalogdan çıkarmayın; uygulama mevcut kullanıcının seçimini çözerken bu girdilere ihtiyaç duyar.

Paketlerin lisansları kendilerine aittir. F4Box internetten resmî paketleri indirir; paketleri kaynak deposunda yeniden dağıtmaz.

## Kaynaklar

- [Tauri Windows gereksinimleri](https://v2.tauri.app/start/prerequisites/)
- [PHP Windows kurulumu](https://www.php.net/manual/en/install.windows.php)
- [PHP desteklenen sürümler ve destek takvimi](https://www.php.net/supported-versions.php)
- [MySQL Windows ZIP kurulumu](https://dev.mysql.com/doc/refman/8.4/en/windows-install-archive.html)
- [Caddy PHP FastCGI](https://caddyserver.com/docs/caddyfile/directives/php_fastcgi)
