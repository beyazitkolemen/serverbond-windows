---
name: f4box-architecture
description: F4Box katmanları, veri dizini ve nereye kod yazılacağı. Mimari, dosya yerleşimi, IPC sınırı veya yeni özellik iskeleti sorulduğunda kullan.
---

# F4Box mimarisi

## Katmanlar

| Yol | Görev |
| --- | --- |
| `crates/f4box-core` | Katalog, indirme, arşiv, süreç, MySQL, isteğe bağlı PostgreSQL, Caddy, projeler, kuyruk/zamanlayıcı, yerel sürüm, CLI |
| `src-tauri` | Tauri komutları, tepsi, Windows başlangıç, klasör seçimi |
| `src` | React/TypeScript. Tek yazma yolu `call()` → IPC |
| `crates/f4box-core/src/bin/f4box.rs` | Aynı `Manager` ile CLI |

Yeni iş kuralı çekirdeğe yazılır, masaüstünde yalnızca komut dışa aktarılır, arayüz `snapshot` okur. F4Box Windows üretim ortamıdır (Forge hissi); uzak SSH ve Herd/Laragon geliştirme kopyası yoktur. Yerel sürüm tarifi `release.rs` + `Project.release`.

## Veri dizini

Varsayılan `%LOCALAPPDATA%\F4Box`, geliştirmede `F4BOX_HOME`. İkinci süreç `manager.lock` ile reddedilir.

```text
bin/  cache/  config/  data/  backups/  logs/  projects/  www/  config.json
```

`projects/` yeni Laravel köklerinin varsayılan çalışma alanıdır. `www/` eski düz yerleşim için taranmaya devam eder. Kullanıcı `projects_dir` verirse yalnızca o yol okunur.

`config.json` 2 MB, en fazla 1000 proje. Kayıt atomiktir; önceki geçerli kopya `config.last-good.json`.

## Sabitler

- Web `8088`, MySQL `13306`, PHP FastCGI `19000`, isteğe bağlı HTTPS `8443`, isteğe bağlı PostgreSQL `15432` (ilk açılışta boş port seçilebilir)
- Proje adresi `{name}.localhost` — hosts dosyası yok
- phpMyAdmin `phpmyadmin.f4box.localhost`
- Ortam loopback; dışarı açılmaz
- HTTPS: Caddy `tls internal`; HTTP aynı hostta HTTPS portuna yönlenir

## Dokunulmaması gerekenler

- Mevcut Laravel `.env` otomatik değiştirilmez
- Herd/Laragon/Docker kurulumlarına dokunulmaz
- DPAPI parolası Windows kullanıcısına bağlıdır; taşıma SQL yedeği iledir
