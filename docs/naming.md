# ServerBond adlandırması

Yeni kod, paket ve üretilen yapılandırmalarda ServerBond kullanılır:

| Alan                   | Ad                                             |
| ---------------------- | ---------------------------------------------- |
| Ürün                   | `ServerBond`                                   |
| npm paketi             | `serverbond`                                   |
| Rust çekirdeği         | `crates/serverbond-core`, `serverbond_core`    |
| Masaüstü paketi ve EXE | `serverbond-desktop`, `serverbond-desktop.exe` |
| CLI                    | `serverbond`                                   |
| Tauri kimliği          | `com.serverbond.desktop`                       |
| Sistem tepsisi         | `serverbond-tray`                              |
| Veri klasörü           | `%LOCALAPPDATA%\ServerBond`                    |
| Veri yolu değişkeni    | `SERVERBOND_HOME`                              |
| PHP uzantı değişkeni   | `SERVERBOND_PHP_EXT`                           |
| Test indirme önbelleği | `SERVERBOND_TEST_CACHE`                        |
| PHP e-posta göndereni  | `serverbond@localhost`                         |
| GitHub deposu          | `beyazitkolemen/serverbond-windows`            |

## Önceki kurulumlar

`crates/serverbond-core/src/legacy.rs` yalnızca eski kurulumları okuyabilmek için gereken adları içerir: `F4Box` veri/başlangıç kaydı, `F4BOX_HOME`, eski phpMyAdmin adresi ve günlük kimliği. Bu adlar yeni yapılandırmalara yazılmaz. Eski veri klasörü kullanılmaya devam ediyorsa mutlak dosya yolları değiştirilmez; proje dosyaları, SQL verileri ve `.env` taşınmaz.

Yeni veri klasöründe yapılandırma veya veri varsa o kullanılır. Yalnızca kurulum EXE'si bulunan yeni klasör, eski verileri gizlemez. `SERVERBOND_HOME` eski ortam değişkenine göre önceliklidir. Başlangıç ayarı kaydedildiğinde yeni kayıt ServerBond olarak yazılır ve eski ad kaldırılır.

PHP yapılandırması servis/terminal hazırlığında `SERVERBOND_PHP_EXT` ile yeniden üretilir. Güncellemeden önce eski uygulamadan tepsi menüsündeki **Çıkış** ile çıkın; güncelleme sonrasında eski proje terminallerini yeniden açın. Uygulama kimliği değiştiği için WebView tema tercihi yeniden seçilebilir; proje ve masaüstü ayarları veri klasöründe kalır.

Geçmiş sürüm notlarındaki gerçek indirme dosyalarının adları değiştirilmez. Geliştiricinin mevcut depo klasörünün adı uygulama kimliği veya veri yolu değildir; kaynak kod farklı adlandırılmış bir klasörden de derlenebilir.
