# Yönetim API'si

ServerBond, arayüzün ve komut satırının yaptığı her işi yerel bir HTTP API üzerinden de sunar. Betikler, CI ajanları veya uzaktan yönetim araçları pencereye dokunmadan hizmetleri başlatabilir, proje ekleyebilir, sürüm çalıştırabilir, kuyruk ve zamanlayıcıyı yönetebilir.

## Güvenlik modeli

- Dinleyici yalnızca `127.0.0.1` üzerine bağlanır; dışarıdan erişim için ayrı bir ters vekil veya tünel gerekir ve önerilmez.
- Her istek `Authorization: Bearer <jeton>` başlığı ister. Tek istisna `GET /api/v1/health`.
- Jeton `sb_` önekli 64 karakterlik rastgele bir dizedir. Diskte yalnızca SHA-256 özeti (`config/api-token.sha256`) saklanır; jetonun kendisi oluşturulduğu anda **bir kez** gösterilir.
- Yeni jeton oluşturmak eskisini anında geçersiz kılar. Jeton silinirse API tüm istekleri `401` ile reddeder.
- İstek gövdesi 1 MB ile sınırlıdır (`413`). Aynı anda en fazla 8 istek işlenir; fazlası `503` alır.
- Her istek `Manager::contain` içinde çalışır: bir panik durumunda süreç çökmez, `500` döner ve uygulama "yeniden başlatma gerekli" durumuna geçer.
- API varsayılan olarak **kapalıdır**. Ayarlar → API'den ya da `serverbond api serve` ile açılır.

## Açma ve jeton

Arayüz: **Ayarlar → API** → "API'yi aç" anahtarı, port (varsayılan `18800`) ve **Jeton oluştur**. Kaydet düğmesi ayarı uygular; dinleyici ortam çalışırken de açılıp kapatılabilir.

Komut satırı (masaüstü uygulaması kapalıyken; veri klasörü kilidi tek süreçlidir):

```powershell
serverbond api token          # yeni jeton üretir ve bir kez yazdırır
serverbond api serve [port]   # API'yi açar, bu süreçte dinler; Enter durdurur
serverbond api status         # enabled/port/listening/tokenSaved/baseUrl
serverbond api routes         # yol listesi
serverbond api forget         # jetonu siler
```

## Yanıt biçimi

Her yanıt JSON'dur:

```json
{ "ok": true,  "data": … }
{ "ok": false, "error": "Türkçe hata metni" }
```

| Durum | Anlamı |
| --- | --- |
| `200` | İşlem tamamlandı; `data` sonucu içerir. |
| `400` | Doğrulama veya işlem hatası. `error`, arayüzde görünen metnin aynısıdır ("Başka bir işlem devam ediyor…" dahil). |
| `401` | Jeton yok veya yanlış. |
| `404` | Yol bilinmiyor. `GET /api/v1` yol listesini verir. |
| `413` | Gövde 1 MB'ı aşıyor. |
| `500` | İşlem panikledi; uygulama yeniden başlatılmalı. |
| `503` | Çok fazla eş zamanlı istek. |

Uzun işlemler (kurulum, `composer install`, sürüm) yanıt dönmeden önce tamamlanır. İstemci tarafında zaman aşımını buna göre (birkaç dakika) ayarlayın. Eş zamanlı durum sorguları engellenmez; her istek kendi iş parçacığında çalışır.

## Yollar

Ön ek: `http://127.0.0.1:<port>/api/v1`. `{id}` yerine proje **kimliği** (UUID) veya proje **adı** yazılabilir.

### Genel

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| GET | `/health` | Sürüm, `busy`, `anyRunning`, `recoveryIssue` (jeton gerekmez) |
| GET | `/` | Yol listesi |
| GET | `/status` | Arayüzün kullandığı tam `Snapshot` |
| GET | `/requirements` | Windows ön koşul denetimleri |
| GET | `/logs/{id}` | Hizmet günlüğü (`serverbond`, `php`, `mysql`, `caddy`, `mailpit`, …) |

### Ayarlar

