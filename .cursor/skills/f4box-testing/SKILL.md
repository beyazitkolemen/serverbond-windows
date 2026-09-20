---
name: f4box-testing
description: F4Box test matrisi ve asistanların hangi komutu nerede çalıştıracağı. Test, CI, clippy veya doğrulama sorulduğunda kullan.
---

# Testler

| Komut | Ortam | Kapsam |
| --- | --- | --- |
| `npm run test:ai` | Linux Cloud Agent | Vite/tsc, `f4box-core`, rustfmt, clippy |
| `npm run test:core` | Her yer | `cargo test -p f4box-core` |
| `npm run check` | Windows | Arayüz + tüm workspace clippy |
| `npm run test:integration` | Windows, uygulama kapalı | Gerçek paket + servis duman testi |
| `cargo test -p f4box-core --tests -- --ignored` | Windows | php_matrix, environment, preferences_runtime |

## Alışkanlık

- Yeni çekirdek davranışı `crates/f4box-core/tests` altına yazılır.
- Bozuk config, kilitli dosya, kayıp MySQL verisi ve panik senaryoları `resilience` testlerinde durur; sessiz sıfırlama ekleme.
- Linux’ta Tauri/NSIS veya `ignored` indirme testini çalıştırıyormuş gibi davranma.
- Ayrıntı: [docs/ai-environment.md](../../../docs/ai-environment.md).
