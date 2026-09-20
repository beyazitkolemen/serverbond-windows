# F4Box — asistan notları

Windows x64 üzerinde Laravel üretim ortamını (Forge hissi, uzak VPS yok) yöneten Tauri + Rust uygulaması. Cloud Agent Linux üzerindedir; masaüstü EXE burada üretilmez.

## Git

Her zaman `main`. `git fetch --prune`, `origin/main` fast-forward, commit ve `git push origin main`. **PR oluşturma yok** (`gh pr create`, ManagePullRequest, draft PR). Özellik dalı açılmaz. Başka dalda veya açık PR’da kalmış iş `main`'e merge edilir. Force-push yok.

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

- `crates/f4box-core`: katalog, indirme, süreç, MySQL, projeler, kuyruk/zamanlayıcı, Cloudflare tüneli, Mailpit, Windows izinleri. Domain enum’ları `crates/f4box-core/src/domain`, yerleşim `crates/f4box-core/src/repository` (`DataDir`). `Manager` cephe olarak kalır.
- `src-tauri`: dar IPC, tepsi, GitHub güncelleyici eklentileri; eylem dizeleri `ToolAction` / `GithubAction` / `EnvironmentAction`
- `src`: React. `src/domain` (Page, WorkspaceService, ToolCommand), `src/services` (IPC), `src/repositories/snapshot.ts`. Ham `call("mail")` ekleme. `src/updates.ts` GitHub sürüm denetimi.
- Sabit paketler `catalog.json` / `php-versions.json` / `tools.json`; SHA-256 olmadan güncellenmez
- Uygulama güncellemesi: imzalı GitHub Release + `docs/updates.md`. Depo public; sırlar repoda değil.

## Cursor skill ve kurallar

Kurallar (otomatik): `.cursor/rules/` — `f4box.mdc`, `git-main.mdc`, `catalog.mdc`, `frontend.mdc`.

Skill’ler (konuya göre oku):

| Skill | Ne zaman |
| --- | --- |
| `f4box-architecture` | Katman, veri dizini, yeni özellik yeri |
| `f4box-catalog` | PHP/MySQL/Caddy/Composer/phpMyAdmin sürüm ve hash |
| `f4box-runtime` | Servis, proje, port, onarım, phpMyAdmin |
| `f4box-desktop` | Tauri, tepsi, Windows başlangıç |
| `f4box-testing` | Hangi testi nerede çalıştıracağın |
