# Windows uyumluluğu

Bu belge v1.2.0 ile gelen uyumluluk hazırlığını tanımlar. Aşağıdaki hedefler, eski Windows üzerinde tamamlanmış uçtan uca test garantisi değildir.

Güncel `main` dalındaki dosya kilidi, süreç ağacı, kuyruk ve kurulum iyileştirmeleri: [Windows dayanıklılığı](windows-reliability.md).

## Hedefler ve sınırlar

| Sistem (x64) | Kapsam |
| --- | --- |
| Windows 10 22H2 / LTSC 2019 ve sonrası | Geriye uyumluluk hedefi. Temiz makinede kurulum ve tüm servis testleri henüz yapılmadı. |
| Windows Server 2019 / 2022, Desktop Experience | Geriye uyumluluk hedefi. Server Core masaüstü arayüzü kapsamına girmez. |
| Windows 11 / Server 2025, Desktop Experience | Güncel hedefler. |
| Windows 10'un erken sürümleri / Server 2016 | Rust/Go temel sürüm koşulunu sağlar. Her isteğe bağlı bileşen için destek iddiası yoktur; ayrı doğrulama gerekir. |
| Windows 7 / 8 / 8.1 / Server 2008 R2 / 2012 / 2012 R2 | Desteklenmez. Güncel Rust ve Go tabanlı bileşenler NT 10.0 gerektirir. |
| 32-bit Windows, ARM64 yerel paket | Dağıtım hedefi değildir. ARM64 üzerinde x64 emülasyonu doğrulanmadı. |

Uyumluluk modu eski Windows desteği sağlamaz. Eski PHP seçimi de ServerBond, Caddy, MySQL ve WebView2'nin işletim sistemi gereksinimlerini düşürmez.

## Uygulanan hazırlık

- NSIS paketi WebView2 Evergreen bootstrapper içerir. Runtime eksikse kurulur; kurulum için internet gerekir. Bu, çevrimdışı WebView2 paketi değildir.
- Kurulumun WebView2 alt sınırı `109.0.1518.0`; Vite JavaScript ve CSS hedefi `chrome109`. Evergreen güncellemeleri açık kalır; eski bir runtime sürümüne sabitleme yapılmaz. Doğrudan EXE kullanımında da WebView2 109 veya üzeri gerekir.
- Dinamik pencere yüksekliği için `vh` geri dönüşü vardır. Daha yeni tarayıcılar `dvh` kullanır.
- **Ayarlar → Kurulum gereksinimleri** gerçek Windows NT sürümü ve derleme numarasını `RtlGetVersion` ile gösterir. Sürüm okunamıyorsa veya NT 10.0'dan eskiyse paket kurulumu engellenir. Yeşil sürüm kontrolü yalnızca temel OS koşulunu belirtir.
- NSIS, uygulama dosyalarını kopyalamadan önce NT 10.0 koşulunu kontrol eder. Bu kontrol WebView2 aşamasından sonra çalışır. Desteklenmeyen sistemlerde çıkış kodu 1633'tür; sessiz kurulum da durur. Doğrudan EXE, çok eski Windows'ta bu denetime ulaşmadan yükleyici hatası verebilir.
- Release derlemesi `windows-2022` üzerinde üretilir. CI `windows-2022` ve `windows-2025` üzerinde derleme/test/NSIS üretimi çalıştıracak şekilde yapılandırılmıştır. Bu CI, Windows 10 veya Server 2019 testi sayılmaz.

## Bileşenlerin kapsamı

Uygulamanın açılması tüm servislerin üretici tarafından desteklendiği anlamına gelmez:

- [Rust MSVC](https://doc.rust-lang.org/rustc/platform-support/windows-msvc.html) Windows 10 / Server 2016 tabanını kullanır.
- [Go 1.21 ve sonrası](https://go.dev/wiki/MinimumRequirements) aynı tabanı gerektirir; Caddy, Mailpit ve cloudflared gibi Go tabanlı araçlar ayrıca kendi dağıtım koşullarına sahiptir.
- [WebView2](https://learn.microsoft.com/en-us/microsoft-edge/webview2/) güncel Windows 10 ve Server sürümlerini destekler. [Windows 7/8.1 için son runtime 109'dur](https://blogs.windows.com/msedgedev/2022/12/09/microsoft-edge-and-webview2-ending-support-for-windows-7-and-windows-8-8-1/); bu eski sistemleri ServerBond için desteklenir yapmaz.
- [PHP](https://www.php.net/manual/en/install.windows.manual.php) x64 Visual C++ runtime ister. PHP sürümlerinin OS koşulları farklıdır.
- [MySQL 8.4](https://www.mysql.com/support/supportedplatforms/database.html) güncel üretici tablosunda Server 2016/2019/2022 ve Windows 11 ile listelenir; Windows 10 bu tabloda yoktur. Windows 10 için üretici destek garantisi verilmez.
- [PostgreSQL 17 Windows dağıtımı](https://www.postgresql.org/download/windows/) Server 2019/2022 üzerinde test edilir. Server 2016 için PostgreSQL 17 desteği iddia edilmez.
- [Node.js 24](https://github.com/nodejs/node/blob/v24.x/BUILDING.md) Windows 10 / Server 2016 tabanını kullanır.

Kaynaklar 21 Eylül 2026'da kontrol edildi. Yeni katalog sürümleri bu sınırları değiştirebilir.

## Eski makinede doğrulama

Her hedef sistemde ayrı, temiz bir sanal makine kullanın; var olan üretim veri klasörünü kullanmayın. İşletim sistemi sürümü, EXE SHA-256 ve WebView2 sürümünü sonuçlara ekleyin.

1. WebView2 bulunmayan ve eski WebView2 bulunan kurulumları ayrı deneyin; uygulamanın açıldığını doğrulayın.
2. Gereksinimler ekranında Windows, Visual C++ x64, disk, veri klasörü ve port sonuçlarını kontrol edin.
3. PHP/MySQL/Caddy kurun; sunucuyu başlatın. Laravel HTTP ve HTTPS sayfasını, veritabanı sorgusunu, kuyruk ve zamanlayıcıyı çalıştırın.
4. Kullanılacak isteğe bağlı hizmetleri (Mailpit, PostgreSQL, Redis, Node.js, tünel) ayrıca başlatın ve doğrulayın.
5. Açık/koyu tema, arama, Ctrl+K, dosya seçimi, tepsi, yeniden açılış ve güncelleme akışını kontrol edin.

Eski sistemlerde gerçek kurulum, servis başlatma ve güncelleme adımları henüz doğrulanmadı. Sürüm karşılaştırma birim testleri bu testlerin yerine geçmez.

Bu değişiklik Windows 11 Pro x64 (NT 10.0, derleme 26200) üzerinde doğrulandı: arayüz derlemesi, Rust biçim/Clippy kontrolü, 131 workspace testi ve imzasız NSIS paket üretimi başarılı. Gerçek servis indiren dört test varsayılan olarak atlandı. NSIS çıktısında gömülü bootstrapper, minimum WebView2 sürümü ve kurulum kancası kontrol edildi; paket bu oturumda kurulmadı. Yeni CI matrisi yapılandırıldı, uzak iş sonuçları bu yerel doğrulamanın parçası değildir.
