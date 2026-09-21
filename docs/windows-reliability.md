# Windows dayanıklılığı

Bu değişiklikler güncel `main` dalındaki süreç, dosya ve proje yönetimini sağlamlaştırır. Eski Windows sürümleri için destek kapsamı [uyumluluk belgesindedir](windows-compatibility.md); bu çalışma farklı işletim sistemlerinde yapılmış test yerine geçmez.

## Düzeltilen davranışlar

- **Alt süreçler:** zaman aşımı ve kapatmada Windows Job Object içindeki alt süreçler de sonlandırılır. Dosya kilitlerinin çözülmesi için en fazla iki saniye beklenir. Komut çıktısı toplanmadan önce süreç ağacı kapatılır; stdout ve stderr birlikte 8 MB ile sınırlıdır. Bu davranış [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects) mekanizmasını kullanır.
- **Geçici dosya kilitleri:** atomik kayıt, Windows hata kodu 5/32/33 için en fazla beş kez yeniden denenir (toplam 775 ms bekleme). Kalıcı izin veya kilit sorunu hata olarak bildirilir; mevcut dosya silinmez ve geçici kayıt temizlenir.
- **Kuyruk işçileri:** durum sorgusunun planlı işçi yeniden başlatması, diğer yazma/durdurma işlemleriyle aynı işlem kilidini kullanır. Meşgulken planlı çıkış bekletilir; durmuş süreç çalışıyor gösterilmez. Durdurma işlemi, bekleyen yeniden başlatma niyetini de kaldırır.
- **Durum sorgusu:** canlı süreç kimlikleri kısa bir kilit altında kopyalanır. Paket/dosya ve DPAPI kontrolleri süreç kilidi bırakıldıktan sonra yapılır. Projelerin paylaştığı PHP sürümü her durum sorgusunda bir kez doğrulanır; sonuç sonraki sorguya kalıcı olarak önbelleklenmez.
- **Proje taraması:** çakışmada gösterilen `acme-shop` gibi adlar, toplu veya kısmi eklemede ve seçim sırası değiştiğinde korunur. Aynı fiziksel klasör iki kez önerilmez. `vendor`, `node_modules`, `.git` gibi klasörler kendi `public/index.php` dosyaları olsa da proje olarak listelenmez.
- **Arşivler:** tüm dosya yolları yazma başlamadan denetlenir. Windows aygıt adları, geçersiz karakterler, sonda nokta/boşluk, harf büyüklüğünden kaynaklanan dosya çakışmaları ve dosya/klasör çakışmaları reddedilir. Kurallar [Microsoft dosya adlandırma belgesine](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file) dayanır. Paketler ayrıca mevcut SHA-256 doğrulamasından geçer.

## Tekrarlanabilir kontroller

`crates/serverbond-core/tests/windows_resilience/mod.rs` gerçek Windows dosya kilitlerini, PowerShell alt süreçlerini, kuyruk durdurma yarışını, arşiv adlarını ve proje içe aktarmayı kapsar. Dosya ve süreç testleri geçici dizinlerde çalışır; mevcut projelere veya hizmetlere müdahale etmez.

```powershell
cargo test --workspace --locked
npm run check
npm run test:mcp
./scripts/ai/test-installer-paths.ps1
cargo test -p serverbond-core --test environment --test preferences_runtime -- --ignored --nocapture --test-threads=1
npm run desktop:build:unsigned
```

Gerçek ortam testleri resmi paketleri indirir ve Türkçe/boşluk içeren geçici klasörlerde ayrı PHP, MySQL ve Caddy süreçleri çalıştırır. SQL verisi, yedek/geri yükleme, PHP sürümü geçişi, bileşen onarımı, ayarların uygulanması ve `.env` korunması kontrol edilir. Ayrıntılı komut kapsamı [test rehberindedir](ai-environment.md).

## Yerel doğrulama — 21 Eylül 2026

Ortam: Windows 11 Pro x64, derleme 26200.

| Kontrol | Sonuç |
| --- | --- |
| Workspace testleri | 187 geçti; 6 isteğe bağlı ağ/ortam testi standart çalıştırmada atlandı |
| Gerçek `environment` ve `preferences_runtime` testleri | Ayrı çalıştırıldı, ikisi de geçti |
| TypeScript/Vite, rustfmt, workspace Clippy | Geçti |
| Resmî MCP SDK | 103 araç, bağlantı, okuma/yazma ve hata senaryoları geçti |
| NSIS kurulum yolları | Varsayılan kök, mevcut kurulum ve özel yol senaryoları geçti |

Canlı sürüm keşfi, tüm PHP sürümleri matrisi, cloudflared indirme ve terminal/markalama ortam testleri bu turda ayrıca çalıştırılmadı. Windows 10 ve Windows Server üzerinde temiz makine/VM testi yapılmadı.
