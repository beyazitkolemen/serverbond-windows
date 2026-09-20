# Laragon incelemesi ve F4Box ayarları

İnceleme tarihi: 20 Eylül 2026. Kapsam: Laragon'un resmî belgeleri ile bu depodaki Rust/Tauri uygulamasının karşılaştırılması. Laragon'un Windows arayüzünü kurup bütün menülerini test ettiğimiz anlamına gelmez. Belgelerde açıklanmayan davranışlar varsayılmamıştır.

## Bulgular ve karşılaştırma

| Alan | Laragon'da belgelenen davranış | F4Box'taki karşılık / açık kalan iş |
| --- | --- | --- |
| Kullanıcı ayarları | `usr` kullanıcı tercihlerini, `usr/tpl` kalıcı şablonları tutar. Güncellemede korunur. [Dizin yapısı](https://laragon.org/docs/directory-structure), [Flexible](https://laragon.org/docs/flexible) | Tercihler `config.json` içinde. PHP, MySQL, Caddy ve phpMyAdmin dosyaları buradan üretilir. Ayarlar onarımda korunur. Önceki tercihler ayrıca yedeklenir. Serbest Caddy/MySQL şablon editörü yok. |
| Proje / veri klasörleri | Document Root ve veritabanı veri dizini Preferences üzerinden değiştirilebilir. [Dizin yapısı](https://laragon.org/docs/directory-structure) | Yeni proje ve SQL yedek klasörü seçilebilir. Kayıtlı projeler kendi yolunda kalır. Mevcut MySQL veri dizinini taşıma sihirbazı yok; ana konum başlangıçta `F4BOX_HOME` ile belirlenebilir. |
| Otomatik adresler | Document Root altındaki proje klasörleri Reload sonrası sanal sunucuya dönüşür; `{name}.test` kalıbı değiştirilebilir. Özel vhost dosyaları korunabilir. [Pretty URLs](https://laragon.org/docs/pretty-urls) | Eklenen projelerin `{name}.localhost` kalıbı değiştirilebilir; örneğin `{name}.dev.localhost`. Tüm kayıtlı adresler birlikte güncellenir. **Klasör tara** yeni projeler klasöründeki `public/index.php` köklerini listeler ve toplu ekler. `.test` için hosts yönetimi yok. |
| Web sunucusu | Apache / Nginx sürümleri ve yapılandırmaları yönetilebilir. [Multi-Version](https://laragon.org/docs/multi-version), [CLI](https://laragon.org/docs/cli) | Caddy kullanılır. Port, FastCGI bağlantı/yanıt süreleri, sıkıştırma ve erişim günlüğü kullanıcı ayarıdır. Apache/Nginx seçimi ve `.htaccess` desteği yok. Laravel'in `public` kökü kullanılır. |
| PHP sürümleri | PHP paketlerini ekleme ve menüden sürüm değiştirme belgelenir. [Multi-Version](https://laragon.org/docs/multi-version), [Operations](https://laragon.org/docs/operations) | PHP 7.4–8.5 kataloğu, varsayılan sürüm ve proje bazlı bağımsız PHP süreçleri mevcut. Ortak profil ve sürüme özel profil eklenmiştir. Aynı PHP sürümünü kullanan projeler aynı ini profilini paylaşır. |
| Veritabanları | MySQL yanında PostgreSQL vb. paketler ve sürümler eklenebilir. [Quick-add](https://laragon.org/docs/quick-add) | MySQL 8.4 LTS yönetilir. Port, InnoDB belleği, bağlantı/paket limitleri, zaman aşımı, SQL modu, utf8mb4 karşılaştırması ve yavaş sorgu kaydı ayarlanır. İsteğe bağlı PostgreSQL 17 (`tools.json`, port 15432, Ayarlar → PostgreSQL) ortamı bloke etmez; `.env` yazılmaz. MySQL ana sürüm geçişi yok. |
| phpMyAdmin | Quick-add üzerinden kurulabilir. [Operations](https://laragon.org/docs/operations) | Doğrulanmış 5.2.3 paketi, cookie ile giriş, aç/kapat, dil, satır sayısı ve oturum süresi ayarları. Gelişmiş configuration-storage tabloları otomatik kurulmaz. |
| HTTPS | Yerel HTTPS, sertifika yönetimi ve sertifikayı güven deposuna ekleme bulunur. [Auto SSL](https://laragon.org/docs/auto-ssl) | Ayarlar → Web sunucusu: `https` ve `httpsPort` (varsayılan 8443). Caddy `tls internal` ile yerel CA üretir; HTTP HTTPS’e yönlenir. Sertifika Windows kullanıcı Root deposuna yazılır (`certutil -user`). Ortam başlarken denenir; Ayarlar’dan ekle/kaldır. |
| Hızlı proje oluşturma | `sites.conf` ile proje tarifleri, otomatik veritabanı oluşturma ve paket önbelleği yönetilir. [Quick-app](https://laragon.org/docs/quick-app) | Laravel 12 Composer kurulumu ve mevcut Laravel projesi ekleme var. MySQL çalışıyorsa proje adıyla (tire → alt çizgi) veritabanı `CREATE DATABASE IF NOT EXISTS` ile açılır. `.env` yazılmaz. WordPress/Symfony tarifleri ve kullanıcı tanımlı komut kataloğu yok. |
| Paket ekleme | `packages.conf` içindeki indirme adresleri değiştirilebilir; toplu ekleme vardır. [Quick-add](https://laragon.org/docs/quick-add) | Sabit sürüm + SHA-256 doğrulamalı beş bileşen, PHP sürüm kataloğu, toplu kurulum ve onarım. Kullanıcının keyfî indirme adresi eklemesi desteklenmez. |
| Terminal | Cmder tabanlı, sekmeli ve izole PATH kullanan terminal sunar. [Terminal](https://laragon.org/docs/terminal) | Projeye ait PHP/Composer, kuruluysa Node.js ve PostgreSQL `bin` dizini ile PowerShell açılır. Sistem PATH'i değiştirilmez. Terminal uygulaması/editör tercihi henüz yok. |
| Başlangıç / süreçler | Procfile; `autorun`, çalışma dizini ve env dosyası seçenekleriyle özel süreçler yönetilebilir. [Easy-to-Extend](https://laragon.org/docs/easy-to-extend) | Windows oturum açılışında F4Box, tepside açılış ve uygulama açılışında servisleri başlatma ayrı tercihlerdir. Proje kartında Laravel `queue:work` işçileri (`--max-jobs`/`--max-time`, yeniden başlatma, günlük, `queue:failed`) ve `schedule:work` zamanlayıcısı yönetilir. Genel Procfile yok. |
| E-posta | Mailpit yerel SMTP yakalama ve web arayüzü sağlar; PHP mail() entegrasyonu açıklanır. [Mailpit](https://laragon.org/docs/mailpit) | Ayarlar → E-posta bölümünde sabit `mailpit` paketi: SMTP ve arayüz portu, saklama sınırı, ortamla otomatik başlatma, tepsiden gelen kutusu ve isteğe bağlı PHP `mail()` yönlendirmesi. Laravel `.env` dosyası F4Box tarafından yazılmaz; değerler kartta gösterilir. |
| Paylaşım | Ngrok tabanlı dış paylaşım, token ve bölge seçenekleri bulunur. [Quick-share](https://laragon.org/docs/quick-share) | Ayarlar → Tünel bölümünde Cloudflare Tunnel: sabit `cloudflared` paketi, DPAPI ile şifrelenen jeton, başlat/durdur ve ortamla otomatik başlatma. Ngrok ve bölge seçimi yok; genel adres eşlemesi Cloudflare panelinde yapılır. |
| Taşınabilirlik | Laragon klasörünün başka sürücü/bilgisayara taşınması belgelenir. [Portable](https://laragon.org/docs/portable) | Programlar özel klasörde tutulur. F4Box parolaları Windows DPAPI ile mevcut hesaba bağlıdır; farklı bilgisayara klasör kopyalamak tam taşınabilirlik sağlamaz. Taşıma/export için ayrı tasarım gerekir. |
| Hızlı erişim | Sistem tepsisi menüsü servis, günlük ve terminal işlemlerini toplar. [Context Menu](https://laragon.org/docs/context-menu) | Windows tepsisinde başlat/durdur/yeniden başlat, servis alt menüleri, phpMyAdmin, ayarlar, günlükler, veri klasörü ve çıkış var. Tepsiye küçültme ve ikinci açılışta mevcut pencereyi gösterme desteklenir. Proje/terminal işlemleri ana pencerede. |

## Bu değişiklikte kullanılabilir ayarlar

**Genel:** yeni proje klasörü, uygulama açılışında ortamı başlatma, web / MySQL / varsayılan FastCGI portları. Port aralığı 1–65535; 80 gibi standart portlar başka uygulama kullanmıyorsa seçilebilir.

**PHP:** saat dilimi, bellek, dosya yükleme ve POST limitleri, çalışma/giriş süreleri, giriş değişkeni sayısı, hata gösterme/kaydetme, OPcache ve belleği, uzantılar, ek `anahtar=değer` ini seçenekleri. Ortak profil tüm sürümlere temel olur. Sürüm profili tam bir kopya olarak ayrılır; ortak ayarlar sonraki değişikliklerde bu profili değiştirmez. “Ortak ayarlara dön” özel profili kaldırır. PHP 7.4 GD kütüphanesi `gd2` adıyla yüklenir; PHP 8.5 gömülü OPcache için DLL yüklenmez. OPcache kapatıldığında 8.5 üzerinde de açıkça devre dışı kalır. Bkz. [PHP OPcache değişikliği](https://wiki.php.net/rfc/make_opcache_required).

**MySQL:** InnoDB buffer pool, en fazla bağlantı, en büyük paket, boşta bağlantı süresi, karşılaştırma düzeni, SQL modları, yavaş sorgu kaydı ve eşiği. Karakter seti utf8mb4; root parolası otomatik üretilip şifreli saklanır. Ayarlar → Sistem’den gösterilir, kopyalanır ve MySQL çalışırken değiştirilir. Karşılaştırma tercihi var olan tabloları dönüştürmez.

**Web:** proje alan adı kalıbı, FastCGI bağlantı ve yanıt süreleri, Gzip/Zstandard sıkıştırma, erişim günlükleri, isteğe bağlı yerel HTTPS ve HTTPS portu. HTTP loopback korunur; HTTPS açıkken HTTP istekleri yönlendirilir. `.localhost` dışında DNS/hosts yönetimi uygulanmadığından diğer son ekler kabul edilmez. Proje adresi değişikliği `.env` içindeki `APP_URL` değerini otomatik değiştirmez.

**phpMyAdmin:** web erişimini aç/kapat, varsayılan dil, sayfa başına satır, cookie/oturum süresi. PHP limitleri varsayılan PHP profilinden alınır. Oturum veya tarayıcı tercihleri varsayılan dilin önüne geçebilir. Kapatmak program dosyalarını ya da MySQL verilerini silmez.

**E-posta:** Mailpit SMTP portu, arayüz portu, saklanacak en fazla e-posta sayısı, ortamla otomatik başlatma ve PHP `mail()` yönlendirmesi. Yönlendirme açıkken `SMTP` ve `smtp_port` anahtarları üretilen php.ini'ye yazılır; bu yüzden aynı anahtarlar ek PHP ayarları alanında reddedilir. Yakalanan e-postalar `data/mailpit/mailpit.db` dosyasında tutulur ve gerçek alıcıya iletilmez.

**PostgreSQL:** isteğe bağlı 17.11 sunucusu, port (varsayılan 15432), ortamla otomatik başlatma, kur/başlat/durdur/onar, parola göster/kopyala/değiştir. Veri dizini `data/postgresql-17`; parola DPAPI. Laravel `.env` yazılmaz; PHP `pgsql` / `pdo_pgsql` uzantıları PHP sekmesinden açılır.

**Yedek ve aktarım:** yeni SQL yedekleri için mevcut klasör seçimi; proje kartından `.sql` geri yükleme (`mysql --one-database`); tercihlerin JSON olarak dışa/içe aktarımı; önceki tercihler ve varsayılanları taslağa alma. Aktarım F4Box tarafından saklanan MySQL yönetici parolasını, proje listesini, SQL verilerini ve paketleri kapsamaz.

## Uygulama ve doğrulama kuralları

1. Ayarlar servisler duruyorken kaydedilir. Formda gezinmek/değişiklik yapmak tek başına etkin ayarları değiştirmez.
2. Portlar, aralıklar, klasörler ve ayar yapısı Rust tarafında doğrulanır. Bilinmeyen JSON alanları kabul edilmez.
3. Kurulu PHP sürümleri geçici ini dosyasıyla gerçekten çalıştırılır: uzantılar, saat dilimi, ini anahtarları ve başlangıç hataları denetlenir. Kurulu olmayan sürüm için runtime kontrolü ilk kullanımda yapılır.
4. Geçersiz ayar ana yapılandırmaya yazılmaz. Kaydetmeden önce önceki tercihler `config/settings.previous.json` dosyasına alınır; bu tek adımlık geri dönüş içindir.
5. Ana yapılandırma geçici dosya + atomik değiştirme ile kaydedilir. Servis dosyaları bundan yeniden oluşturulur. Onarım program klasörünü yeniler, kullanıcı profillerini silmez.
6. İçe aktarma/varsayılana dönme/önceki ayarları getirme önce taslak yükler. Kullanıcı ayrıca kaydeder. Sıfırlama mevcut portları korur.
7. Yeni proje/yedek klasörü seçimi dosya taşımaz. SQL yedek ve geri yüklemede MySQL'in Unicode yol kısıtını aşmak için önce yönetilen geçici dosya oluşturulur.
8. MySQL ve Caddy için sunulan alanlar sınırlı, doğrulanan seçeneklerdir; keyfî yapılandırma metni çalıştırılmaz. Bu, Laragon'un bütün şablon esnekliğiyle aynı kapsam değildir.

Uygulanan direktifler için birincil kaynaklar: [PHP ini](https://www.php.net/manual/en/ini.core.php), [MySQL 8.4 değişkenleri](https://dev.mysql.com/doc/refman/8.4/en/server-system-variables.html), [Caddy FastCGI](https://caddyserver.com/docs/caddyfile/directives/php_fastcgi), [phpMyAdmin yapılandırması](https://docs.phpmyadmin.net/en/latest/config.html).

## Öncelikli devam işleri

- Yerel HTTPS ve sertifika yaşam döngüsü (v1.1: Caddy `tls internal`, HTTPS portu, kullanıcı deposu güveni).
- Node.js sürümleri arasında geçiş (v1.1: tek sabit LTS paketi).
- Terminal/editör seçimi.
- Proje bazlı queue/scheduler süreçleri (v1.1: `queue:work` ve `schedule:work`).
- Dış paylaşım (v1.1: Cloudflare Tunnel bağlayıcısı; genel adres eşlemesi Cloudflare panelinde).
- Yerel e-posta yakalama (v1.1: Mailpit; dışa giden gerçek SMTP aktarımı yok).
- MySQL veri taşıma, parola değiştirme ve zamanlanmış yedek (v1.1: yedek + `.sql` geri yükleme).
- Tepsi menüsü ve kullanıcı tercihiyle Windows açılışında başlama.
- Otomatik proje keşfi ve özel vhost/web kökü yönetimi (v1.1: `projects` / `www` ve isteğe bağlı çalışma alanı; iki seviye müşteri/uygulama taraması; özel vhost yok).

Bunlar tamamlanmış özellikler değildir. Mevcut ayar ekranı, bugün F4Box tarafından yönetilen PHP/MySQL/Caddy/phpMyAdmin bileşenlerinin günlük geliştirme seçeneklerini kapsar.
