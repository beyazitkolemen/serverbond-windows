# F4Box — asistan notları

Windows x64 için PHP, MySQL, Caddy ve Laravel projelerini yöneten Tauri + Rust uygulaması. Cloud Agent Linux üzerinde çalışır; masaüstü EXE burada üretilmez.

## Kurulum

```bash
bash scripts/ai/install.sh
```

## Test

```bash
npm run test:ai
```

Bu komut arayüz derlemesini, `f4box-core` testlerini, rustfmt ve clippy denetimini çalıştırır. Ayrıntılar: [docs/ai-environment.md](docs/ai-environment.md).

## Çalıştırma

- Salt okunur önizleme: `npm run dev` → `http://127.0.0.1:1420`
- Gerçek kurulum ve servisler: Windows’ta `npm run desktop`

## Kod

- `crates/f4box-core`: katalog, indirme, süreç, MySQL, projeler
- `src-tauri`: dar IPC ve tepsi
- `src`: React arayüzü
- Sabit paket sürümleri `crates/f4box-core/catalog.json` ve `php-versions.json` içindedir; SHA-256 olmadan güncellenmez
