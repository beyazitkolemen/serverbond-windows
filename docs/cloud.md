# ServerBond Cloud bağlantısı

Cloud panelinde cihaz ekleyip bağlantı kodunu kopyalayın. Windows uygulamasında Ayarlar → Cloud bölümüne yalnızca bu kodu yapıştırıp **Cloud’a bağlan** düğmesine basın. Hesap, cihaz ve soket ayarları otomatik alınır; adres, port veya soket anahtarı girmeniz gerekmez. Uygulama açıkken servislerinizi uzaktan yönetebilirsiniz; tepside çalışması yeterlidir.

Bağlantı için yerel API'yi açmanız veya API jetonunu paylaşmanız gerekmez. Cihaz anahtarı config/cloud.dpapi içinde kullanıcıya bağlı Windows DPAPI ile korunur. Cloud çalışmazsa yerel servisler çalışmaya devam eder.

Yerel geliştirme: Cloud projesinde HTTP sunucusunu 127.0.0.1:19876, Reverb'i 127.0.0.1:18081 üzerinde başlatın. Windows debug derlemesini SERVERBOND_CLOUD_ALLOW_HTTP=1 ortam değişkeniyle çalıştırın ve Cloud adresini http://127.0.0.1:19876 girin. Release derlemesi yalnızca HTTPS/WSS kabul eder.

# Cloud protokolü v1

GitHub cihaz akışı: `github.auth-start` açık onayla Windows cihaz akışını başlatır ve flowId/userCode/verificationUri/expiresAt/interval döndürür. `github.auth-poll` flowId ile sorgular; flowId/status/retryAfter/login döndürür. `github.auth-cancel` flowId ve açık onayla iptal eder. Erişim/yenileme jetonları ve OAuth device code Windows içinde kalır. Native bekleme/slow-down kuralları korunur. Cloud geçersiz veya süresi dolmuş kullanıcı kodlarını gizler; gerçek GitHub hesabıyla uçtan uca doğrulama henüz yapılmadı.

GitHub hesap yönetimi: `github.show` yerel bağlantı meta verisini döndürür; `github.client` Client ID ayarını kaydeder veya boş değerle temizler; `github.disconnect` bekleyen girişi ve yerel hesap jetonlarını kaldırır. Değişikliklerde `confirm:true` zorunludur. Sonuç yalnızca tokenSaved/login/clientId/authMethod/expiresAt içerir; erişim/yenileme jetonları gönderilmez. `SMOKE_GITHUB=1` test Client ID kaydı, yeniden okuma ve bağlantısız cihazda kaldırmayı gerçek Reverb üzerinden doğrular. OAuth cihaz koduyla giriş ve bağlı hesap kaldırma bu Cloud testinde henüz doğrulanmadı. Cloud yeni işlem adlarını kabul eden sürüme önce güncellenmelidir.

Tünel: `tunnel.show` sır içermeyen durum döndürür; `tunnel.token` jeton kaydeder, `tunnel.apply` jetonu kaydedip gerekiyorsa kurar ve başlatır, `tunnel.clear` bağlantıyı durdurup yerel jetonu siler, `tunnel.auto-start` açılış tercihini kaydeder. Değişiklikler `confirm:true` gerektirir. Sonuç yalnızca version/installed/repairable/running/tokenSaved/autoStart/hasIssue içerir; jeton ve ham hata metni dönmez. Jeton parametresi Cloud veritabanında şifreli, Windows üzerinde DPAPI korumalıdır. Önce Cloud yeni işlem adlarını kabul edecek sürüme güncellenmelidir. `SMOKE_TUNNEL=1` gerçek Reverb üzerinden test jetonuyla kayıt/okuma/otomatik başlatma tercihi/silme akışını doğrular; gerçek Cloudflare bağlantısı ve alan adı erişimini doğrulamaz.