| Yöntem | Yol | Gövde |
| --- | --- | --- |
| GET | `/settings` | — |
| PUT | `/settings` | Tam `Settings` nesnesi (GET çıktısını değiştirip gönderin) |
| GET | `/settings/defaults` | — |
| GET | `/settings/previous` | — |

Ortam çalışırken port ve sunucu ayarları değiştirilemez (`400`); klasör, açılış ve API tercihleri kaydedilebilir. `api.enabled` veya `api.port` değişirse dinleyici yanıt verdikten sonra yeni ayara geçer.

### Hizmetler ve PHP

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| POST | `/services/{id}/{action}` | `id`: `all`, `php`, `mysql`, `caddy`, `composer`, `phpmyadmin`, `mail`, `postgres`, `redis`, `tunnel`, `node`. `action`: `install`, `repair`, `start`, `stop`, `restart`. |
| POST | `/php/{version}/select` | Varsayılan PHP sürümünü seçer (kurulu değilse indirir) |
| POST | `/php/{version}/repair` | PHP sürümünü onarır |

`node` yalnızca `install`/`repair` kabul eder. `restart`, `all` için tüm ortamı, tek hizmet için durdur-başlat sırasını uygular.

### Projeler

| Yöntem | Yol | Gövde / Açıklama |
| --- | --- | --- |
| GET | `/projects` | Durumlu proje listesi |
| POST | `/projects` | `{ "name": "magaza", "path": "C:\\\\dev\\\\magaza" }` — mevcut Laravel kökünü ekler |
| POST | `/projects/create` | `{ "name": "magaza", "parent": "C:\\\\dev" }` — Composer ile yeni Laravel |
| POST | `/projects/import` | `{ "url": "https://github.com/owner/repo", "name?": "…", "branch?": "main" }` — klonlar ve ekler |
| GET | `/projects/discover` | Çalışma alanındaki Laravel kökleri |
| POST | `/projects/import-folders` | `{ "paths": ["C:\\\\dev\\\\a", "C:\\\\dev\\\\b"] }` |
| GET | `/projects/{id}` | Tek proje |
| DELETE | `/projects/{id}` | Listeden kaldırır; dosyalar ve veritabanı korunur |
| POST | `/projects/{id}/php` | `{ "version": "8.3.33" }` |

### Sürüm (deployment)

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| POST | `/projects/{id}/deploy` | Sürüm tarifini çalıştırır; `ReleaseRecord` döner (`success`, `branch`, `sha`, `output`, `durationMs`) |
| GET | `/projects/{id}/releases` | Geçmiş, en yeni önce |
| GET | `/projects/{id}/release` | Tarif: `branch`, `composer`, `migrate`, `optimizeClear`, `restartJobs`, `extraArtisan` |
| PUT | `/projects/{id}/release` | Tarifi kaydeder |
| GET | `/projects/{id}/git` | `{ present, branch, sha }` |

Başarısız sürüm `400` döner ama geçmişe de yazılır; `output` alanı `--- git ---`, `--- composer ---` gibi bölümlerle her adımın çıktısını içerir.

### Ortam dosyası

| Yöntem | Yol | Gövde |
| --- | --- | --- |
| GET | `/projects/{id}/env` | `{ exists, content, example }` |
| PUT | `/projects/{id}/env` | `{ "content": "APP_ENV=production\\n…" }` (256 KB, UTF-8) |

### Kuyruk ve zamanlayıcı

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| GET | `/projects/{id}/jobs` | `{ workers, schedule }` |
| PUT | `/projects/{id}/jobs` | Aynı şekil; kaydeder ve çalışan süreçleri uzlaştırır |
| POST | `/projects/{id}/workers/{workerId}/{start\|stop\|restart}` | Tek işçi havuzu |
| GET | `/projects/{id}/schedule` | `schedule:list` çıktısı |
| POST | `/projects/{id}/schedule/{start\|stop\|restart}` | Zamanlayıcı |
| GET | `/projects/{id}/failed-jobs` | `queue:failed` çıktısı |
| POST | `/projects/{id}/failed-jobs/retry` | `{ "job?": "uuid" }`; gövde yoksa tümü |
| POST | `/projects/{id}/failed-jobs/flush` | Başarısızları temizler |
| GET | `/projects/{id}/logs?source=php` | `source`: `php`, `schedule`, `worker:<workerId>` |

