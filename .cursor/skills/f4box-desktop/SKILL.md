---
name: f4box-desktop
description: Tauri IPC, sistem tepsisi, Windows başlangıç ve masaüstü tercihleri. Pencere, tray, autostart veya src-tauri değişince kullan.
---

# Masaüstü (Tauri 2)

`src-tauri` ince kalır. İş kuralı `crates/f4box-core::Manager` içindedir; komutlar `blocking` + `contain()` ile çalışır.

## IPC

Komutlar `src-tauri/src/main.rs` içinde. Arayüz `src/api.ts` → `call()`. Okuma (`snapshot`, `read_log`, …) 10 sn UI zaman aşımına girebilir; yazma/kurulum zaman aşımıyla yeniden başlatılmaz.

İzinler: `capabilities/default.json` — `core:default`, `dialog:allow-open`. Yeni dosya erişimi için yetki genişletmeden önce gerekçeyi yaz.

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