Ayar paneli için çekirdek hazırlığı: `settings_with_revision` aynı anlık ayar kopyasıyla SHA-256 revizyonunu döndürür. `save_settings_checked` yerel kayıtla aynı Manager kilidi altında revizyonu karşılaştırıp mevcut doğrulama/kayıt akışını çağırır. Eski revizyon `SettingsConflict` üretir; config ve önceki ayar yedeği değişmez. Bu yöntemler `settings.show/save` üzerinden Cloud paneline bağlıdır. `settings.validate` yalnızca model doğrulaması yapar; servis/port/dosya sistemi uygunluğu kayıtta denetlenir. `settings.defaults/previous` ayar adayını döndürür, kaydetmez. Cloud paneli bu adayı düzenlenebilir taslağa açar; cihaz/işlem sahipliğini denetler ve kayıtta güncel ayar revizyonuyla ayrı onay ister. `SMOKE_SETTINGS=1` gerçek Reverb testinde açılış tercihi kaydı ve yeniden okumayı doğrular. `cargo test -p serverbond-core --test settings_revision --test preferences` yerel değişikliğin korunması, eşzamanlı iki kayıttan yalnızca birinin kabulü, geçersiz girdinin reddi ve yeniden açılışta kalıcılığı doğrular.

MySQL sorguları komut satırı argümanında taşınmaz; boyut ve süre sınırları korunan standart girdi kullanılır. MySQL parola değişimi önce yeni sırrı `mysql-password.dpapi.next` dosyasına şifreleyip geri okuyarak doğrular, sonra sunucuyu değiştirir ve şifreli dosyayı yerleştirir. Son yerleştirme başarısızsa `.next` korunur ve hata döner; bu sunucu/dosya işlemlerini tek atomik işlem yapmaz. Gerçek izole MySQL testi şifreli kayıt hazırlığı hatasında eski erişimi, başarılı değişiklik sonrası yeni erişimi ve SQL yedek/geri yüklemeyi doğrular: `cargo test -p serverbond-core --test mysql_credentials -- --ignored`.

Cihaz seviyesinde `mysql.connection` parametresiz olarak Windows loopback adresi, port ve root kullanıcı adını verir. `mysql.credentials` açık `confirm: true` ile mevcut parolayı alır. `mysql.password`, `password` ve `confirm: true` ile aynı Manager parola değiştirme akışını kullanır; sonuç yalnızca `changed: true` içerir. Parola 8–128 ASCII karakterdir; boşluk, tırnak, #, ; ve ters eğik çizgi kabul edilmez. Cloud tekrar alanını doğrular ve Windows'a göndermez. Parola parametre/sonuçları şifreli saklanır; Reverb yalnızca bildirim taşır. Panelde alınan parola iki dakikalık pencere içinde gösterilir ve yeni değişiklik isteğinde gizlenir; bu görsel süre kısıtı şifreli işlem kaydını silmez. PostgreSQL için `postgres.connection`, `postgres.credentials` ve `postgres.password` aynı sözleşmeyi kullanır; kullanıcı adı `postgres` olur. Motorların sonuç ve geçmişleri ayrı tutulur. PostgreSQL sorguları da SQL içeriğini argümanlar yerine standart girdiden alır; parola değiştirme hatası sır içermeyen sabit mesajla döner. Gerçek izole PostgreSQL testi eski parola reddini, yeni parola ile Türkçe sorguyu ve şifreli kayıt hazırlığı hatasında eski erişimin korunmasını doğrular: `cargo test -p serverbond-core --lib postgres_rotation -- --ignored`. `SMOKE_POSTGRES=1` Cloud testi kurulum, başlatma, bağlantı/parola okuma, değişiklik ve yeni parolayla yeniden başlatmayı gerçek Reverb üzerinden doğrular.

Proje veritabanı işlemleri: `database.show/create/backup` yalnızca proje `id` alanını alır. Hedef, kayıtlı proje adından (`-` → `_`) hesaplanır; Cloud serbest veritabanı adı gönderemez. `show` hedef adını verir, veritabanının mevcut olduğunu iddia etmez. `database.restore` ayrıca Windows'taki mutlak `.sql` dosya yolunu ve `confirm: true` ister; ortak Manager 512 MB sınırını ve MySQL çalışma durumunu denetler. Sonuç `projectId/name/database/action/path` içerir; yalnızca başarılı yedekte `path` doludur. Yedek dosyası cihazda kalır. `.env` değiştirilmez. Önce yeni operation adlarını kabul eden Cloud sürümü yayınlanmalıdır.

