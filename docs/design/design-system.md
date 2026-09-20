# F4Box tasarım sistemi

Türkçe Windows masaüstü geliştirme ortamı. React ve mevcut CSS altyapısı korunur; tüm durumlar Rust çekirdeğinden gelir.

## Görsel dil

Kurumsal ürün yüzeyi: koyu orman yeşili kenar çubuğu, açık çalışma alanı, tek yeşil vurgu. Pazarlama başlığı ve nane yıkanmış kartlar kullanılmaz.

- **Renkler:** zemin `--canvas` `#eef1ef`, yüzey `--surface` `#ffffff`, kenar `--border` `#d5ded9`. Metin `--ink` `#14241c`, ikincil `--muted` `#5d6d65`. Kenar çubuğu `--sidebar` `#10241c`. Vurgu `--accent` `#147a56`. Hata `--danger` `#b42318`, uyarı `--warning` `#b54708`. Bileşenler bu tokenları kullanır; slate veya nane hex bırakılmaz.
- **Yazı:** Segoe UI Variable → Segoe UI → system-ui. Sayfa başlığı 28/650, bölüm 18/650, gövde 14, kontroller 13/600. Sürümler ve ölçümler tabular. Ürün satırı yalnızca kenar çubuğundadır; sayfa kaşı tekrarlanmaz.
- **Yerleşim:** kenar çubuğu 224 piksel (850 altında 76), içerik en fazla 1400. Üst araç çubuğu çalışma alanının tam genişliğinde beyaz kromdur; iç satır içerikle hizalanır.
- **Şekil:** kontroller 6, kartlar 8 piksel. Gölge yalnızca kart, tablo, modal ve yapışkan çubuklarda (`--shadow-sm` / `--shadow-md`).
- **Durum:** çalışan servis hapı yeşil; hata ve uyarı ayrı anlam renkleri. Odak halkası `--accent`. Sekmeler ve PHP seçici de beyaz yüzey kullanır.

## Sayfalar

- **Genel bakış:** ortam özeti (beyaz kart, ölçümler), bileşen tablosu, projeler, son günlükler.
- **Bileşenler:** aynı servis tablosu, lisans/PID ve onarım.
- **Projeler:** adres ve klasör, PHP, terminal, veritabanı, kuyruk-zamanlama, günlük görüntüleyici.
- **Ayarlar:** yapışkan sekmeler, kartlı formlar, altta kaydetme çubuğu.
- **Günlükler:** kaynak sekmeli görüntüleyici.

## Davranış ve erişilebilirlik

Checkbox anahtarları klavye ve etiket ilişkisini korur. Görünür odak, içeriğe geç, native dialog/Escape ve reduced-motion desteklenir.

850 piksel altında kenar çubuğu simgelere daralır. 540 altında özet ve formlar yeniden yerleşir. Masaüstü pencere en az 900 × 650.

`concept.png` tarihsel konsepttir. Güncel görüntüler [ekran görüntüleri](../screenshots/README.md) bölümündedir.