İşçi nesnesi: `id`, `name`, `enabled`, `autoStart`, `connection`, `queue`, `processes` (1–8), `timeout`, `memory`, `sleep`, `maxTries`, `backoff`, `maxJobs`, `maxTime`. Yeni işçi için `id` boş bırakılabilir; sunucu üretir.

### Veritabanı ve sırlar

| Yöntem | Yol | Gövde |
| --- | --- | --- |
| POST | `/projects/{id}/database/create` | — (MySQL çalışıyor olmalı) |
| POST | `/projects/{id}/database/backup` | — → `{ "path": "…\\\\backups\\\\magaza-….sql" }` |
| POST | `/projects/{id}/database/restore` | `{ "path": "C:\\\\yedek.sql" }` |
| POST | `/mysql/password` | `{ "password": "…" }` (8–128 ASCII; boşluk, tırnak, `#`, `;`, `\\` yok) |
| POST | `/postgres/password` | `{ "password": "…" }` |
| POST | `/tunnel/token` | `{ "token": "…" }` |
| DELETE | `/tunnel/token` | — |
| POST | `/tunnel/auto-start` | `{ "autoStart": true }` |
| POST | `/github/token` | `{ "token": "ghp_…" }` |
| DELETE | `/github/token` | — |
| POST | `/https/{trust\|untrust}` | Yerel CA'yı Windows kullanıcı deposuna ekler/kaldırır |

Parolalar ve jetonlar yanıtlarda hiç dönmez; API yalnızca kayıt durumunu bildirir.

## Örnekler

PowerShell:

```powershell
$token = "sb_…"
$h = @{ Authorization = "Bearer $token" }
Invoke-RestMethod http://127.0.0.1:18800/api/v1/status -Headers $h | ConvertTo-Json -Depth 5

Invoke-RestMethod -Method Post http://127.0.0.1:18800/api/v1/services/all/start -Headers $h

$body = @{ url = "https://github.com/acme/magaza"; branch = "main" } | ConvertTo-Json
Invoke-RestMethod -Method Post http://127.0.0.1:18800/api/v1/projects/import -Headers $h -Body $body -ContentType "application/json"

Invoke-RestMethod -Method Post http://127.0.0.1:18800/api/v1/projects/magaza/deploy -Headers $h -TimeoutSec 900
```

curl:

```bash
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:18800/api/v1/projects
curl -X PUT -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"content":"APP_ENV=production\nAPP_DEBUG=false\n"}' \
  http://127.0.0.1:18800/api/v1/projects/magaza/env
```

Tipik dağıtım betiği: `GET /health` → `POST /projects/{ad}/deploy` → yanıtın `success` alanını denetle → başarısızsa `output`u günlüğe yaz.

## Uygulama notları

- Kod: `crates/serverbond-core/src/api.rs`. Yönlendirme `route()` ve `project_route()` içindeki desen eşlemeleridir; yeni yol eklerken `routes()` listesini de güncelleyin.
- Sunucu `tiny_http` üzerinde çalışır; kabul döngüsü `serverbond-api` iş parçacığında, her istek `serverbond-api-request` iş parçacığındadır. `ApiServer` düşürüldüğünde `unblock()` ile döngü kapanır ve iş parçacığı birleştirilir.
- `Manager::ensure_api` ayarı dinleyiciyle eşitler: açılışta (Tauri `setup`), her `save_settings` sonrasında ve API'nin kendi `PUT /settings` yolunda çağrılır. `shutdown()` dinleyiciyi kapatır.
- Testler: `crates/serverbond-core/tests/api.rs` gerçek bir dinleyici açar; yetkilendirme, yol listesi, JSON hata gövdeleri, ada göre proje, jeton yenileme ve dinleyicinin ayarlarla kapanmasını doğrular.
