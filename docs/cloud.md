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

Servisler: all, php, mysql, caddy, mail, postgres, redis, tunnel.
Eylemler: start, stop, restart. Her iki taraf da izin listesini doğrular.

Cloud durumları: pending → delivered → succeeded / failed / uncertain / expired.
Teslim edilmemiş pending işlemi 60 saniye sonra expired olur. Bağlantı iptalinde pending → cancelled, delivered → uncertain. İptal, zaten başlamış işlemi geri almaz.

poll yalnızca Reverb bildirimi, ilk abonelik/yeniden bağlantı ve yerel işlem tamamlanması üzerine çağrılır; 10 saniyelik heartbeat komut teslim etmez. Veritabanı işlemi cihaz kaydını kilitler; aynı cihazda iki aktif komuta izin verilmez.

Windows kaydı: config/cloud-commands-UUID.json; komut çalışmadan önce running diske atomik yazılır. Sonuç yeniden gönderilebilir. Aynı kimlik ikinci kez yürütülmez. Açılışta running kaydı bulunduğunda işlem uncertain olarak raporlanır. Bozuk veya dolu günlükte yeni işlem çalıştırılmaz; dosya sessizce sıfırlanmaz.

Bağlantı arızasında 10, 20, 40, en fazla 60 saniye beklenir. 401 erişimi durdurur; tekrar eşleştirme gerekir. Mevcut yerel servisler Cloud arızası nedeniyle durdurulmaz.

## Yerel doğrulama

PHPUnit müşteri izolasyonu, oturum/CSRF, eşleştirme tekrarları/süre, hız sınırları, komut sahipliği/süre/tek aktif işlem ve özel Reverb kanal yetkilerini kapsar. Windows testleri DPAPI, URL sınırları, yönlendirme reddi, kalıcı tekrar engeli ve yarım kalan komutları kapsar. İsteğe bağlı cloud_live testi gerçek Laravel/Reverb ile izole Windows Manager çalıştırır.
