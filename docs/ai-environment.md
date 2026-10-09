# Yapay zeka ortamı ve testler

Cloud Agent ve diğer asistanlar Windows masaüstü uygulamasını bu Linux ortamında çalıştırmaz. Çekirdek mantık, arayüz derlemesi ve biçim denetimleri burada koşar.

Git kuralı: yalnızca `main`. **PR oluşturma yok** (`gh pr create`, ManagePullRequest, draft). Özellik dalı açılmaz; diğer dallardaki ve açık PR’lardaki iş `main`'e alınır.

## Ortam

`.cursor/environment.json` her önyüklemede `scripts/ai/install.sh` çalıştırır, ardından salt okunur önizleme için `npm run dev` (`http://127.0.0.1:1420`) açar. Kurulum sonlanır; test veya uzun süren derleme içermez.

`rust-toolchain.toml` `stable` kullanır. Windows x64 masaüstü derlemesi için rustup varsayılanı `x86_64-pc-windows-msvc` olmalıdır; GitHub Windows işi bunu açıkça seçer.

## Testler

| Komut | Nerede | Ne yapar |
| --- | --- | --- |
| `npm run test:ai` | Linux Cloud Agent, yerel Linux/macOS | Vite/TypeScript derlemesi, `serverbond-core` testleri, rustfmt, clippy |
| `npm run test:core` | Her yer | Nextest ile paralel `serverbond-core` birim/entegrasyon testleri |
| `npm run test:core:serial` | Her yer | Çekirdek testlerinin seri karşılaştırması |
| `npm run test:rust:parallel` | Windows ve Linux, Nextest 0.9.146 | Workspace genelinde işlemci sayısına göre en fazla sekiz eşzamanlı Rust testi; `ignored` testler ayrı kalır |
| `npm run test:mcp` | Windows / Linux | Resmi MCP SDK ile geçici Rust API sunucusuna gerçek bağlantı |
| `npm run check` | Windows geliştirme | Arayüz + tüm workspace clippy (Tauri masaüstü dahil) |
| `npm run desktop:build:unsigned` | Windows CI | İmzasız NSIS + EXE (`createUpdaterArtifacts` kapalı) |
| `.github/workflows/release.yml` | GitHub tag `v*` | İmzalı kurulum, `latest.json`, otomatik güncelleme |
| `npm run test:integration` | Windows, ServerBond kapalı | Gerçek PHP/MySQL/Caddy duman testi |
| `cargo test -p serverbond-core --tests -- --ignored` | Windows | İndirme ve servis bütünleşme testleri |

Yeni çekirdek davranışı için `crates/serverbond-core/tests` altına test ekleyin. Masaüstü IPC veya tepsi davranışı Windows CI / `npm run desktop` ile doğrulanır.

`SERVERBOND_TEST_JOBS=4` işçi sayısını değiştirir (1–32). Varsayılan, kullanılabilir mantıksal işlemci sayısı ile sekizin küçüğüdür. Nextest seçenekleri komutun sonuna `--` sonrasında eklenebilir.

## Sınırlar

- `src-tauri` ve NSIS paketleri Windows x64 ister.
- Paket indirme, MySQL ilk kurulum ve phpMyAdmin tarayıcı akışları yok sayılmış (`ignored`) testlerdir; Cloud Agent bunları çalıştırmaz.
- MySQL ve PostgreSQL kurup başlatan testler yalnızca ilgili servis dosyaları değiştiğinde çalışır. Yayın işi önceki etiketle karşılaştırıp yalnızca etkilenen ortam testini seçer; seçim `node --test scripts/ai/select-release-tests.test.mjs` ile doğrulanır. Aynı yayın içindeki ayrı testler doğrulanmış paket arşivlerini `SERVERBOND_TEST_CACHE` üzerinden yeniden kullanır; her testin veritabanı dizini ayrıdır. Kural: `.cursor/rules/long-service-tests.mdc`.
- `SERVERBOND_HOME` veri dizinini taşır; testler geçici dizin kullanır.

## Windows kurulum yolu testi

`./scripts/ai/test-installer-paths.ps1`, NSIS şablonundaki gerçek başlangıç fonksiyonlarını geçici bir test EXE’sinde çalıştırır. Yeni kurulumda `C:\ServerBond`, güncellemede kayıtlı yol ve `/D=` ile özel klasör seçimi doğrulanır. Uygulamayı kurmaz veya çalışan servislere dokunmaz; geçici test dosyaları ve test kayıt anahtarı temizlenir. NSIS konumu gerekirse `-NsisDir` ile verilir.

`./scripts/ai/test-webview-prerequisites.ps1`, gerçek NSIS WebView2 bölümünü taklit edilmiş kayıt ve bootstrapper işlevleriyle sınar. Eksik/eski/güncel çalışma zamanı, sessiz kurulum ve hata senaryolarını kapsar; bilgisayara WebView2 kurmaz.

`./scripts/ai/test-installer-lifecycle.ps1`, güncel Tauri derlemesinin tam NSIS kurucu/kaldırıcı kaynağını benzersiz test kimliğiyle paketleyip gerçekten çalıştırır. Üretim EXE dosyasının kopyalanan SHA-256 özeti, varsayılan ve Türkçe/boşluklu yollar, kısayollar, kayıtlı konuma güncelleme, kilitli EXE hatası ve kaldırma/yeniden kurulum sırasında test `.env`, yapılandırma ve veritabanı dosyalarının korunması denetlenir. Çalışan uygulama kontrolü yalnız testin oluşturduğu zararsız süreçle sınanır. Mevcut ServerBond kayıtları, dosyaları, kısayolları ve çalışma zamanı kayıtlarının önce/sonra aynı kaldığı doğrulanır.

Bu testten önce `npm run desktop:build:unsigned` çalıştırılmalıdır; eski üretilmiş NSIS kaynakları reddedilir. Test mevcut WebView2 gerektirir ve gerçek ServerBond'u başlatmaz. Eksik çalışma zamanı kurulumu, yönetici izni, temiz Windows, MSI geçişi ve arayüzde uygulama verilerini silme seçeneği ayrı izole misafir testleridir. Sonuçlar/loglar D: geçici test klasöründe kalır; CI `-ArtifactsRoot $env:RUNNER_TEMP` kullanır.

## Redis ve Mailpit veri koruma testleri

`cargo test -p serverbond-core --test optional_services -- --ignored --test-threads=1`, Windows üzerinde gerçek Redis ve Mailpit indirip ayrı geçici veri dizinlerinde başlatır. Redis'in son yazılarının durdurma/başlatma, onarım ve yeniden açılışta korunmasını; disk kaydı başarısızsa sürecin açık kalmasını doğrular. Mailpit testi gerçek SMTP mesajını gönderir ve onarım sonrası aynı mesajı API üzerinden okur. `SERVERBOND_TEST_CACHE` doğrulanmış paket arşivlerini paylaşabilir; servis verileri paylaşılmaz.
