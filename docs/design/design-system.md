# ServerBond tasarım sistemi

Türkçe Windows Laravel üretim paneli. React ve mevcut CSS altyapısı korunur; tüm durumlar Rust çekirdeğinden gelir.

## Görsel dil

Sade çalışma alanı: nötr yüzeyler, ince ayırıcılar, tek yeşil vurgu. Uygulamalar yan yana kartlar yerine her biri ayrı satırda listelenir. Açık temada kenar çubuğu yumuşak gri-yeşildir; koyu temada aynı hiyerarşi nötr koyu yüzeylerle korunur.

- **Renkler:** zemin `--canvas` `#f6f8fa`, yüzey `#ffffff`, kenar çubuğu `#eef2f3`, kenar `--border` `#e4e9ec`. Metin `--ink` `#20282d`, ikincil `--muted` `#626e76`. Vurgu `--accent` `#147a56`. Hata `--danger` `#b42318`, uyarı `--warning` `#b54708`. Bileşenler renkleri tokenlardan alır.
- **Yazı:** Segoe UI Variable → Segoe UI → system-ui. Sayfa başlığı 32/650 (1150 altında 28), bölüm 18/650, gövde 14, kontroller 13/600. Sürümler ve ölçümler tabular. Ürün satırı yalnızca kenar çubuğundadır; sayfa kaşı tekrarlanmaz.
- **Izgara:** 4 piksel. Aralıklar `--space-1`…`--space-9` (4–36). Kenar boşluğu dışı 9, 11, 13, 15, 17, 19, 22, 25 kullanılmaz.
- **Yerleşim:** kenar çubuğu `--sidebar-w` 224 (850 altında 76, 540 altında 60). İçerik, araç çubuğu ve alt bilgi aynı `--content-max` 1400 + `--page-pad` 36 rayındadır (1150 altında 24, 540 altında 16). Kenar çubuğu yatay dolgusu 16; marka, gezinme ve alt bilgi aynı dikey hatta durur.
- **Kenar çubuğu grupları:** Çalışma alanı (Genel bakış, Projeler, Günlükler), Ortam (Bileşenler, Hizmetler), alta sabitlenen Yönetim (Ayarlar). Gruplar kutu yerine boşluk ve küçük başlıklarla ayrılır; seçili satır dolu yeşil yüzey ve açık metinle belirtilir.
- **Kontrol:** birincil yükseklik `--control-h` 40, sıkışık `--control-h-sm` 32. Simge düğmesi 32, ortam düğmesi 40. Girdi ve seçici aynı yükseklikte hizalanır.
- **Şekil:** kontroller 8, liste ve panel dış köşeleri 12 piksel. Liste satırları tek yüzey içinde ince çizgilerle ayrılır. `--shadow-sm` kapalıdır; modal gibi yükseltilmiş katmanlarda `--shadow-md` kullanılır.
- **Durum:** çalışan servis hapı yeşil; hata ve uyarı ayrı anlam renkleri. Odak halkası `--accent`. Sekmeler ve PHP seçici de beyaz yüzey kullanır.
- **Koyu görünüm:** aynı token adları `:root[data-theme="dark"]` altında ikinci bir değer seti alır; bileşenler renk için yalnızca token kullanır (`--console-bg`, `--console-ink`, `--sidebar-edge`, `--on-accent` dahil). Tercih Ayarlar → Genel → Görünüm'de; `system` Windows'u izler.

## Paylaşılan bileşenler

| Bileşen | Kullanım |
| --- | --- |
| `SearchField` | Etiketli, temizlenebilir arama alanı; temizleme sonrası odak girdiye döner. |
| `QuickNavigation` | `Ctrl+K` / `Cmd+K` ile açılan sayfa seçici; yön tuşları, Enter ve Escape. |
| `StatusBadge` | Her durum hapı: `tone` = `running` / `stopped` / `issue` / `warning`. Elle `service-status` + `status-dot` yazılmaz. |
| `Toggle` | Etiketli onay kutusu anahtarı. |
| `NumberField` | Sayı girişi; alan boşaltılabilir, sınır dışı değer üst bileşene iletilmez, `aria-invalid` işaretlenir. |
| `SegmentedControl` | Birbirini dışlayan seçenekler (`radiogroup`). |
| `EmptyState` | Boş liste/pano: simge, başlık, açıklama, eylemler. |
| `ErrorBoundary` | Kökte tam ekran; `scope` verildiğinde sayfa içinde kalan ve `resetKey` değişince toparlanan yerel kutu. |
| `ServiceConsole`, `ServiceRepair`, `LogViewer` | Hizmet denetim şeridi, onarım kartı, günlük görüntüleyici. |

