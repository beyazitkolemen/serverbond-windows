---
name: f4box-desktop
description: Tauri IPC, sistem tepsisi, Windows başlangıç ve masaüstü tercihleri. Pencere, tray, autostart veya src-tauri değişince kullan.
---

# Masaüstü (Tauri 2)

`src-tauri` ince kalır. İş kuralı `crates/f4box-core::Manager` içindedir; komutlar `blocking` + `contain()` ile çalışır.

## IPC

Komutlar `src-tauri/src/main.rs` içinde. Arayüz `src/api.ts` → `call()`. Okuma (`snapshot`, `read_log`, `read_project_worker_log`, `read_project_schedule_log`, `list_project_schedule`, `list_failed_jobs`, `discover_projects`, …) 10 sn UI zaman aşımına girebilir; yazma/kurulum zaman aşımıyla yeniden başlatılmaz. `database` `create|backup|restore`; `https_trust` `trust|untrust`; `import_projects` yolları alır. Kuyruk: `save_project_jobs`, `start|stop|restart_project_worker`, `start|stop|restart_project_schedule`, `retry_failed_jobs`, `flush_failed_jobs`.

İzinler: `capabilities/default.json` — `core:default`, `dialog:allow-open`, `updater:default`, `process:allow-restart`. Yeni dosya erişimi için yetki genişletmeden önce gerekçeyi yaz.

## Güncelleyici

`tauri-plugin-updater` + `tauri-plugin-process`. Uç nokta GitHub `releases/latest/download/latest.json`. Kullanıcı onayı olmadan kurma. Arayüz: `src/updates.ts`, `Ayarlar → Güncellemeler`, tepsi **Güncellemeleri denetle**. İmzasız CI: `npm run desktop:build:unsigned`. İmzalı sürüm: `.github/workflows/release.yml` ve `docs/updates.md`. Depo public olmalı; özel anahtar `TAURI_SIGNING_PRIVATE_KEY` sırrındadır, repoya yazılmaz.

## Pencere ve tepsi

- Ana pencere başlangıçta gizli; tepsi yoksa veya kurtarma varsa gösterilir
- X: `close_to_tray` açıksa gizle, değilse servisleri durdurup çık
- Menü `Çıkış` her zaman tam kapanış
- `--autostart` + `start_minimized` + tepsi + sağlıklı config → gizli açılış
- Tek örnek: `tauri-plugin-single-instance`

## Windows başlangıç

`src-tauri/src/startup.rs` kullanıcı Run kaydı. Yönetici istemez. 260 karakter sınırı, boşluklu/Türkçe yollar tırnaklanır. Program yeri değişince kayıt yenilenmeli.

## Geliştirme

`tauri.conf.json` `devUrl` = `http://127.0.0.1:1420` (Vite IPv4). Linux Cloud Agent `npm run desktop` çalıştırmaz.
