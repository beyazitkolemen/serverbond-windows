# ServerBond

Windows x64 üzerinde Laravel uygulamasını **üretim gibi** çalıştıran Rust + Tauri masaüstü uygulaması. Laravel Forge’un uzak VPS katmanı yoktur; PHP, MySQL, Caddy, kuyruk, zamanlayıcı ve yapılandırılmış sürüm aynı makinede yönetilir. Herd veya Laragon geliştirme kopyası değildir.

PHP sürümü seçimi, proje PHP’si, kuyruk ve zamanlama süreçleri, phpMyAdmin, Mailpit, isteğe bağlı PostgreSQL, GitHub’dan proje ekleme, Cloudflare tüneli, sistem tepsisi ve Windows başlangıç tercihleri aynı panelden yönetilir.

## İndir — v1.1.1

Uygulama sürümü **1.1.1**. Windows x64 yayını `ServerBond_1.1.1_x64-setup.exe`, `ServerBond_1.1.1_x64.exe` ve `SHA256SUMS.txt` dosyalarını içerir.

- [Windows x64 kurulum EXE'si](https://github.com/beyazitkolemen/serverbond-windows/releases/download/v1.1.1/ServerBond_1.1.1_x64-setup.exe): ServerBond'ı kurar ve WebView2 gereksinimini yönetir.
- [Doğrudan çalıştırılabilir EXE](https://github.com/beyazitkolemen/serverbond-windows/releases/download/v1.1.1/ServerBond_1.1.1_x64.exe): WebView2 kurulu bir Windows x64 bilgisayarda açılabilir; verileri `%LOCALAPPDATA%\ServerBond` altında saklar.
- [v1.1.1 sürüm notları](docs/releases/v1.1.1.md)

Bu sürüm elle kurulur; otomatik güncelleme için imzalı paket içermez. Önceki kurulumların verileri korunur; ayrıntılar [adlandırma ve uyumluluk](docs/naming.md) belgesindedir.

Kaynak kodunu indirmeniz veya derlemeniz gerekmez. PHP/MySQL/Caddy gibi bileşenler ilk kullanımda ayrıca indirilir.

Kurulu masaüstü uygulaması **Ayarlar → Güncellemeler** veya tepsi menüsünden GitHub’daki son sürümü denetler. Yeni paket siz onaylamadan kurulmaz. Bu kanalın çalışması için deponun herkese açık olması ve imzalı bir GitHub Release (`latest.json`) yayımlanmış olması gerekir. Ayrıntı: [docs/updates.md](docs/updates.md).

![ServerBond genel bakış: PHP, MySQL ve Caddy servisleri çalışırken](docs/screenshots/01-genel-bakis.png)

## Ekran görüntüleri

Görüntüler uygulamanın kendi arayüzünden alınmıştır; tasarım maketi değildir. Proje adları, portlar ve süreç kimlikleri örnek veridir.

| Bileşen yönetimi                                                       | Kuyruk ve zamanlama                                                                  |
| ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| ![Bileşenler](docs/screenshots/02-bilesenler.png)                      | ![Projeler, kuyruk işçileri ve zamanlayıcı](docs/screenshots/03-projeler-kuyruk.png) |
| Yerel e-posta yakalama                                                 | Node.js ve Windows izinleri                                                          |
| ![E-posta ayarları ve Mailpit servisi](docs/screenshots/04-eposta.png) | ![Node.js kurulumu ve Windows izinleri](docs/screenshots/06-sistem.png)              |
| Yerel HTTPS                                                            | Proje günlükleri                                                                     |
| ![Web sunucusu ve Auto SSL](docs/screenshots/08-web-https.png)         | ![Proje günlük görüntüleyicisi](docs/screenshots/09-proje-gunlukleri.png)            |
| Yerel sürüm                                                            | İsteğe bağlı PostgreSQL                                                              |
| ![Proje sürüm tarifi](docs/screenshots/10-proje-surum.png)             | ![İsteğe bağlı PostgreSQL](docs/screenshots/11-postgresql.png)                       |
| GitHub hesabı                                                          | GitHub’dan proje                                                                     |
| ![GitHub hesabı](docs/screenshots/12-github.png)                       | ![GitHub’dan proje ekle](docs/screenshots/13-proje-github.png)                       |
| Proje .env                                                             | İsteğe bağlı Redis                                                                   |
| ![Proje .env editörü](docs/screenshots/14-proje-env.png)               | ![İsteğe bağlı Redis](docs/screenshots/15-redis.png)                                 |
| Hizmetler                                                              | Cloudflare tüneli                                                                    |
| ![Hizmetler listesi](docs/screenshots/16-hizmetler.png)                | ![Cloudflare tüneli ayarları](docs/screenshots/05-tunel.png)                         |

[PHP ayarları ve tam boy görüntüler →](docs/screenshots/README.md)

## Kullanım

Derlenmiş `ServerBond` uygulamasını açın. **Bileşenleri kur** ile gerekli paketleri hazırlayın; **Ortamı başlat** ile MySQL, PHP FastCGI ve Caddy'yi çalıştırın. **Proje ekle** ekranında mevcut Laravel kök klasörünü seçin, **GitHub** sekmesinden depo klonlayın veya **Yeni Laravel projesi** sekmesinden Laravel 12 oluşturun.

Projeler `http://proje-adi.localhost:8088` biçimindeki adreslerden açılır. `.localhost` alan adı kullanıldığı için hosts dosyası düzenlenmez. **Ayarlar → Web sunucusu** içinde yerel HTTPS açılırsa adres `https://proje-adi.localhost:8443` olur; HTTP istekleri yönlendirilir. Caddy dahili CA’sı Windows kullanıcı güven deposuna yazılabilir. İlk açılışta 8088/13306/19000 portları doluysa bir sonraki boş portlar seçilip kaydedilir. Daha sonra kaydedilmiş bir port başka uygulama tarafından kullanılırsa ServerBond o süreci durdurmaz; Ayarlar'dan boş bir port seçin. Geçerli adres proje satırında görünür.

**Hizmetler → GitHub → Ayarlar** özel depolar için kişisel erişim jetonunu bir kez kaydeder. Jeton Windows DPAPI ile şifrelenir; `git clone` / `git pull` komut satırına yazılmaz. Genel depolar jeton olmadan da klonlanır. Depo `projects` (veya Ayarlar’daki çalışma alanı) altına iner; `.env` yazılmaz. Komut: `serverbond github token <jeton>|forget|import <depo> [ad] [dal]`.

**Klasör tara** proje çalışma alanındaki (`projects`, eski `www` veya Ayarlar’daki yol) Laravel köklerini listeler. Hem `magaza` hem `musteri/magaza` bulunur; `vendor`, `node_modules` ve benzeri klasörler atlanır. Aynı klasör adı çakışırsa iç klasör `musteri-magaza` olur. MySQL çalışırken proje eklemek veya Laravel oluşturmak, proje adıyla (tire → alt çizgi) veritabanını `IF NOT EXISTS` ile açar; `.env` yazılmaz. Proje kartından SQL yedeği alınır ve `.sql` geri yüklenir.

| Bileşen         | Sabit sürüm                                                  | İşlev                               |
| --------------- | ------------------------------------------------------------ | ----------------------------------- |
| PHP x64 NTS     | 7.4.33 / 8.0.30 / 8.1.34 / 8.2.33 / 8.3.33 / 8.4.25 / 8.5.10 | Seçilebilir PHP CLI ve FastCGI      |
| MySQL Community | 8.4.10 LTS                                                   | Yerel veritabanı                    |
| Caddy           | 2.11.4                                                       | Proje yönlendirme ve dosya sunma    |
| Composer        | 2.10.3                                                       | Laravel proje kurulumu              |
| phpMyAdmin      | 5.2.3, tüm diller                                            | Tarayıcıdan MySQL yönetimi          |
| Cloudflared     | 2026.9.1                                                     | İsteğe bağlı Cloudflare tüneli      |
| Mailpit         | 1.31.2                                                       | İsteğe bağlı yerel e-posta yakalama |
| PostgreSQL      | 17.11                                                        | İsteğe bağlı pgsql sunucusu         |
| Node.js         | 24.21.0 LTS                                                  | İsteğe bağlı npm / npx              |

Paketler uygulama kurulum paketine gömülmez; ilk kullanımda resmî kaynaklarından indirilir. Windows x64 Visual C++ 2015–2022 Redistributable ve WebView2 Runtime gerekir. Bu bilgisayarda ikisi de mevcuttur. Başka bir bilgisayarda PHP/MySQL başlatılamıyorsa önce [Microsoft Visual C++ Runtime](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist) kurulmalıdır. Tauri kurulum paketi WebView2 gereksinimini yönetir.

### PHP sürümü seçimi

Genel bakış veya Bileşenler ekranındaki **PHP sürümü** listesinden sürümü seçip **İndir ve kullan** düğmesine basın. Yalnızca seçilen Windows x64 NTS paketi indirilir ve resmî SHA-256 özetiyle doğrulanır. Kurulu sürüm için **Bu sürümü kullan** düğmesi görünür; tekrar indirilmez. Varsayılan sürüm 8.4.25'tir.

7.4 ve 8.0–8.5 serilerinin katalogdaki sabit yama sürümleri sunulur. Sürüm listesi `crates/serverbond-core/php-versions.json` dosyasından gelir; çevrimiçi en son sürüme kendiliğinden güncellenmez. 7.4/8.0/8.1, eski projelerle uyumluluk için bulunur ve resmî güvenlik desteği sona ermiştir.

**Varsayılan PHP sürümü** yeni projeler ve genel CLI için kullanılır. Her proje kartındaki PHP listesinden ayrı sürüm seçilebilir; örneğin bir proje PHP 7.4, diğeri PHP 8.4 ile aynı anda çalışabilir. Eksik paket **İndir ve uygula** ile indirilir. Her proje ayrı bir PHP FastCGI süreci ve otomatik seçilen yerel port kullanır. Seçim uygulama yeniden açıldığında korunur. Önceki yapılandırmalar açılışta mevcut varsayılan sürüme sabitlenir; proje dosyaları değiştirilmez.

Projenin sürümü değiştirilmeden önce paket ve uzantılar doğrulanır. Yalnızca ilgili PHP süreci değiştirilir; Caddy yönlendirmesi kısa bir yeniden başlatmayla güncellenir. Diğer projelerin PHP süreçleri ve MySQL korunur. Yeni süreç başlatılamazsa eski proje seçimi ve süreç geri yüklenir. Varsayılan PHP değişikliği mevcut projelerin sürümlerini değiştirmez.

### Proje terminali ve gereksinimler

Proje kartındaki **Terminal**, o proje klasöründe Windows PowerShell açar. `php`, `composer` ve Composer'ın `@php` alt komutları projenin seçili PHP sürümünü kullanır. Örneğin `php artisan migrate` doğrudan çalıştırılabilir. Kuruluysa `node`/`npm`/`npx` ve PostgreSQL `bin` (`psql`) de PATH’e eklenir. Yalnızca açılan terminalin ortamı ayarlanır; sistem PATH'i değişmez. Proje sürümü değiştikten sonra açık terminali kapatıp yeniden açın. PHP ve Composer paketlerinin kurulu olması gerekir; terminal açılması servisleri başlatmaz.

### Kuyruk ve zamanlama

Her proje kartında Supervisor benzeri kuyruk işçileri ve Laravel zamanlayıcı vardır. İşçi `php artisan queue:work` sürecidir: bağlantı, kuyruk adı, süreç sayısı, zaman aşımı, bellek, azami iş (`--max-jobs`) ve azami süre (`--max-time`) ayarlanır. Sıfır değer bayrağı eklemez. Birden fazla işçi (örneğin `default` ve `emails`) eklenebilir. **Ortamla başlat** açıkken **Ortamı başlat** bu süreçleri de açar; ayrı **Başlat / Durdur / Yeniden başlat** ile tek tek yönetilir. Ortam çalışırken kaydedilen yeni veya yeni etkinleştirilen `Ortamla başlat` işçileri de açılır.

**Laravel schedule (crontab)** `php artisan schedule:work` çalıştırır; bu, Linux crontab’daki `* * * * * php artisan schedule:run` karşılığıdır. Görevler `routes/console.php` veya `app/Console` içinde tanımlanır; ServerBond `.env` dosyasını yazmaz. **Görevler** `schedule:list --next` çıktısını gösterir. **Günlük** işçi ve zamanlayıcı süreç kayıtlarını açar. **Başarısız kuyruk işleri** `queue:failed`, `queue:retry all` ve `queue:flush` komutlarını çalıştırır. `artisan` dosyası olmayan klasörlerde kuyruk başlatılmaz. Kapalı işçi veya zamanlayıcı başlatılamaz.

Windows Görev Zamanlayıcısı kullanılmaz; süreçler ServerBond kapanınca durur. Bellek, azami iş veya süre sınırından çıkan işçi otomatik yeniden başlamaz; **Yeniden başlat** ile açın. Redis kuyruğu için **Hizmetler → Redis** ile isteğe bağlı sunucu kurulur; `database` veya `sync` bağlantısı yerel MySQL ile de kullanılabilir. Komut satırı: `serverbond queue <ad> start|stop|restart|failed|retry|flush|log [işçi|iş]`, `serverbond schedule <ad> start|stop|restart|list|log` ve `serverbond logs <ad> [php|schedule|işçi]`.

### Yerel sürüm

Proje detayındaki **Sürüm** sekmesi Laravel Forge Deployments’ın bu Windows makinesindeki karşılığıdır. Uzak VPS, SSH ve serbest kabuk scripti yoktur. Tarif işaret kutularıdır: `git pull` (isteğe bağlı dal), `composer install --prefer-dist` (isteğe bağlı `--no-dev`), `php artisan migrate --force`, `optimize:clear`, ek Artisan satırları (`a-z0-9:_-` + `--bayrak`) ve kayıtlı kuyruk/zamanlayıcı süreçlerinin yeniden başlatılması. `.env` yazılmaz. Çıktı birleştirilir (`--- git ---`); tavan 10 dakikadır; son 20 kayıt `logs/release-{proje}.jsonl` dosyasına yazılır. `git` PATH’te yoksa işlem Türkçe hata ile durur. Komut satırı: `serverbond project release <ad>`. Karşılaştırma: [docs/herd-forge-karsilastirma.md](docs/herd-forge-karsilastirma.md).

### Proje .env

Proje detayındaki **Ortam** sekmesi kök `.env` dosyasını açar. ServerBond kurulum, proje ekleme, sürüm veya veritabanı işlemlerinde `.env` yazmaz; yalnızca bu sekmede **Kaydet** yazdırır. 256 KB ve UTF-8 sınırı vardır. `.env.example` varsa **Örnekten doldur** taslağı doldurur. İçerik günlüğe yazılmaz. PHP veya kuyruk açıksa kayıttan sonra süreçleri yeniden başlatın. Komut: `serverbond env <ad>`.

### Proje günlükleri

Her proje kartındaki **Günlükler** paneli PHP FastCGI, Laravel zamanlayıcı ve kuyruk işçisi süreç kayıtlarını aynı görüntüleyicide açar. Kaynak sekmeleri, metin araması, satır numarası, hata/uyarı vurgusu, kopyalama ve açıkken otomatik yenileme vardır. **Günlükler** sayfası aynı görüntüleyiciyi ServerBond, PHP, MySQL, Caddy, Composer ve PostgreSQL için kullanır.

### Node.js, npm ve npx

**Ayarlar → Sistem → Node.js** sabit Node.js LTS paketini kurar. Özet resmî `SHASUMS256.txt` dosyasından alınır. Kurulumdan sonra proje kartındaki **Terminal** penceresinde `node`, `npm` ve `npx` projenin PHP sürümüyle birlikte hazır olur; `npm install` ve `npm run dev` doğrudan çalışır. Sistem PATH'i değiştirilmez, bu yüzden bilgisayarınızdaki başka bir Node kurulumu etkilenmez. Açık terminalleri kurulumdan sonra kapatıp yeniden açın. Komut satırından: `serverbond node install|repair|status`.

### Hizmet onarımı

Hizmet kartındaki **Onarım** program dosyalarını SHA-256 doğrulanmış paketten yeniden kurar ve onay ister. Eksik kurulumda **Kur** yerine **Kurulumu onar** görünür. Yakalanan e-postalar, veritabanı dizini, jeton, parola ve proje `.env` dosyalarına dokunulmaz. Önceki program klasörü `before-repair` adıyla saklanır.

### Yerel e-posta yakalama

**Hizmetler → E-posta** bölümü Mailpit'i yönetir: yerel bir SMTP sunucusu projelerinizin gönderdiği e-postaları yakalar ve tarayıcıdaki gelen kutusunda gösterir. Hiçbir ileti gerçek alıcıya iletilmez. Paket sabit sürümdür, SHA-256 doğrulanarak indirilir ve ortamın çalışması için gerekli değildir.

SMTP portu (varsayılan 1025), arayüz portu (varsayılan 8025) ve saklanacak en fazla e-posta sayısı ayarlanabilir; portlar diğer bileşenlerin portlarından farklı olmak zorundadır. Laravel tarafında `.env` dosyasına `MAIL_MAILER=smtp`, `MAIL_HOST=127.0.0.1` ve `MAIL_PORT=1025` yazın; kullanıcı adı ve parola gerekmez. ServerBond `.env` dosyanızı değiştirmez.

**PHP mail() çağrılarını Mailpit'e yönlendir** açıkken üretilen `php.ini` dosyasına `SMTP` ve `smtp_port` anahtarları yazılır, böylece Laravel dışındaki kodun `mail()` çağrıları da yakalanır. Bu anahtarlar bu formdan yönetildiği için PHP sekmesindeki ek ayarlar alanına yazılamaz; değişiklik PHP yeniden başladığında geçerli olur.

**Ortam başlatıldığında Mailpit'i de başlat** açıkken **Ortamı başlat** gelen kutusunu da açar. Mailpit başlatılamazsa ortam çalışmaya devam eder; hata e-posta kartında ve **Günlükler → mailpit** bölümünde görünür. Yakalanan e-postalar veri klasöründeki `data/mailpit/mailpit.db` dosyasında tutulur. Tepsi menüsündeki **Gelen kutusunu aç** ve komut satırındaki `serverbond mail install|start|stop|open|status` aynı işi yapar.

### İsteğe bağlı PostgreSQL

**Hizmetler → PostgreSQL** bölümü resmi EDB Windows x64 arşivinden PostgreSQL 17 kurar. MySQL varsayılan kalır; PostgreSQL ortamın çalışması için gerekli değildir. Port varsayılanı 15432’dir (sistem 5432 ile çakışmaz). İlk başlatmada veri dizini `data/postgresql-17` oluşturulur; `postgres` kullanıcısının parolası Windows DPAPI ile saklanır. Sunucu `postgres.exe` ile 127.0.0.1’e bağlanır; kapatırken `pg_ctl stop` kullanılır.

Laravel tarafında `.env` dosyasına `DB_CONNECTION=pgsql`, `DB_HOST=127.0.0.1`, `DB_PORT=15432`, `DB_USERNAME=postgres` ve ServerBond’ın gösterdiği parolayı yazın. ServerBond `.env` dosyanızı değiştirmez. PHP `pgsql` ve `pdo_pgsql` uzantıları Ayarlar → PHP’den açılır.

**Ortam başlatıldığında PostgreSQL'i de başlat** açıkken **Ortamı başlat** PostgreSQL’i de açar. PostgreSQL başlatılamazsa ortam çalışmaya devam eder; hata PostgreSQL kartında ve **Günlükler → postgres** bölümünde görünür. Tepsi menüsü → Servisler → PostgreSQL ve komut satırı `serverbond postgres install|start|stop|repair|password [parola]|status` aynı işi yapar.

### İsteğe bağlı Redis

**Hizmetler → Redis** Laravel kuyruk, önbellek ve oturum için Redis 8 kurar. Ortamın çalışması için gerekli değildir. Port varsayılanı 16379’dur (sistem 6379 ile çakışmaz). Sunucu `redis-server` ile 127.0.0.1’e bağlanır; parola yoktur. Veri dizini `data/redis`. ServerBond `.env` yazmaz.

Laravel tarafında `.env` dosyasına `REDIS_CLIENT=predis`, `REDIS_HOST=127.0.0.1`, `REDIS_PORT=16379` ve isteğe bağlı `CACHE_STORE=redis` / `QUEUE_CONNECTION=redis` yazın. Resmî PHP NTS paketinde `redis` uzantısı yoktur; `predis/predis` kullanın.

**Ortam başlatıldığında Redis'i de başlat** açıkken **Ortamı başlat** Redis’i de açar. Redis başlatılamazsa ortam çalışmaya devam eder; hata Redis kartında ve **Günlükler → redis** bölümünde görünür. Tepsi menüsü → Servisler → Redis ve komut satırı `serverbond redis install|start|stop|repair|status` aynı işi yapar.

### Cloudflare tüneli

**Hizmetler → Tünel** bölümü Cloudflare Tunnel bağlayıcısını (`cloudflared`) yönetir. Sabit sürüm SHA-256 doğrulanarak indirilir; diğer bileşenler gibi ServerBond klasörüne kurulur ve ortamın çalışması için gerekli değildir.

1. Cloudflare Zero Trust → Networks → Tunnels ekranında tünel oluşturun. **Install and run a connector** adımındaki jetonu veya tüm `cloudflared.exe service install …` satırını kopyalayın.
2. **Hizmetler → Tünel** alanına yapıştırın. ServerBond jetonu ayıklar ve Windows DPAPI ile mevcut hesaba bağlı olarak şifreler; günlüklere ve komut satırına yazılmaz, sürece ortam değişkeni olarak verilir.
3. **Kaydet ve tüneli başlat** Cloudflared’ı yoksa kurar, jetonu kaydeder ve bağlayıcıyı çalıştırır. Hangi genel adresin hangi porta gittiğini Cloudflare panelindeki tünel yapılandırması belirler.

Kurulum, onarım, başlat/durdur ve **Ortam başlatıldığında tüneli de başlat** aynı karttan yönetilir. Tepsi menüsü → Servisler → Cloudflare tüneli aynı başlat/durdur işini yapar. Tünel başlatılamazsa ortam çalışmaya devam eder; hata tünel kartında ve **Günlükler → cloudflared** bölümünde görünür. Komut satırından: `serverbond tunnel install|token <jeton>|apply <jeton>|start|stop|status`.

### Windows izinleri

ServerBond gündelik işini yönetici yetkisi olmadan yapar: PHP, MySQL ve Caddy yalnızca `127.0.0.1` üzerinde dinler, dosyalar kendi veri klasörüne yazılır ve `.localhost` adresleri hosts dosyası gerektirmez. Uygulama açılırken Windows’tan **bir kez** tam yetki ister. Onay şunları uygular:

- ServerBond'ın çalıştırdığı programlar (kurulu PHP sürümleri, `mysqld`, `caddy`, varsa `cloudflared`, `mailpit`, `redis-server` ve `node`) için güvenlik duvarında özel ve etki alanı profillerinde gelen/giden izin kuralı.
- Veri klasöründe Windows kullanıcınıza tam erişim (`icacls`).
- Microsoft Defender'da veri klasörü istisnası (açılıştaki istek bunu da ister; Ayarlar’dan kapatılabilir).
- Sonraki kurulumlarda yeniden sormamak için `ServerBond Permissions` zamanlanmış görevi (en yüksek yetki).

Yükseltilmiş yetkiyle yalnızca ServerBond'ın ürettiği bu betik çalışır; uygulamanın kendisi her açılışta yükseltilmez. Onay bir kez verildikten sonra yeni PHP sürümü veya Mailpit kurulumu güvenlik duvarına sessizce eklenir. İstek reddedilirse ServerBond bir daha kendiliğinden sormaz; **Ayarlar → Sistem** ekranından yeniden istenebilir. Komut satırından: `serverbond permissions ensure|grant [defender]`.

**Ayarlar → Kurulum gereksinimleri**, Windows x64, Visual C++ x64 çalışma zamanı, Windows PowerShell, veri klasörüne yazma, disk alanı ve portları denetler. ServerBond'a ait açık portlar kullanılabilir kabul edilir; başka uygulamanın portu hata olarak gösterilir. Visual C++ eksikse Microsoft indirme bağlantısı sunulur. Paket indirmeden önce platform, çalışma zamanı ve yazma erişimi denetlenir; düşük disk alanı uyarı olarak gösterilir. WebView2 kurulumu Tauri kurulum paketi tarafından yönetilir.

Her sürüm `bin/php/<sürüm>/` altında, üretilen PHP ayarları `config/php/<sürüm>/php.ini` altında tutulur. Uzantı dizini kullanılan PHP paketine aittir; eski kurulumların ortak `config/php.ini` dosyası artık kullanılmaz. ServerBond bu ayar dosyalarını başlangıçta yeniden üretir. Windows genel PATH ayarı değiştirilmez.

PHP CLI ve FastCGI uzantıları sürüm geçişinden önce ayrı ayrı doğrulanır. Türkçe karakterli veri yolları için ServerBond uzantı yolunu süreç argümanı ve `SERVERBOND_PHP_EXT` ortam değişkeniyle geçirir; Composer alt süreçleri aynı ayarı devralır. Üretilen `php.ini` başka bir terminalde kullanılacaksa bu ortam değişkeni de seçilen sürümün `ext` klasörünü göstermelidir.

**Yeni Laravel projesi** Laravel 12 oluşturduğu için PHP 8.2 veya üzeri gerektirir. Daha eski PHP seçiliyken işlem dosya oluşturmadan açıklama gösterir; mevcut projeler eklenebilir.

### MySQL

**phpMyAdmin:** Bileşenler ekranından phpMyAdmin'i indirin veya **Bileşenleri kur** ile tümünü kurun. PHP, MySQL ve Caddy çalışırken phpMyAdmin satırındaki **Aç** düğmesini kullanın. Adres `http://phpmyadmin.serverbond.localhost:<web-portu>/` biçimindedir. Ayrı bir phpMyAdmin servisi gerekmez; varsayılan PHP sürümü kullanılır. Mevcut projelerin PHP seçimleri değişmez.

Kullanıcı `root`; parola **Ayarlar → Sistem → MySQL bağlantısı → Parolayı göster** bölümündedir. MySQL portu her web sunucusu başlangıcında ServerBond ayarlarından alınır. Giriş cookie kimlik doğrulaması kullanır; MySQL parolası phpMyAdmin yapılandırmasına yazılmaz. Oturum ve geçici dosyalar `data/phpmyadmin/` altında, web kökünün dışında saklanır. Yalnızca yerel bilgisayardan erişilir; yapılandırma ve kurulum dizinleri HTTP üzerinden açılmaz. `config.inc.php` ServerBond tarafından üretilir. Onarım MySQL verilerini değiştirmez.

phpMyAdmin'in isteğe bağlı yapılandırma depolaması tabloları otomatik oluşturulmaz. Bu nedenle gelişmiş özelliklerle ilgili bir bildirim görülebilir; veritabanlarını görüntüleme ve SQL çalıştırma kullanılabilir.

- İlk başlangıçta veri dizini hazırlanır ve rastgele root parolası atanır.
- Parola Ayarlar → Sistem → MySQL bağlantısı üzerinden gösterilir, kopyalanır ve MySQL çalışırken değiştirilir; diskte kullanıcıya bağlı Windows DPAPI ile şifrelenir. `.env` yazılmaz.
- Varsayılan bağlantı `127.0.0.1:13306`, kullanıcı `root`.
- Proje satırındaki veritabanı düğmesi proje adıyla veritabanı oluşturur; tireler alt çizgiye dönüşür.
- Yedek düğmesi `mysqldump` ile SQL yedeği alır. Yedekler `backups/` altında saklanır.
- Mevcut Laravel `.env` dosyaları otomatik değiştirilmez. Bağlantı bilgilerini projenizin `.env` dosyasına girin. Yeni Laravel kurulumu kendi SQLite varsayılanıyla gelir.

```dotenv
DB_CONNECTION=mysql
DB_HOST=127.0.0.1
DB_PORT=13306
DB_DATABASE=proje_adi
DB_USERNAME=root
# Ayarlar ekranındaki parolayı buraya yazın; kaynak kontrolüne eklemeyin.
```

### Dosyalar

Varsayılan veri dizini `%LOCALAPPDATA%\ServerBond`; geliştirme/test için `SERVERBOND_HOME` ile değiştirilebilir.

```text
ServerBond/
  bin/       # sürüme göre ayrılmış programlar
  cache/     # SHA-256 doğrulanan indirme önbelleği
  config/    # PHP, Caddy, MySQL ayarları ve şifreli parola
  data/      # kalıcı MySQL verileri
  backups/   # SQL yedekleri
  logs/      # servis ve kurulum kayıtları
  projects/  # Laravel çalışma alanı (müşteri/uygulama alt klasörleri taranır)
  www/       # eski düz proje klasörü; hâlâ taranır
  config.json
```

Uygulama kapanırken servisler durur. Windows Job Objects beklenmeyen kapanışta alt süreçlerin açık kalmasını önler. MySQL normal kapanışta `mysqladmin shutdown` kullanır. Aynı veri klasörünü ikinci bir ServerBond süreci açamaz. Projeyi listeden kaldırmak proje klasörünü veya veritabanını silmez.

### Sorun giderme ve onarım

- **Kurulum eksik:** Yardımcı dosyalar ve kurulum kaydı kontrol edilir. Ortamı durdurup Bileşenler ekranındaki **Onar** düğmesine basın. Bozuk, etkin olmayan PHP sürümü listeden seçilince **Onar ve kullan** görünür.
- Onarım SHA-256 doğrulanmış önbellekten veya resmî indirmeden yapılır. Yeni paket tamamen açılmadan mevcut klasör değiştirilmez. Önceki program klasörü `bin/<bileşen>/<sürüm>-before-repair-<kimlik>` adıyla korunur; `data`, `projects`, `www`, `backups` ve proje `.env` dosyalarına dokunulmaz.
- **Beklenmedik servis kapanışı:** Bileşenler ekranındaki uyarıyı ve ilgili günlüğü inceleyin; **Başlat** başarılı olduğunda uyarı temizlenir.
- Portlar kaydedilirken kullanımda olup olmadıkları kontrol edilir. Port sonradan başka program tarafından alınırsa servis başlangıcında tekrar kontrol edilir.
- PHP resmî sürüm adresi 404/410 döndürürse aynı sabit paket resmî arşivde aranır; SHA-256 kontrolü değişmez. İndirmede bağlantı için 30 saniye, tüm aktarım için 30 dakika bekleme sınırı vardır.
- PHP doğrulaması 20 saniye, MySQL sorguları 30 saniye ile sınırlıdır. Zaman aşımında sahip olunan alt süreçler kapatılır. Yedek dosyaları benzersiz ad taşır; peş peşe yedekler birbirinin üzerine yazılmaz.

## Geliştirme

Gereksinimler: Windows x64, Node.js 22.12+ veya 24, Rust stable MSVC, Microsoft C++ Build Tools (Desktop development with C++) ve WebView2. `rust-toolchain.toml` `stable` kullanır; Windows masaüstü derlemesi için rustup varsayılanı `x86_64-pc-windows-msvc` olmalıdır. Cloud Agent kurulumu ve hızlı testler: [docs/ai-environment.md](docs/ai-environment.md).

```powershell
npm ci
npm run desktop
```

`npm run dev` yalnızca salt okunur tarayıcı önizlemesini `http://127.0.0.1:1420` adresinde açar. Kurulum ve servis işlemleri için `npm run desktop` kullanın. Önizleme sahte kurulum sonucu üretmez.

### Derleme

```powershell
npm run desktop:build
```

Çalıştırılabilir dosya `target/release/serverbond-desktop.exe`; NSIS kurulum paketi `target/release/bundle/nsis/` altında üretilir. Yerel veya CI imzasız paketi `npm run desktop:build:unsigned` ile üretilir. İmzalı güncelleme paketi ve `latest.json` için `TAURI_SIGNING_PRIVATE_KEY` ile `npm run desktop:build` veya `.github/workflows/release.yml` kullanılır. Windows CI imzasız `ServerBond_<sürüm>_x64.exe`, `ServerBond_<sürüm>_x64-setup.exe` ve `SHA256SUMS.txt` yükler.

### Doğrulama

```powershell
npm run test:ai
```

Linux Cloud Agent ve çekirdek denetimleri `npm run test:ai` ile çalışır. Tam workspace clippy ve Tauri derlemesi Windows’ta:

```powershell
npm run build
cargo test -p serverbond-core
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Gerçek paketleri indirip MySQL/PHP/Caddy ile bütünleşme testi:

```powershell
# Masaüstü uygulaması kapalı olmalı. Ayrı veri klasörü istenirse SERVERBOND_HOME ayarlayın.
npm run test:integration
```

Bu test boş portlar seçer; geçici projeyle PHP FastCGI yönlendirmesini, PDO MySQL uzantısını, MySQL yazma/okumayı, SQL yedeğini, gizli dosya engelini ve servislerin yeniden başlamasını doğrular. Teste ait veritabanı/proje temizlenir ve önceki port ayarları geri yüklenir. Kurulan paketler ve MySQL veri dizini korunur.

CLI de aynı çekirdeği kullanır:

```powershell
cargo run -p serverbond-core --bin serverbond -- status
cargo run -p serverbond-core --bin serverbond -- install all
cargo run -p serverbond-core --bin serverbond -- php
cargo run -p serverbond-core --bin serverbond -- php 7.4.33
cargo run -p serverbond-core --bin serverbond -- serve
cargo run -p serverbond-core --bin serverbond -- add benim-projem C:\Projeler\benim-projem
cargo run -p serverbond-core --bin serverbond -- queue benim-projem start default
cargo run -p serverbond-core --bin serverbond -- queue benim-projem failed
cargo run -p serverbond-core --bin serverbond -- schedule benim-projem list
cargo run -p serverbond-core --bin serverbond -- logs benim-projem php
```

Tüm PHP paketlerini indirerek FastCGI, uzantılar, Composer, çalışan ortamda sürüm geçişi, başarısız geçişten geri dönüş ve seçim kalıcılığı testi (ayrı geçici veri dizini kullanır):

```powershell
cargo test -p serverbond-core --test php_matrix -- --ignored --nocapture
cargo test -p serverbond-core --test environment -- --ignored --nocapture
```

`environment` testi ayrı ve geçici MySQL veri dizininde ilk kurulumu, Türkçe SQL verisini, yedekleri ve MySQL program onarımından sonra verinin korunmasını sınar. İsteğe bağlı `SERVERBOND_TEST_CACHE` mevcut bir indirme önbelleğini gösterir; dosyalar test klasörüne kopyalanıp yeniden doğrulanır, mevcut veritabanı kullanılmaz.

PHP matrisi testi ayrıca iki projenin eşzamanlı farklı sürüm kullanmasını, diğer projenin süreç kimliğinin korunmasını, proje terminalindeki PHP ve Composer alt süreçlerini, Türkçe/kesme işaretli yolları ve proje sürümü geçişinin geri alınmasını sınar. Proje PHP günlükleri proje kartından açılabilir. Bozuk bir proje PHP paketi, ortam durdurulduktan sonra **Onar ve uygula** ile varsayılan sürüm değiştirilmeden onarılabilir.

## İlk sürümün sınırları

- Windows x64, proje başına seçilebilir PHP 7.4–8.5 ve katalogdaki MySQL sürümü desteklenir. Redis isteğe bağlıdır. Node.js tek sabit LTS paketiyle sunulur; sürümler arasında geçiş yoktur.
- Ortam bu Windows makinesinde üretim içindir; Caddy loopback’e bağlanır. Dışarı Cloudflare tüneli ile açılır. Her proje tek bir PHP FastCGI süreci kullanır. Hatalar varsayılan olarak sayfada gösterilmez.
- Yeni Laravel oluşturma PHP bağımlılıklarını kurar; frontend bağımlılıkları ve Vite derlemesi proje terminalinden `npm` ile yapılır.
- MySQL sürüm yükseltmesi, otomatik veri taşıma ve yedekten geri yükleme henüz yoktur. Veri klasörünü başka MySQL sürümüyle açmayın.
- DPAPI parolası Windows kullanıcısına bağlıdır; veri dizininin başka bilgisayara kopyalanması tek başına taşınabilir kurulum sağlamaz. Taşıma için SQL yedeği kullanın.
- Kurulum paketi kod imzalı değildir. Mevcut Herd/Laragon/Docker kurulumları ve proje `.env` dosyaları değiştirilmez.

## Hata yönetimi ve kurtarma

- Masaüstü komutlarında yakalanabilen Rust panikleri uygulama durumunu kilitlemez. Yeni işlemler engellenir; durum, günlükler ve servisleri durdurma kullanılabilir kalır. Ekrandaki yönlendirmeyle ortamı durdurup uygulamayı yeniden açın.
- Bozuk `config.json` veya mevcut veri/yedek varken kaybolan yapılandırma, masaüstünde kurtarma ekranını açar. **Son geçerli yedeği geri yükle**, `config.last-good.json` içindeki önceki ayarları ve proje listesini geri getirir. Bozuk dosya `config.corrupt-<kimlik>.json` olarak saklanır. Geçerli yedek yoksa uygulama dosyayı sıfırlamaz. Bu işlem SQL/veritabanı yedeğini geri yüklemez.
- Ayarlar, üretilen PHP/MySQL/Caddy dosyaları ve şifrelenmiş parolalar geçici dosyaya yazılıp diske aktarılır, ardından hedef dosya değiştirilir. İzin, disk veya dosya kilidi hatasında eski dosya korunur. Yapılandırmanın önceki geçerli sürümü ayrı tutulur.
- Daha önce hazırlanmış MySQL'in veri klasörü kayıpsa boş veritabanı oluşturulmaz. Mevcut veri ile parola dosyası tutarsızsa başlatma durur. Sistem denetimi bu sorunları ve açık süreçlerin yanıt vermeyen portlarını gösterir.
- Yapılandırma 2 MB, kurulum kaydı 256 KB, şifrelenmiş anahtar 64 KB ile sınırlıdır. Arşivlerde dosya sayısı, toplam açılmış boyut, gerçek dosya uzunluğu ve boş disk alanı denetlenir. Yakalanan komut çıktısı işlem sürerken izlenir; 8 MB sınırı veya süre sınırı aşılırsa ilgili alt süreç kapatılır.
- Günlük görüntüleme son 64 KB ile sınırlıdır. ServerBond'ın kendi günlüğü 4 MB üzerinde döndürülür; bir önceki dosya saklanır. PHP/MySQL/Caddy'nin sürekli yazdığı servis günlükleri bu döndürme kapsamına girmez.
- Beklenmedik servis kapanmaları görünür hata oluşturur; otomatik yeniden başlatma döngüsü yoktur. Başlangıç başarısızlığında yalnızca o işlemde başlatılan süreçler geri alınır. Kapatmada sahip olunan tüm servislere durdurma uygulanır.
- Arayüz çizim hatalarında yeniden yükleme ekranı gösterilir. Durum okuması yanıt vermediğinde işlemler devre dışı kalır; aynı bekleyen okuma tekrar gönderilmez. Uzun süren yazma/kurulum işlemleri arayüz zaman aşımıyla yeniden başlatılmaz.

Elektrik kesintisi, işletim sisteminin süreci zorla kapatması veya bellek tükenmesi için kesintisiz çalışma garantisi yoktur. Yapılandırma yedeği veritabanı yedeğinin yerini tutmaz; önemli veriler için SQL yedeği alın. Otomatik testler arasında bozuk/kayıp yapılandırma, geçersiz yedek, kilitli dosya, kayıp MySQL verisi/parolası, iç panik ve aşırı komut çıktısı senaryoları bulunur.

## Yönetim API'si

Arayüzün ve CLI'nın yaptığı her iş `http://127.0.0.1:18800/api/v1` altındaki yerel HTTP API'den de yapılabilir: hizmetleri başlat/durdur, proje ekle veya Git'ten klonla, sürüm çalıştır, kuyruk ve zamanlayıcıyı yönet, `.env` ve ayarları oku/yaz. API varsayılan olarak kapalıdır; **Ayarlar → API** bölümünden açılır ve bir kez gösterilen `Authorization: Bearer` jetonu oluşturulur (diskte yalnızca SHA-256 özeti kalır). Komut satırı: `serverbond api serve|token|forget|status|routes`. Tüm yollar, gövdeler ve örnekler: [docs/api.md](docs/api.md).

## Mimari

- `crates/serverbond-core`: `Manager` cephesi — katalog, güvenli indirme/arşiv açma, süreç yönetimi, MySQL, projeler, kuyruk/zamanlayıcı, sürüm, yerel API, CLI.
- `src-tauri`: dar kapsamlı masaüstü IPC komutları, tepsi ve Windows başlangıcı.
- `src`: React/TypeScript arayüzü; `services/` IPC katmanı, `hooks/` taslak ve tema, paylaşılan bileşenler.
- Ayrıntılı katman, işlem modeli, kurtarma ve veri klasörü: [docs/architecture.md](docs/architecture.md).
- `docs/design`: konsept ve tasarım sistemi; `docs/api.md`: yönetim API'si; `docs/packages.md`: paket kökeni ve SHA-256 güncelleme süreci.

## Kullanıcı tarafından yönetilen ayarlar

Ayarlar ekranı Genel, PHP, MySQL, Web sunucusu, Yedek ve aktarım, Sistem, API, Güncellemeler bölümlerine ayrılır. Genel bölümündeki **Görünüm** sistem/açık/koyu temayı seçer. phpMyAdmin, e-posta, PostgreSQL, Redis, GitHub ve tünel **Hizmetler** sayfasında; port ve jeton her hizmetin **Ayarlar** düğmesindedir. Ortamı durdurun, tercihleri düzenleyin, **Ayarları kaydet** ile doğrulatın ve ortamı yeniden başlatın. PHP ayarları ortak veya sürüme özel kaydedilebilir; uzantılar ve ek ini seçenekleri gerçek kurulu PHP ile denetlenir. Açık proje terminallerini yeniden açın.

Yeni proje ve SQL yedek klasörleri seçilebilir. Adres kalıbı değişikliği tüm kayıtlı proje adreslerine uygulanır; `.env` dosyaları korunur. JSON içe aktarma, önceki ayarları getirme ve varsayılanlara dönme önce taslak oluşturur. Tercihlerin önceki sürümü `config/settings.previous.json` içinde saklanır. Onarımda kullanıcı tercihleri korunur.

Üretilen servis dosyalarını elle düzenlemek yerine bu ekranı kullanın. Desteklenen seçenekler, ayrıntılı Laragon karşılaştırması ve henüz uygulanmayan özellikler: [Laragon incelemesi](docs/laragon-incelemesi.md).

Gerçek PHP/MySQL/Caddy ayar testi:

```powershell
cargo test -p serverbond-core --test preferences_runtime -- --ignored --nocapture
```

## Windows masaüstü ve sistem tepsisi

Saat yanındaki ServerBond simgesine sol tıklamak pencereyi açar; sağ tıklamak hızlı menüyü açar. Simge Windows'un gizli simgeler bölümünde olabilir. Menüde tüm servisleri başlat/durdur/yeniden başlat, ayrı PHP/MySQL/web servisleri, phpMyAdmin, Ayarlar / Özellikler, **Güncellemeleri denetle**, günlükler, veri klasörü ve Çıkış bulunur. Menü durumu pencere gizliyken de güncellenir. Ana penceredeki **Hızlı menü** aynı Windows menüsünü açar.

**Ayarlar → Genel → Windows ve sistem tepsisi** altında üç tercih vardır:

- Windows oturumu açıldığında ServerBond'ı çalıştır: yalnızca mevcut Windows kullanıcısının başlangıç kaydını yönetir. İlk kurulumda kapalıdır; yönetici yetkisi istemez.
- Windows başlangıcında tepside çalıştır: otomatik açılışta pencereyi gizler. Normal kısayolla açılış her zaman pencereyi gösterir; tepsi veya yapılandırma hatasında pencere gizlenmez.
- Pencereyi kapatınca tepsiye küçült: varsayılan olarak açıktır; X düğmesi servisleri çalışır bırakır. Bu seçenek kapalıysa X düğmesi servisleri durdurup çıkar. Menüdeki **Çıkış — servisleri durdur** ve penceredeki **ServerBond'tan çık** her zaman tam çıkış içindir.

Bu tercihler servisler çalışırken de kaydedilebilir. PHP/MySQL/Caddy'nin ServerBond açıldığında başlaması, ayrı **ServerBond açıldığında ortamı otomatik başlat** seçeneğine bağlıdır. Tam otomatik ortam için hem Windows başlangıcını hem ortam başlangıcını açın. Kısayola ikinci kez tıklamak mevcut pencereyi öne getirir; ikinci bir servis grubu başlatmaz.

Masaüstü tercihleri `config/desktop.json` içinde, önceki dosya `config/desktop.previous.json` içinde tutulur. Windows başlangıç kaydı işletim sisteminden okunur; uygulama açılırken kullanıcı izni olmadan yeniden etkinleştirilmez. Bu cihaz tercihleri PHP/sunucu ayarlarının JSON aktarımına dahil değildir. Boşluklu/Türkçe yollar tırnaklanır; Windows başlangıç komutunun 260 karakter sınırı denetlenir. Programın konumunu değiştirdiğinizde başlangıç seçeneğini kapatıp yeniden açın; kaldırmadan önce otomatik başlangıcı kapatın.

Teknik kaynaklar: [Tauri sistem tepsisi](https://v2.tauri.app/learn/system-tray/), [tek uygulama örneği](https://v2.tauri.app/plugin/single-instance/), [Windows kullanıcı başlangıç kayıtları](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys).
