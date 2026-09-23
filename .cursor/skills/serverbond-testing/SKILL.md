---
name: serverbond-testing
description: ServerBond test matrisi ve asistanların hangi komutu nerede çalıştıracağı. Test, CI, clippy veya doğrulama sorulduğunda kullan.
---

# Testler

| Komut | Ortam | Kapsam |
| --- | --- | --- |
| `npm run test:ai` | Linux Cloud Agent | Vite/tsc, `serverbond-core`, rustfmt, clippy |
| `npm run test:core` | Her yer | Nextest ile paralel `serverbond-core` testleri |
| `npm run test:core:serial` | Her yer | Seri çekirdek testi karşılaştırması |
| `npm run test:rust:parallel` | Windows/Linux, Nextest 0.9.146 | Workspace testleri, kullanılabilir işlemci sayısına göre en fazla sekiz eşzamanlı test |
| `npm run check` | Windows | Arayüz + tüm workspace clippy |
| `npm run desktop:build:unsigned` | Windows CI | İmzasız NSIS/EXE |
| `.github/workflows/release.yml` | sürümle eşleşen tag `v*` | Gerçek servis testlerinden sonra EXE/NSIS ve SHA-256; anahtar varsa imzalı güncelleme paketi |
| `npm run test:integration` | Windows, uygulama kapalı | Gerçek paket + servis duman testi |
| `cargo test -p serverbond-core --tests -- --ignored` | Windows | php_matrix, environment, preferences_runtime |

## Alışkanlık

- Yeni çekirdek davranışı `crates/serverbond-core/tests` altına yazılır.
- Bozuk config, kilitli dosya, kayıp MySQL verisi ve panik senaryoları `resilience` testlerinde durur; sessiz sıfırlama ekleme.
- Linux’ta Tauri/NSIS veya `ignored` indirme testini çalıştırıyormuş gibi davranma.
- Gerçek MySQL veya PostgreSQL kuran testler yalnızca o servisin kaynağı, testi veya paket sabiti değiştiyse çalışır. Ayrıntı: `.cursor/rules/long-service-tests.mdc`.
- Ayrıntı: [docs/ai-environment.md](../../../docs/ai-environment.md).
