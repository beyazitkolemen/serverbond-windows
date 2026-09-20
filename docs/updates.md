# GitHub üzerinden otomatik güncelleme

F4Box, yayımlanmış GitHub sürümlerindeki imzalı NSIS paketini `latest.json` ile denetler. Kullanıcı onayı olmadan indirmez veya kurmaz.

## Kullanıcı

1. Masaüstü uygulamasını açın. Yeni sürüm varsa genel bakışta bir bildirim görünür.
2. **Ayarlar → Güncellemeler** veya tepsi menüsündeki **Güncellemeleri denetle** ile GitHub’ı sorun.
3. Sürüm notlarını okuyup **kur ve yeniden başlat** deyin. Çalışan PHP/MySQL/Caddy durur; kurulum bitince F4Box yeniden açılır.

Kaynak: `https://github.com/beyazitkolemen/f4box-laravel/releases/latest/download/latest.json`

Depo herkese açık olmalıdır. Özel depoda `latest.json` oturumsuz indirilemez; uygulama “güncelleme yok” veya ağ hatası gösterir.

İmzasız yerel derleme (`npm run desktop:build:unsigned`) veya el ile kopyalanan EXE bu kanalı kullanamaz. Güncelleme, Release işinin ürettiği imzalı kurulum paketinden gelir.

## Yayımlama

1. Depoyu GitHub’da **public** yapın (Settings → General → Danger zone → Change repository visibility).
2. Depo sırlarına ekleyin:
   - `TAURI_SIGNING_PRIVATE_KEY`: minisign özel anahtarının içeriği
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: anahtar parolası; yoksa boş bırakın
3. `package.json`, `src-tauri/tauri.conf.json` ve `src-tauri/Cargo.toml` sürümlerini birlikte yükseltin.
4. `v1.2.0` gibi bir etiket itin veya **Release** işini elle çalıştırın.

```powershell
git tag v1.2.0
git push origin v1.2.0
```

`Release` işi Windows’ta imzalı NSIS, `latest.json` ve `.sig` üretir. Genel bakıştaki güncelleyici yalnızca yayımlanmış (draft olmayan) sürümleri görür.

Anahtar üretmek:

```powershell
npx @tauri-apps/cli signer generate -w f4box-updater.key
```

Açık anahtarı `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` alanına yazın. Özel anahtarı depoya eklemeyin (`.gitignore` `*.key` dosyalarını dışlar). Açık anahtarı değiştirecekseniz daha önce imzalanmış kurulumlar yeni sürümü doğrulayamaz; ilk yayımlanan güncelleyici anahtarını saklayın.

## Yerel derleme

İmzalı üretim paketi anahtar ister:

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw .\f4box-updater.key
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
- `relaunch` mevcut F4Box sürecini kapatır; `Manager` düşerken servisler durur.