Ortam dosyası işlemleri: `env.read` proje UUID'siyle `.env` ve varsa `.env.example` içeriğini alır. `env.write`, `id`, `contentBase64`, `expectedRevision` ve `confirm: true` ister. Dosyalar ayrı ayrı 256 KB UTF-8 ile sınırlıdır; NUL reddedilir. İçerik base64 taşıma kodlaması kullanır, şifreleme HTTPS ve DPAPI katmanlarındadır. Sonuç `projectId`, `exists`, `revision`, `contentBase64`, `exampleBase64` alanlarını içerir. Revizyon SHA-256(varlık baytı + içerik) olarak hesaplanır; eksik dosya ile boş dosya farklıdır. Kayıt Manager kilidi altında revizyonu denetler ve atomik yazılır. Harici editörlere dosya kilidi uygulanmaz. Çakışma sabit, sırsız bir hata mesajıyla döner. Normal 32 KB parametre / 256 KB sonuç sınırları korunur; yalnızca `env.write` parametreleri 384 KB, iki ortam işleminin sonuçları 768 KB olabilir. Poll yanıt sınırı 512 KB'dir. Cloud'u bu iki yeteneği kabul eden sürüme önce güncelleyin.

## Taşıma

Windows yalnızca dışarı doğru HTTPS ve Reverb WSS bağlantısı açar. Yerel yönetim API'si bu protokolün parçası değildir. Debug derlemede açık SERVERBOND_CLOUD_ALLOW_HTTP=1 ile loopback HTTP/WS kullanılabilir. Sertifika doğrulaması kapatılamaz; HTTP yönlendirmeleri izlenmez.

Tüm cihaz HTTP uçları /api/agent/v1 altındadır. pair dışında Authorization: Bearer cihaz-anahtarı zorunludur. JSON gövdeleri kullanılır; anahtarlar URL'ye yazılmaz.

| POST yolu | Girdi | Çıktı |
| --- | --- | --- |
| /pair | code: 32 büyük hex, key: 64 küçük hex | device_id, name, account |
| /socket | {} | key (Reverb public key), host, port (sayı), scheme, channel |
| /authorize | socket_id, channel_name | Pusher özel kanal auth imzası |
| /heartbeat | services: [{id, running}] | command: null |
| /poll | services: [{id, running}] | command: null veya {id, service, action, expires_at} |
| /commands/{id}/result | status | ok: true |
| /revoke | {} | ok: true |

Eşleştirmeden önce Windows anahtarı DPAPI ile yazar. Kodun geçerlilik süresi içinde aynı kod/anahtar ile tekrar istek aynı cihaza döner; farklı anahtar reddedilir. Bu, ağda eşleştirme yanıtı kaybolduğunda kurtarma içindir.

Cihaz kendi private-devices.UUID kanalını dinler. Pusher protokol v7, bağlantı/abonelik olayları, ping/pong ve yeniden bağlanma desteklenir. command.ready komut sorgusunu tetikler; connection.revoked bağlantıyı kapatır. Sadece bildirimler bu kanala gider, anahtar veya servis sırları gönderilmez.

Tarayıcı Laravel oturumu ve CSRF ile /broadcasting/auth üzerinden private-customers.ID kanalına bağlanır. device.changed olayı ilgili panel bölümünü yeniler. Cihaz anahtarı müşteri kanalına erişemez; kullanıcı oturumu cihaz kanalını yetkilendiremez. Reverb client-* olayları kabul edilmez.

## İşlem kuralları

Servisler: all, php, mysql, caddy, mail, postgres, redis, tunnel, composer, phpmyadmin, node.
Eylemler: install, repair, start, stop, restart. Her iki taraf da servis/eylem eşleşmesini doğrular.

Servis raporu id/running yanında desteklenen actions, varsa installed ve version içerir. Cloud yalnızca kendi izin listesiyle cihazın eylemlerinin kesişimini kabul eder. Eski istemciler actions göndermediğinde yalnızca start/stop/restart kullanılır. Tam snapshot, proje yolları ve sırlar rapora dahil edilmez. Önce 11 servisli raporu kabul eden Cloud sürümünü, sonra Windows güncellemesini yayınlayın.

Cloud durumları: pending → delivered → succeeded / failed / uncertain / expired.
Teslim edilmemiş pending işlemi 60 saniye sonra expired olur. Bağlantı iptalinde pending → cancelled, delivered → uncertain. İptal, zaten başlamış işlemi geri almaz.

