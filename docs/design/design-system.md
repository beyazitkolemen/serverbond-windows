# F4Box tasarım sistemi

Türkçe Windows Laravel üretim paneli. React ve mevcut CSS altyapısı korunur; tüm durumlar Rust çekirdeğinden gelir.

## Görsel dil

Kurumsal ürün yüzeyi: koyu orman yeşili kenar çubuğu, açık çalışma alanı, tek yeşil vurgu. Pazarlama başlığı ve nane yıkanmış kartlar kullanılmaz.

- **Renkler:** zemin `--canvas` `#eef1ef`, yüzey `--surface` `#ffffff`, kenar `--border` `#d5ded9`. Metin `--ink` `#14241c`, ikincil `--muted` `#5d6d65`. Kenar çubuğu `--sidebar` `#10241c`. Vurgu `--accent` `#147a56`. Hata `--danger` `#b42318`, uyarı `--warning` `#b54708`. Bileşenler bu tokenları kullanır; slate veya nane hex bırakılmaz.
- **Yazı:** Segoe UI Variable → Segoe UI → system-ui. Sayfa başlığı 28/650, bölüm 18/650, gövde 14, kontroller 13/600. Sürümler ve ölçümler tabular. Ürün satırı yalnızca kenar çubuğundadır; sayfa kaşı tekrarlanmaz.
- **Izgara:** 4 piksel. Aralıklar `--space-1`…`--space-9` (4–36). Kenar boşluğu dışı 9, 11, 13, 15, 17, 19, 22, 25 kullanılmaz.
- **Yerleşim:** kenar çubuğu `--sidebar-w` 224 (850 altında 76). İçerik, araç çubuğu ve alt bilgi aynı `--content-max` 1400 + `--page-pad` 36 rayındadır (1150 altında 24, 540 altında 16). Kenar çubuğu yatay dolgusu 16; başlık, gezinme ve alt bilgi aynı dikey hatta durur.
- **Kontrol:** birincil yükseklik `--control-h` 36, sıkışık `--control-h-sm` 32. Simge düğmesi 32, ortam düğmesi 36. Girdi ve seçici aynı yükseklikte hizalanır.
- **Şekil:** kontroller 6, kartlar 8 piksel. Gölge yalnızca kart, tablo, modal ve yapışkan çubuklarda (`--shadow-sm` / `--shadow-md`).
- **Durum:** çalışan servis hapı yeşil; hata ve uyarı ayrı anlam renkleri. Odak halkası `--accent`. Sekmeler ve PHP seçici de beyaz yüzey kullanır.

## Sayfalar

- **Genel bakış:** kontrol odası. Ortam özeti (ölçümler ilgili sayfaya gider), sade bileşen tablosu, kompakt proje satırları, son kayıtlar. PHP seçici, kuyruk paneli ve uzun notlar bu sayfada yoktur.
- **Bileşenler:** PHP sürümü, tam servis tablosu, lisans/PID ve onarım.
- **Projeler:** solda proje listesi, sağda sekmeli detay: Özet, Ortam, Zamanlama, Kuyruklar, Sürüm, Günlükler, Veritabanı. Aç ve Terminal başlıkta kalır.
- **Hizmetler:** sol kenar çubuğunda; phpMyAdmin, e-posta, PostgreSQL, Redis, GitHub ve tünel.
- **Ayarlar:** Ortam / Yönetim gruplu sekmeler, kartlı formlar, altta kaydetme çubuğu.
- **Günlükler:** kaynak sekmeli görüntüleyici.

## Davranış ve erişilebilirlik

Başarı bildirimi dört saniyede kapanır; hata kullanıcı kapatana kadar kalır. Çalışan ortamdan çıkış onay ister. Boş proje durumunda tara ve ekle eylemleri görünür.

Checkbox anahtarları klavye ve etiket ilişkisini korur. Görünür odak, içeriğe geç, native dialog/Escape ve reduced-motion desteklenir.

850 piksel altında kenar çubuğu simgelere daralır. 540 altında özet ve formlar yeniden yerleşir. Masaüstü pencere en az 900 × 650.

`concept.png` tarihsel konsepttir. Güncel görüntüler [ekran görüntüleri](../screenshots/README.md) bölümündedir.
