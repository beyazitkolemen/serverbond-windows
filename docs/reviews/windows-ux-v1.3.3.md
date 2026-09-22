# Windows UX incelemesi — v1.3.3

İncelenen kapsam: genel bakış ve uygulama kabuğu, proje seçimi, ortam dosyası, sürüm tarifi, kuyruk/zamanlayıcı, günlük kaynakları, sunucu/Windows/Cloud ayarları ve güncelleme yönlendirmesi. React taslak ve async akışları Tauri çağrıları ve ilgili Rust kaydetme/dağıtım sözleşmeleriyle karşılaştırıldı. Veri veya servis kurulum kuralları değiştirilmedi.

## Düzeltilen bulgular

| Etki | Tetikleyici ve önceki davranış | Düzeltme ve kabul kanıtı |
| --- | --- | --- |
| Yüksek | Kirli ortam/sürüm/ayar taslağından başka sayfa veya projeye geçmek taslağı sessizce kapatıyordu. | Ortak NavigationGuard, yalnız kirli alanların etiketleriyle düzenlemeye dönme veya devam etme seçimi sunar. React testi ve tarayıcıda gerçek gezinme/girdi koruması doğrulandı. |
| Yüksek | Cloud temiz kuyruk ayarlarını değiştirdiğinde wasDirty etkisi kaynak eşitlemesinden önce çalışıp formu kirli sanıyordu. | Kuyruk/zamanlayıcı ortak useDraft kullanır. Dış güncelleme, kirli taslak koruması ve son kaynağa reset testi var. |
| Yüksek | Ek Artisan metni ayrı state olduğundan useDraft bu düzenlemeyi göremiyor; uzaktan tarif değişiminde metin silinebiliyordu. | Metin ve tarif tek taslakta; ham satırlar yazarken korunur, sadece kayıtta normalize edilir. Dış değişiklik ve kayıt sonrası temiz durum test edildi. |
| Orta | Sayısal alan blur sırasında geçersiz girdiyi sessizce eski değere çeviriyordu; kuyruk kaydı bir form gönderimi değildi. | Geçersiz değer görünür hata taşır, native form doğrulaması kaydı engeller; Vazgeç alan içi taslağı da sıfırlar. |
| Orta | Kaldırılan günlük kaynağının satırları, yeni varsayılan kaynak okunamayınca ekranda kalıyordu. | Kaynak değişiminde görüntü sıfırlanır; geç gelen eski yanıt dikkate alınmaz. Her iki yol test edildi. |
| Orta | Ortam dosyası hatasında yükleniyor metni sürüyor; sürüm geçmişi okuma hatası hiç sürüm yokmuş gibi gösteriliyordu. | Hata/yükleme/boş durumlar ayrıldı. Başarılı dağıtım sonrası geçmiş yenilenememesi de dağıtımı başarısız göstermez. |
| Orta | Snapshot bağlantı hatası genel bildirim kapatma düğmesine rağmen kaybolmuyor, son verinin eskidiği belirtilmiyordu. | Kalıcı bağlantı uyarısı, son başarılı saat ve yeniden dene eylemi ayrı tutulur. |
| Orta | Güncellemeye yönlendirme sayacı Settings yeniden bağlandığında tekrar tüketilip Genel yerine Güncellemeler açılıyordu. | Ana uygulama yönlendirmeyi tüketir; bölüm içi denetleme isteği ayrı tutulur. |
| Düşük | Kısmi/boş bileşen listesinde every() sunucu kurulmuş sonucuna ulaşabiliyordu. | Başlatma için üç zorunlu bileşenin her birinin gerçekten listede ve kurulu olması aranır. |

## Tasarım

Açık/koyu temada lacivert vurgular, mavi-gri nötr yüzeyler ve tutarlı odak halkaları kullanıldı. Başarılı, uyarılı ve hatalı durumlar kendi anlam renklerini korur. Uzun form taslakları, hata metinleri ve devam etme eylemleri ayrı ve görünürdür. Önizlemede 900 × 650 ve 390 × 844 ölçülerinde yatay taşma yoktu; tarayıcı konsolunda hata görülmedi.

## Doğrulama ve sınır

11 React regresyon testi yaklaşık bir saniyede geçer. TypeScript/Vite, Rust biçim kontrolü ve Windows checks akışının test/Clippy/EXE/NSIS kontrolleri yayın için kullanılır. UI testleri CI akışlarına da eklendi.

Tarayıcı salt okunur önizlemedir; gerçek Windows kullanıcısının DPAPI, autostart, tepsi, Cloud eşleştirmesi ve hizmet/SQL işlemleri bu incelemede canlı işletilmedi. Bunlar için geçmiş CI kanıtları yeni canlı test olarak sunulmaz. Uzun gerçek servis indirme testleri tekrar çalıştırılmadı.
