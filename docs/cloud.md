# ServerBond Cloud bağlantısı

Ayarlar → Cloud bölümünde Cloud adresi ve panelden aldığınız kod ile eşleştirin. Uygulama açıkken servislerinizi uzaktan yönetebilirsiniz; tepside çalışması yeterlidir.

Bağlantı için yerel API'yi açmanız veya API jetonunu paylaşmanız gerekmez. Cihaz anahtarı config/cloud.dpapi içinde kullanıcıya bağlı Windows DPAPI ile korunur. Cloud çalışmazsa yerel servisler çalışmaya devam eder.

Yerel geliştirme: Cloud projesinde HTTP sunucusunu 127.0.0.1:19876, Reverb'i 127.0.0.1:18081 üzerinde başlatın. Windows debug derlemesini SERVERBOND_CLOUD_ALLOW_HTTP=1 ortam değişkeniyle çalıştırın ve Cloud adresini http://127.0.0.1:19876 girin. Release derlemesi yalnızca HTTPS/WSS kabul eder.

# Cloud protokolü v1

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
