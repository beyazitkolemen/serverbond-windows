# NSIS şablonu

`installer.nsi`, Tauri CLI **2.11.4** sürümünün resmi NSIS şablonudur:
https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi

MIT lisansı `TAURI-LICENSE-MIT` dosyasındadır. Tauri CLI yükseltilirken şablon da karşılaştırılmalıdır.

Yerel fark: `currentUser` için yeni kurulum varsayılanı `C:\ServerBond` olur. `/D=` ile verilen yol ve kayıtlı önceki kurulum konumu Tauri'nin mevcut seçim akışıyla korunur. WebView2, güncelleme, kısayol ve kaldırma akışı değiştirilmez. Kaldırıcı `$INSTDIR` için yalnızca boş klasörü kaldırmayı dener; kullanıcı tarafından oluşturulan `www`, `data` ve `backups` dizinlerini özyinelemeli silmez.

Veri dizini çekirdekte `product::default_home` tarafından seçilir. Eski AppData verileri otomatik taşınmaz. Doğrulama: `./scripts/ai/test-installer-paths.ps1`.

Ayrıntılar: [adlandırma ve uyumluluk](../../docs/naming.md).
