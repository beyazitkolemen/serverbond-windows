# F4Box tasarım sistemi

Türkçe Windows masaüstü geliştirme ortamı. React ve mevcut CSS altyapısı korunur; tüm durumlar Rust çekirdeğinden gelir.

## Görsel dil

- Zemin `#f7f9f8`, kenar çubuğu `#f1f5f2`, içerik yüzeyi beyaz, çizgiler `#e1e8e4`.
- Ana renk yeşil `#157557`; güçlü metin `#23312d`, ikincil metin `#687870`. Hata ve uyarılar ayrı anlam renkleri kullanır.
- Windows yazı ailesi: Segoe UI Variable → Segoe UI → system-ui. Sayfa başlıkları 34/650, bölüm başlıkları 18/650, kontroller 13/600. Sürümler ve ölçümler sabit genişlikli veya tabular rakamlarla gösterilir.
- Kenar çubuğu 224 piksel, içerik en fazla 1400 piksel. Sayfa yolu ve masaüstü işlemleri üst araç çubuğundadır.
- Kontrollerde 8, tablolarda 12, durum panelinde 14 piksel köşe yarıçapı. Gölge küçük simge ve seçili sekme gibi hiyerarşi gereken yerlerde kullanılır.

## Sayfalar

- **Genel bakış:** gerçek servis durumu, etkin servis sayısı, varsayılan PHP ve proje sayısı; PHP seçimi; bileşen tablosu; projeler; son günlükler.
- **Bileşenler:** aynı servis tablosu, ek lisans/PID bilgisi ve onarım işlemleri.
- **Projeler:** proje adresi ve klasörü, projeye özel PHP, terminal, veritabanı, yedek, SQL geri yükleme, klasör taraması, kapalı/açılır kuyruk-zamanlama paneli ve satır numaralı günlük görüntüleyici (PHP, zamanlayıcı, işçiler; arama ve hata vurgusu). Boş durumda mevcut/yeni proje akışı açıklanır.
- **Ayarlar:** yapışkan bölüm sekmeleri, gruplandırılmış formlar ve altta sabit kaydetme çubuğu. Windows tercihleri ve güncellemeler ayrı kaydedilir.
- **Günlükler:** kaynak seçimi ve açık renkli, sabit genişlikli kayıt alanı.

## Davranış ve erişilebilirlik

Checkbox temelli görsel anahtarlar klavye ve etiket ilişkilerini korur. Görünür odak, içeriğe geç bağlantısı, native dialog/Escape ve reduced-motion desteklenir. Mevcut yüklenme, işlem kilidi ve hata kontrolleri korunur.

850 piksel altında kenar çubuğu simgelere daralır. 540 piksel altında özet ve formlar yeniden yerleşir; geniş tablolar kendi alanında kayar. Masaüstü pencerenin minimum boyutu 900 × 650'dir; dar görünüm ayrıca tarayıcıda doğrulanır.

`concept.png` ilk tasarımın tarihsel konseptidir. Güncel gerçek uygulama görüntüleri [ekran görüntüleri](../screenshots/README.md) bölümündedir. Ekran görüntüleri uygulama arayüzünün yerine kullanılmaz; örnek servis durumları veya sahte ölçümler üretilmez.
