# Birleşik proje kurulumu

Windows 1.3.8 ile `Manager::setup_preflight` ve `Manager::setup_project`, masaüstü IPC, yerel HTTP/MCP ve Cloud tarafından ortak kullanılır. HTTP yolları `POST /api/v1/projects/setup-check` ve `POST /api/v1/projects/setup` şeklindedir.

İstek sözleşmesi: `requestId` (UUID), `source` (`github`, `git`, `laravel`, `existing`), `name`, `location`, `branch`, `phpVersion`, `installDependencies`, `composer`, `build`. Laravel kaynağı Composer kurulumunu her zaman gerektirir. `build` için `package-lock.json` ve `scripts.build` gerekir. Kaynağın Composer/npm betikleri seçilen proje PHP'si ile çalışır.

Ön kontrol mevcut klasörün başka adla kayıtlı olmasını da reddeder. Yeni projeler çalışma alanında geçici klasörde hazırlanır; başarıdan sonra hedefe taşınır ve kayıt edilir. Kayıt başarısız olursa hazırlanmış hedef klasör korunur. Mevcut klasör üzerinde talep edilen Composer/npm işlemleri yerinde çalışır; bunların dosya değişiklikleri otomatik geri alınmaz.

`config/setup/<requestId>.json` altında istek özeti, durum ve başarılı proje sonucu atomik kaydedilir. Başarılı aynı isteğin tekrarı önceki sonucu verir; farklı parametre veya yarım kalmış istek reddedilir. Proje kaydı ile günlük yazımı ayrı dosyalardır: kayıt sonrasında süreç kesilirse sonraki istek otomatik tekrar çalıştırılmaz. Projeler ekranı ve hedef klasör üzerinden sonuç incelenmelidir.

Kurulum `.env` dosyasını otomatik düzenlemez; kaynak betikleri kullanıcı tarafından seçilen işlemlerin parçasıdır. Yeni kaynak kaydı veritabanı oluşturmaz veya migration çalıştırmaz. Sonraki yayın tarifinde frontend build tercihi saklanır. İşlem çıktısı Windows'taki `logs/setup.log` dosyasına gider; Cloud yalnızca sabit aşama ve hata metinlerini alır.

## Kabul kontrolleri

- Çift tıklama, aynı isteğin tekrar teslimi ve farklı parametrelerle kimlik tekrar kullanımı.
- Kayıtlı klasörün farklı adla eklenmesi ve `.env` korunması.
- PHP seçiminin kurulum boyunca değişmemesi, eski tarifte build varsayılanının kapalı olması.
- Gerçek Windows ortam testinde mevcut kök kurulumu ve aynı isteğin tekrarı; kurulum sonrası PHP/MySQL/web kontrolleri.
- GitHub ağ erişimi, gerçek Composer/npm uygulama kurulumu ve Cloud cihaz turu ayrıca cihaz üzerinde doğrulanmalıdır; birim testleri bunları kanıtlamaz.

## Yol haritasında kalan işler

1. Gerçek uygulama veritabanı bağlama ve bağlantı doğrulaması; ortam dosyası için açık kullanıcı kaydı.
2. Sabit commit SHA ile ayrı sürüm klasöründe yayın, paylaşılan storage/.env, sağlık denetimi, beş sürüm tutma ve dosya geri alma. Veritabanı değişiklikleri otomatik geri alınmayacak.
3. Cloudflare hesap, tünel, DNS ve hostname yaşam döngüsü; proje bazlı isteğe bağlı GitHub webhook yayını ve teslim tekilleştirme.
4. Tek çalışma zamanı sahibi Windows Service, kimliği doğrulanan named pipe istemcisi ve DPAPI veri geçişi.
5. Zamanlanmış MySQL yedeği, yerel/S3 saklama, doğrulanmış geri yükleme, uyarılar ve owner/operator/viewer rolleri.

Bu liste tamamlanmış özellikleri ifade etmez; yukarıdaki beş madde bu sürümün dışında kalır.
