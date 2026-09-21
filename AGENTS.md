# ServerBond — asistan notları

Windows x64 üzerinde Laravel üretim ortamını (Forge hissi, uzak VPS yok) yöneten Tauri + Rust uygulaması. Cloud Agent Linux üzerindedir; masaüstü EXE burada üretilmez.

## Git

Her zaman `main`. `git fetch --prune`, `origin/main` fast-forward, commit ve `git push origin main`. **PR oluşturma yok** (`gh pr create`, ManagePullRequest, draft PR). Özellik dalı açılmaz. Başka dalda veya açık PR’da kalmış iş `main`'e merge edilir. Force-push yok.

## Her değişiklikten sonra build ve release

Kullanıcının kalıcı talimatı: Bundan sonraki her tamamlanan Windows değişikliği yeni bir build ve yayımlanmış GitHub Release ile teslim edilir. Yalnızca commit/push yapmak işi tamamlamaz. Aynı görev içindeki düzenlemeler tek sürümde toplanır; her dosya kaydında ayrı release açılmaz.

- Varsayılan sürüm artışı **patch**: `1.3.0 → 1.3.1 → 1.3.2`. Kullanıcı istemedikçe minor/major artırılmaz. Başlamadan önce yerel sürümleri ve uzak etiket/yayınları kontrol et; mevcut etiketi veya yayımlanmış paketi değiştirme.
- package.json, package-lock.json, iki Cargo.toml, Cargo.lock ve tauri.conf.json sürümlerini birlikte güncelle; sürüm notlarını hazırla.
- Değişikliğe uygun testleri ve gerekli yayın kontrollerini çalıştır. Son kaynaklardan arayüzü ve Windows EXE/NSIS paketini yeniden derle; önceki sürümün binary dosyasını yeniden adlandırarak kullanma.
- `main` commit/push ardından yeni sürüm etiketini ve release'i yayımla. EXE, kurulum paketi ve SHA256SUMS.txt dosyalarını yükle; yayımlanan sürümü ve indirilen dosyaların SHA-256 özetlerini doğrula.
- İmza anahtarı yoksa mevcut imzasız yayın yöntemini kullan ve elle kurulum gerektiğini belirt. İmza üretildiğini veya otomatik güncellemenin doğrulandığını iddia etme.
- Build/test/yayın başarısızsa hatayı gider. Çözülemeyen engeli ve henüz yayımlanmadığını açıkça bildir; işi release tamamlanmış gibi kapatma. Yerel doğrulama ile CI sonuçlarını ayrı raporla.
- Bu talimat sonraki değişiklikler için build ve release yetkisidir; her seferinde yeniden onay isteme. Kullanıcı belirli bir görev için farklı talimat verirse onu uygula.

Yayın akışı: [docs/updates.md](docs/updates.md). Cursor kuralı: [.cursor/rules/release-after-change.mdc](.cursor/rules/release-after-change.mdc).

## Kurulum

```bash
bash scripts/ai/install.sh
```

## Test

```bash
npm run test:ai
```

Ayrıntı: [docs/ai-environment.md](docs/ai-environment.md).

## Çalıştırma

- Salt okunur önizleme: `npm run dev` → `http://127.0.0.1:1420`
- Gerçek kurulum ve servisler: Windows’ta `npm run desktop`

## Kod

- `crates/serverbond-core`: katalog, indirme, süreç, MySQL, projeler, kuyruk/zamanlayıcı, Cloudflare tüneli, Mailpit, Windows izinleri. Domain enum’ları `crates/serverbond-core/src/domain`, yerleşim `crates/serverbond-core/src/repository` (`DataDir`). `Manager` cephe olarak kalır.
- `src-tauri`: dar IPC, tepsi, GitHub güncelleyici eklentileri; eylem dizeleri `ToolAction` / `GithubAction` / `EnvironmentAction`
- `src`: React. `src/domain` (Page, WorkspaceService, ToolCommand), `src/services` (IPC), `src/repositories/snapshot.ts`. Ham `call("mail")` ekleme. `src/updates.ts` GitHub sürüm denetimi.
- Sabit paketler `catalog.json` / `php-versions.json` / `tools.json`; SHA-256 olmadan güncellenmez
- Uygulama güncellemesi: imzalı GitHub Release + `docs/updates.md`. Depo public; sırlar repoda değil.

## Cursor skill ve kurallar

Kurallar (otomatik): `.cursor/rules/` — `serverbond.mdc`, `git-main.mdc`, `catalog.mdc`, `frontend.mdc`.

Skill’ler (konuya göre oku):

| Skill | Ne zaman |
| --- | --- |
| `serverbond-architecture` | Katman, veri dizini, yeni özellik yeri |
| `serverbond-catalog` | PHP/MySQL/Caddy/Composer/phpMyAdmin sürüm ve hash |
| `serverbond-runtime` | Servis, proje, port, onarım, phpMyAdmin |
| `serverbond-desktop` | Tauri, tepsi, Windows başlangıç |
| `serverbond-testing` | Hangi testi nerede çalıştıracağın |