poll yalnızca Reverb bildirimi, ilk abonelik/yeniden bağlantı ve yerel işlem tamamlanması üzerine çağrılır; 10 saniyelik heartbeat komut teslim etmez. Veritabanı işlemi cihaz kaydını kilitler; aynı cihazda iki aktif komuta izin verilmez.

Windows kaydı: config/cloud-commands-UUID.json; komut çalışmadan önce running diske atomik yazılır. Sonuç yeniden gönderilebilir. Aynı kimlik ikinci kez yürütülmez. Açılışta running kaydı bulunduğunda işlem uncertain olarak raporlanır. Bozuk veya dolu günlükte yeni işlem çalıştırılmaz; dosya sessizce sıfırlanmaz.

Bağlantı arızasında 10, 20, 40, en fazla 60 saniye beklenir. 401 erişimi durdurur; tekrar eşleştirme gerekir. Mevcut yerel servisler Cloud arızası nedeniyle durdurulmaz.

## Yerel doğrulama

Parametreli proje yönetimi: Yeni istemci operations listesini bildirir. Cloud'daki Projeler sekmesi projects.list/add/remove/create/import/github/discover/import-folders komutlarını gönderir. Aynı yerel Manager doğrulamaları ve dosya koruma kuralları geçerlidir. Parametreler 32 KB, yapılandırılmış sonuç 256 KB ile sınırlıdır; liste ve keşif 50 kayıtlık sayfalar kullanır. Cloud HTTP yanıtı, Unicode JSON kaçış payıyla 256 KB sınırındadır.

Sonuç `config/cloud-output-CIHAZ-KOMUT.dpapi` içinde şifreli saklanır; sonuç kalıcı yazılmadan komut tamamlandı sayılmaz. Tekrar teslimde kaydedilmiş sonuç gönderilir. Kayıp/bozuk çıktı veya yarım işlem uncertain olur ve yeniden çalıştırılmaz. Cloud da parametre/sonuç sütunlarını şifreler. Reverb yalnızca bildirim taşır; içerik HTTPS ile gider.

Proje ayrıntısı ve dağıtım: `projects.show` Git durumu, tarif ve son beş sürüm kaydını döndürür. `projects.release` tarif kaydeder; `projects.deploy` ayrıca `confirm:true` gerektirir. Her iki mutasyon `expectedRevision` ile mevcut tarifin SHA-256 özetini Manager kilidi altında karşılaştırır. Eski ekran tarifi ezemez veya dağıtımı başlatamaz. Çıktı kaydı başına ilk 8.000 karakter Cloud'a gönderilir; tam kayıt yerelde kalır. Cloud migration ve kodunu Windows istemcisinden önce yayınlayın.

PHP yönetimi: `php.list/select/repair` varsayılan sürümü; `projects.php` ve `projects.php-repair` proje sürümünü yönetir. Parametrelerde sürüm ve proje işlemlerinde UUID bulunur. Aynı Manager kurulum, SHA-256 doğrulaması, yeniden başlatma/geri alma ve çalışan sunucuda onarım reddi uygulanır. Sonuç yalnızca sürüm durumlarını içerir; tam ayarlar veya paket adresleri gönderilmez. Cloud'u yeni yetenekleri kabul edecek sürüme önce güncelleyin.

Kuyruk yönetimi: `jobs.show/save/worker/schedule` aynı yerel Manager ayar ve süreç yönetimini kullanır. Ayar kaydı `expectedRevision` ile kilit altında kontrol edilir; eski Cloud taslağı yerel değişiklikleri ezemez. Başlatma/durdurma/yeniden başlatma hedef projeye ait işçi UUID'siyle yapılır. Sonuçta yapılandırma ve süreç sayıları bulunur; ham hata metni veya log gönderilmez.

İsteğe bağlı çıktı işlemleri: `jobs.failed/tasks` liste alır; `jobs.retry` iş kimliği veya `all` ve açık onayla yeniden kuyruğa alır; `jobs.flush` açık onayla başarısız kayıtları siler. `projects.log` yalnızca projenin PHP/zamanlayıcı/işçi günlüklerini okuyabilir. Günlüklerin son 32.000 karakteri, diğer çıktıların ilk 32.000 karakteri şifreli sonuç olarak saklanıp HTTPS ile iletilir; Reverb bildirimine girmez. Proje dışı işçi ve genel dosya yolu reddedilir.