Form taslakları `useDraft` ile tutulur: kullanıcı yazarken anlık görüntü yenilemesi girdiyi silmez, taslak temizken kaynak izlenir. Sayfa bileşenleri `key` ile yeniden bağlanarak sıfırlanmaz.

## Sayfalar

- **Genel bakış:** ortam durumu, ölçümler, bileşen/sürüm/durum/işlem tablosu, kompakt proje satırları ve son kayıtlar. Sayfa alt yazısı, ortam açıklaması, bileşen açıklama sütunu ve proje klasör yolları gösterilmez. Hatalar ve işlem sonuçları görünür kalır.
- **Bileşenler:** PHP sürümü, tam servis tablosu, lisans/PID ve onarım.
- **Projeler:** solda ad, alan adı veya klasör yoluyla aranabilen tek proje listesi, sağda sekmeli detay: Özet, Ortam, Zamanlama, Kuyruklar, Sürüm, Günlükler, Veritabanı. Aç ve Terminal başlıkta kalır.
- **Hizmetler:** her uygulama tam genişlikte bir satır. Üstte Tümü / Etkin filtresi ve arama yer alır. Etkin, çalışan servislerin yanında açık phpMyAdmin erişimini ve bağlı GitHub hesabını da kapsar. Simge/ad, açıklama, durum ve açma oku masaüstünde hizalıdır. 1100 altında açıklama adın altına; 540 altında durum da alta geçer. Satırın tamamı klavyeyle erişilebilir bir düğmedir. Hizmete girince üstte ortak denetim şeridi (durum, Kur / Başlat / Durdur / Aç); port ve jeton **Ayarlar** düğmesindedir. Onarım ayrı karttır.
- **Ayarlar:** Ortam / Yönetim gruplu sekmeler, kartlı formlar, altta kaydetme çubuğu.
- **Günlükler:** kaynak sekmeli görüntüleyici.

## Davranış ve erişilebilirlik

Bilgilendirmeler kısa ve eyleme dönüktür. Sayfa başlıklarının altında tekrarlayan açıklamalar yoktur. Metinler yalnızca kullanıcının kararını veya sonraki adımını etkileyen bilgiyi içerir. Laravel bağlantı örnekleri kapalı başlayan “Laravel bağlantısı” bölümlerinde açılır. Silme/geri yükleme etkileri, yeniden başlatma gereksinimleri, güvenlik izinleri ve hata ayrıntıları kısaltma uğruna kaldırılmaz.

Başarı bildirimi dört saniyede kapanır; hata kullanıcı kapatana kadar kalır. Çalışan ortamdan çıkış onay ister. Boş proje durumunda tara ve ekle eylemleri görünür.

Checkbox anahtarları klavye ve etiket ilişkisini korur. Görünür odak, içeriğe geç, native dialog/Escape ve reduced-motion desteklenir.

Arama Türkçe karakterlerle veya düz klavyeyle aynı sonuçları verir (`Tünel` / `tunel`). Boş sonuçlarda arama/filtre temizleme eylemi görünür. Hızlı gezinme açık bir işlem penceresinin önüne geçmez; kapanınca önceki odağı geri verir.

850 piksel altında kenar çubuğu simgelere daralır. 540 altında özet ve formlar yeniden yerleşir. Masaüstü pencere en az 900 × 650. Tarayıcı önizlemesi 320 piksele kadar yeniden yerleşir; günlük kaynakları ve görünüm seçenekleri dar alanda alt satıra geçer. Araç çubuğundaki sayfa araması 1150 altında simgeye daralır.

`concept.png` tarihsel konsepttir. Güncel görüntüler [ekran görüntüleri](../screenshots/README.md) bölümündedir.
