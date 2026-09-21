---
name: serverbond-architecture
description: ServerBond katmanları, veri dizini ve nereye kod yazılacağı. Mimari, dosya yerleşimi, IPC sınırı veya yeni özellik iskeleti sorulduğunda kullan.
---

# ServerBond mimarisi

## Katmanlar

| Yol | Görev |
| --- | --- |
| `crates/serverbond-core/src/domain` | `ComponentId`, `ToolAction`, `GithubAction`, `EnvironmentAction` |
| `crates/serverbond-core/src/repository` | `DataDir` — `config.json`, `bin/<id>/<version>` yerleşimi |
| `crates/serverbond-core` | `Manager` cephesi: katalog, indirme, süreç, MySQL, isteğe bağlı PostgreSQL/Redis, Caddy, projeler, kuyruk/zamanlayıcı, yerel sürüm, CLI |
| `src-tauri` | Tauri komutları; eylem dizelerini domain enum’una parse eder |
| `src/domain` | `Page`, `WorkspaceService`, `ToolCommand`, `ToolAction`, `ComponentId` |
| `src/services` | IPC sarmalayıcılar (`mailService`, `environmentService`, …) |
| `src/repositories` | `snapshotRepository.get()` |
| `src` | React. Yeni yazma `services` üzerinden; ham `call("mail")` ekleme |
| `crates/serverbond-core/src/bin/serverbond.rs` | Aynı `Manager` ile CLI |
| `crates/serverbond-core/src/api.rs` | Yerel HTTP API (127.0.0.1, Bearer jeton); yeni yol `route()`/`project_route()` + `routes()` |
| `src/hooks` | `useDraft` (anlık görüntüye karşı yerel taslak), `useTheme` |
| `src/components` paylaşılanlar | `StatusBadge`, `Toggle`, `NumberField`, `SegmentedControl`, `EmptyState`, `ErrorBoundary` |

Yeni iş kuralı çekirdeğe yazılır, masaüstünde yalnızca komut dışa aktarılır, arayüz `snapshot` okur. `Manager` tam rewrite edilmez. Tel dizileri değişmez: süreç `mailpit`, IPC komutu `mail`, eylem `install`/`start`. Komut adı süreç kimliği değildir. ServerBond Windows üretim ortamıdır (Forge hissi); uzak SSH ve Herd/Laragon geliştirme kopyası yoktur. Yerel sürüm tarifi `release.rs` + `Project.release`.

## Veri dizini

Varsayılan `%LOCALAPPDATA%\ServerBond`. Geliştirmede `SERVERBOND_HOME`. Önceki kurulumların verileri `legacy.rs` okuma uyumluluğuyla korunur; `docs/naming.md` sözleşmesini izleyin. İkinci süreç `manager.lock` ile reddedilir.

```text
bin/  cache/  config/  data/  backups/  logs/  projects/  www/  config.json
```

`projects/` yeni Laravel köklerinin varsayılan çalışma alanıdır. `www/` eski düz yerleşim için taranmaya devam eder. Kullanıcı `projects_dir` verirse yalnızca o yol okunur.

`config.json` 2 MB, en fazla 1000 proje. Kayıt atomiktir; önceki geçerli kopya `config.last-good.json`.

## Sabitler

- Web `8088`, MySQL `13306`, PHP FastCGI `19000`, isteğe bağlı HTTPS `8443`, isteğe bağlı PostgreSQL `15432`, isteğe bağlı Redis `16379`, API `18800` (ilk açılışta boş port seçilebilir)
- Proje adresi `{name}.localhost` — hosts dosyası yok
- phpMyAdmin `phpmyadmin.serverbond.localhost`
- Ortam loopback; dışarı açılmaz
- HTTPS: Caddy `tls internal`; HTTP aynı hostta HTTPS portuna yönlenir

## Dokunulmaması gerekenler

- Mevcut Laravel `.env` otomatik değiştirilmez; kullanıcı Ortam sekmesinden Kaydet ile yazar
- Herd/Laragon/Docker kurulumlarına dokunulmaz
- DPAPI parolası ve GitHub jetonu Windows kullanıcısına bağlıdır; taşıma SQL yedeği iledir

Ayrıntı: `docs/architecture.md`, `docs/api.md`.