Kuyruk bağlantısı `default` olduğunda `queue:work` komutuna bağlantı adı verilmez; Laravel'in `queue.default` / `QUEUE_CONNECTION` ayarı kullanılır. `redis` veya `database` gibi açık bağlantı adları aynen iletilir. Bu davranış yerel uygulama, API ve Cloud için ortaktır.

PHPUnit müşteri izolasyonu, oturum/CSRF, eşleştirme tekrarları/süre, hız sınırları, komut sahipliği/süre/tek aktif işlem ve özel Reverb kanal yetkilerini kapsar. Windows testleri DPAPI, URL sınırları, yönlendirme reddi, kalıcı tekrar engeli ve yarım kalan komutları kapsar. İsteğe bağlı cloud_live testi gerçek Laravel/Reverb ile izole Windows Manager çalıştırır.
`settings.show/save` sonuçları ayrıca sabit PHP kataloğundaki `versions` listesini taşır. Cloud bu sürümlerden profil ekler; mevcut varsayılan taslak kopyalanır ve profil kaldırma dahil tüm değişiklikler normal revizyon/onay kontrollü kayıtla uygulanır. Önce bu alanı kabul eden Cloud sürümünü yayınlayın. `SMOKE_SETTINGS=1` profil ekleme, farklı bellek değerini kalıcı kaydetme, tekrar okuma, canlı yenilemede taslak koruma ve kaldırmayı doğrular.
Ayar komutları (`settings.save/validate`) için parametre sınırı 256 KB UTF-8 JSON değeridir; diğer normal komutlar 32 KB ile sınırlı kalır. Cloud poll yanıtını Unicode kaçışları olmadan üretir; Windows tarafındaki 512 KB poll sınırı korunur. Gerçek `SMOKE_SETTINGS=1` turu 32 KB üzeri Türkçe PHP profillerinin doğrulama, kayıt ve kayıpsız geri okunmasını kapsar. Boyut sınırını aşan ayarlar yürütülmeden reddedilir.

## Varsayılan sunucu ve bağlantı durumu (v1.3.2)

Ayarlar → Cloud ekranı yeni kurulumda `https://serverbond.on-forge.com` adresini hazır getirir. Kullanıcı yalnızca panelden aldığı bağlantı koduyla cihazını bir kez eşleştirir. Kayıtlı özel sunucu adresleri korunur; adres düzenlenebilir ve varsayılan sunucuya tek tuşla dönülebilir. Çekirdek eşleştirme çağrısında boş adres de aynı Forge sunucusuna karşılık gelir.

WebSocket hostu, portu, uygulama anahtarı ve özel cihaz kanalı Cloud API üzerinden otomatik alınır. Kullanıcının `/sockets` sayfa adresini veya bir Reverb anahtarını girmesi gerekmez. Windows başlangıcında kaydedilmiş eşleştirme kullanılır; ağ kesintilerinde mevcut artan bekleme süresiyle yeniden bağlanılır.

Ekran eşleştirme, sokete bağlanma, canlı abonelik, yeniden deneme ve erişim iptali durumlarını ayrı gösterir. “Soket bağlı” durumu yalnızca özel kanal aboneliği ve ilk Cloud iletişimi başarılı olduktan sonra görünür. Sunucu, anahtar içermeyen soket adresi ve son başarılı iletişim görüntülenir. Bağlantı kodu veya cihaz anahtarı durum yanıtında bulunmaz.

## Tek kodla bağlantı (v1.3.4)

Varsayılan Forge sunucusu için IPC çağrısında da adres gerekmez. Özel Cloud sunucuları gelişmiş bağlantı ayarlarından seçilebilir; kayıtlı özel adresler korunur ve kod gönderilmeden önce ekranda gösterilir. Kopyalanmış koddaki boşluk ve tireler temizlenir. Eşleştirme tamamlanınca bağlantı döngüsü beklemeden uyandırılır. Başarılı eşleştirme ile canlı Cloud bağlantısı ayrı durumlar olarak gösterilir; yinelenen gönderimler ve eski durum sorgularının yeni sonucu ezmesi engellenir.
