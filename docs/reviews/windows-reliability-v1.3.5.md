# Windows güvenilirlik incelemesi — v1.3.5

İnceleme arayüz → Tauri IPC → Manager kayıt/çalıştırma akışlarını izledi. Cloud eşleştirme, sunucu/hizmet/API ayarları, kuyruk/zamanlayıcı, sürüm tarifi, .env, Windows başlangıç kaydı ve kapanış kontrolleri okundu. Aşağıdaki somut sorunlar düzeltildi; bu rapor bütün olası hataların giderildiği anlamına gelmez.

| Bulgu | Tetikleyici ve önceki sonuç | Düzeltme / kanıt |
| --- | --- | --- |
| Yüksek: eski ayarlar yeni kaydı eziyordu | Windows formu açıkken Cloud veya API ayarları değiştiriyor; yerel Kaydet eski tam ayar kopyasını yazıyordu. Hizmetler aynı kayıt komutunu kullanıyordu. | İlk okunan değerler formda tutulur; Tauri bunların revizyonuyla aynı Manager kilidi altında karşılaştırmalı kayıt yapar. API formu yalnızca kendi ayar grubunu karşılaştırır. Settings revision ve desktop conflicts testleri. |
| Yüksek: kuyruk/zamanlayıcı ve sürüm tarifi çakışması | Kirli taslak korunmasına rağmen Kaydet güncel işçi, zamanlayıcı veya dağıtım tarifini ezebiliyordu. | Mevcut Cloud karşılaştırmalı kayıt yöntemleri masaüstüne açıldı. Eski kayıtta config değişmez; güncel snapshot ile çakışma görünür, taslak korunur, Vazgeç güncel değerleri alır. Rust ve React testleri. |
| Yüksek: yanlış tarifin çalıştırılması | Kaydet ve Çalıştır arasındaki başka bir işlem tarifi değiştirebiliyordu; dağıtım farklı tarifle başlıyordu. | Çalıştır isteği görülen tarifi de taşır; Manager aynı işlem kilidi altında kontrol ederek dağıtımı başlatır. Eski tarifle denemede hiçbir sürüm kaydı/komutu oluşmadığı test edildi. |
| Yüksek: .env dış değişikliğinin kaybı | Editör açıkken dosya başka bir editörden veya Cloud’dan değiştiriliyor, oluşturuluyor ya da siliniyordu; Kaydet koşulsuz yazıyordu. | İçerik ve dosyanın varlığı birlikte karşılaştırılır. Çakışmada dosya ve yerel taslak korunur. Yeniden yükleme mevcut taslak bırakma diyaloğunu kullanır. Dış oluşturma/düzenleme/silme ve güncel kaydetme testleri. |
| Orta: başlangıç kaydı geri alma hatası | Eski ürün adıyla kayıtlı başlangıç komutu yeni ada taşındıktan sonra desktop.json kaydı başarısız olursa rollback iki adı birebir geri getirmiyordu. Eski approval kaydı yeni komutun durumuna da karışabiliyordu. | Her adın Run ve StartupApproved değeri ayrı saklanıp geri alınır. Eski kayıt temizliği hataları yutulmaz; başarısızlıkta rollback çalışır. Yeni/eski kaydın etkinlik durumu kendi approval değeriyle değerlendirilir. Windows testi yalnızca geçici HKCU test anahtarlarını kullanır. |
| Orta: temiz formda geçici eski değer | Asenkron snapshot geldiğinde eylemler etkinleşirken useEffect henüz form değerlerini taşımamış olabiliyordu. | Temiz taslak, çocuk bileşenler ekrana uygulanmadan güncel kaynakla eşitlenir. Kirli formun ilk okunan değerleri sonraki snapshotlarla değiştirilmez. Cloud varsayılan/özel sunucu ve taslak testleri. |
| Düşük: yalnızca boşluk içeren Artisan taslağından dönememe | Anlamlı komut değişmediği için Vazgeç görünmüyor, fakat gezinme koruması ham taslağı kirli sayıyordu. | Vazgeç ve Kaydet ham taslak durumunu da dikkate alır. Boşluk/satır sonu regresyon testi. |

## Doğrulama

- 23 React testi: mevcut Cloud ve çalışma alanı senaryoları, özgün kayıt değerleri, çakışma, taslak koruma, .env yeniden yükleme ve sürüm çalıştırma sözleşmesi.
- 4 yeni kısa Rust entegrasyon testi + mevcut settings revision testi; gerçek hizmet/indirme gerektirmez.
- TypeScript/Vite, rustfmt ve çekirdek Clippy kontrolleri. Yeni yerel Clippy sürümünün işaretlediği iki eşdeğer GitHub koşulu sadeleştirildi.
- Windows CI: tam standart workspace testleri, masaüstü derlemesi, Clippy, EXE/NSIS ve kurulum yolu/yükseltme kontrolleri. Uzun ignored servis testleri çalıştırılmaz.

## Sınırlar

Gerçek kullanıcı Windows oturumunda Cloud komutu, çalışan PHP/MySQL süreçleri, UAC ve başlangıçta oturum açma bu çalışmada denenmedi. .env karşılaştırması uygulamanın işlem kilidi altında yapılır; bu kilide uymayan harici bir editörün karşılaştırma ile atomik dosya değişimi arasındaki çok kısa aralıkta yazması bütünüyle engellenmez. Yerel HTTP API/CLI sözleşmeleri korunur; yeni zorunlu özgün değerler aynı sürümde gelen Tauri arayüzüne aittir.
