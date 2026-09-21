# GitHub üzerinden otomatik güncelleme

ServerBond, yayımlanmış GitHub sürümlerindeki imzalı NSIS paketini `latest.json` ile denetler. Kullanıcı onayı olmadan indirmez veya kurmaz.

## Kullanıcı

1. Masaüstü uygulamasını açın. Yeni sürüm varsa genel bakışta bir bildirim görünür.
2. **Ayarlar → Güncellemeler** veya tepsi menüsündeki **Güncellemeleri denetle** ile GitHub’ı sorun.
3. Sürüm notlarını okuyup **kur ve yeniden başlat** deyin. Çalışan PHP/MySQL/Caddy durur; kurulum bitince ServerBond yeniden açılır.

Kaynak: `https://github.com/beyazitkolemen/serverbond-windows/releases/latest/download/latest.json`

**v1.1.0 ve v1.1.1:** Paketler Windows'ta yerel olarak derlenip yayımlandı. Güncelleme imza anahtarı tanımlı olmadığından bu sürümler `latest.json` ve `.sig` içermez; GitHub Releases sayfasındaki kurulum EXE'si elle çalıştırılır. Aşağıdaki imzalı yayın akışı, Actions ve imza anahtarı hazır olduğunda kullanılabilir.

**v1.2.0:** İmza anahtarı henüz tanımlı değildir. Release akışı testlerden sonra imzasız EXE, NSIS kurulum paketi ve `SHA256SUMS.txt` yayımlar; bu sürüm de elle kurulur.

Depo herkese açık olmalıdır. Özel depoda `latest.json` oturumsuz indirilemez; uygulama “güncelleme yok” veya ağ hatası gösterir.

İmzasız yerel derleme (`npm run desktop:build:unsigned`) veya el ile kopyalanan EXE bu kanalı kullanamaz. Güncelleme, Release işinin ürettiği imzalı kurulum paketinden gelir.

## Yayımlama

1. Depoyu GitHub’da **public** yapın (Settings → General → Danger zone → Change repository visibility).
2. Depo sırlarına ekleyin:
   - `TAURI_SIGNING_PRIVATE_KEY`: minisign özel anahtarının içeriği
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: anahtar parolası; yoksa boş bırakın
3. `package.json`, `crates/serverbond-core/Cargo.toml`, `src-tauri/tauri.conf.json` ve `src-tauri/Cargo.toml` sürümlerini birlikte yükseltin; npm/Cargo kilit dosyalarını yenileyin ve `docs/releases/v<sürüm>.md` notlarını ekleyin.
4. Sürümle eşleşen etiketi itin veya **Release** işini o etiket üzerinde elle çalıştırın. `main` üzerinden doğrudan yayın reddedilir.

```powershell
git tag v1.2.0
git push origin v1.2.0
```

`Release` işi derleme, biçim, Clippy, standart testler ve gerçek Windows servis testlerini çalıştırır. İmza anahtarı varsa imzalı NSIS, `latest.json` ve `.sig`; yoksa imzasız NSIS üretir. Sürüm numaralı EXE, kurulum paketi ve SHA-256 özetleri yüklenene kadar yayın taslak kalır. Genel bakıştaki güncelleyici yalnızca imzalı, yayımlanmış (draft olmayan) sürümleri görür.

Anahtar üretmek:

```powershell
npx @tauri-apps/cli signer generate -w serverbond-updater.key
```

Açık anahtarı `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` alanına yazın. Özel anahtarı depoya eklemeyin (`.gitignore` `*.key` dosyalarını dışlar). Açık anahtarı değiştirecekseniz daha önce imzalanmış kurulumlar yeni sürümü doğrulayamaz; ilk yayımlanan güncelleyici anahtarını saklayın.

## Yerel derleme

İmzalı üretim paketi anahtar ister:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw .\serverbond-updater.key
npm run desktop:build
```

İmzasız CI / geliştirme paketi:

```powershell
npm run desktop:build:unsigned
```

`createUpdaterArtifacts` üretim yapılandırmasında açıktır. İmzasız derleme `.github/tauri-unsigned.json` ile bu adımı kapatır.

## Güvenlik

- İndirilen paket minisign ile `pubkey` karşısında doğrulanır.
- CSP, güncelleyicinin `github.com`, `objects.githubusercontent.com` ve `release-assets.githubusercontent.com` adreslerine bağlanmasına izin verir.
- `relaunch` mevcut ServerBond sürecini kapatır; `Manager` düşerken servisler durur.
