# Yönetim API'si

ServerBond'un servis, proje, yapılandırma ve masaüstü işlemleri yerel HTTP API üzerinden yönetilir. Betikler hizmetleri başlatabilir, proje ekleyebilir, deployment çalıştırabilir; pencereyi, temayı ve imzalı uygulama güncellemesini kontrol edebilir. Masaüstü uçları Tauri uygulamasında kullanılabilir; yalnızca CLI ile açılan sunucuda `501` döner. `GET /capabilities` ile host desteğini denetleyin.

## Güvenlik modeli

- Dinleyici yalnızca `127.0.0.1` üzerine bağlanır; dışarıdan erişim için ayrı bir ters vekil veya tünel gerekir ve önerilmez.
- Her istek `Authorization: Bearer <jeton>` başlığı ister. Tek istisna `GET /api/v1/health`.
- Jeton `sb_` önekli 64 karakterlik rastgele bir dizedir. Diskte yalnızca SHA-256 özeti (`config/api-token.sha256`) saklanır; jetonun kendisi oluşturulduğu anda **bir kez** gösterilir.
- Yeni jeton oluşturmak eskisini anında geçersiz kılar. Jeton silinirse sağlık denetimi dışındaki istekler `401` alır. Jeton tüm yönetim işlemlerine yetki verir.
- İstek gövdesi 1 MB ile sınırlıdır (`413`). Aynı anda en fazla 8 istek işlenir; fazlası `503` alır.
- Her istek `Manager::contain` içinde çalışır: bir panik durumunda süreç çökmez, `500` döner ve uygulama "yeniden başlatma gerekli" durumuna geçer.
- API varsayılan olarak **kapalıdır**. Sol menü → API'den ya da `serverbond api serve` ile açılır.

## Açma ve jeton

Arayüz: **Sol menü → API** → "API'yi aç" anahtarı, port (varsayılan `18800`) ve **Jeton oluştur**. Kaydet düğmesi ayarı uygular; dinleyici ortam çalışırken de açılıp kapatılabilir.

API sayfası etkin bağlantı adresini, dinleyici/jeton durumunu, bağlantı örneğini ve tüm uç noktaları gösterir. Yol veya açıklamaya göre arama ve HTTP yöntemi filtresi kullanılabilir. Satırı açarak açıklamayı ve varsa JSON gövdesi şemasını inceleyin. **OpenAPI indir** dinleyici kapalıyken de sözleşmeyi verir; bu işlem jeton üretmez veya yenilemez. Tarayıcı önizlemesi aynı yol kataloğunu gösterir, yönetim işlemleri ve sözleşme indirme masaüstünde çalışır.

Komut satırı (masaüstü uygulaması kapalıyken; veri klasörü kilidi tek süreçlidir):

```powershell
serverbond api token          # yeni jeton üretir ve bir kez yazdırır
serverbond api serve [port]   # API'yi açar, bu süreçte dinler; Enter durdurur
serverbond api status         # enabled/port/listening/tokenSaved/baseUrl
serverbond api routes         # yol listesi
serverbond api schema         # OpenAPI JSON; masaüstü açıkken de çalışır, veri klasörünü açmaz
serverbond api forget         # jetonu siler
```

## Yanıt biçimi

Her yanıt JSON'dur. `GET /openapi.json` doğrudan OpenAPI 3.1 belgesini döndürür (Swagger/Postman gibi istemcilere aktarılabilir); diğer yanıtlar şu zarfı kullanır:

```json
{ "ok": true,  "data": … }
{ "ok": false, "error": "Türkçe hata metni" }
```

| Durum | Anlamı |
| --- | --- |
| `200` | İşlem tamamlandı; `data` sonucu içerir. |
| `202` | Çıkış, yeniden başlatma veya güncelleme kabul edildi; yanıt gönderildikten sonra uygulanır. Tamamlanma garantisi değildir. |
| `400` | Doğrulama veya işlem hatası. `error`, arayüzde görünen metnin aynısıdır ("Başka bir işlem devam ediyor…" dahil). |
| `401` | Jeton yok veya yanlış. |
| `404` | Yol bilinmiyor. `GET /api/v1` yol listesini verir. |
| `413` | Gövde 1 MB'ı aşıyor. |
| `500` | İşlem panikledi; uygulama yeniden başlatılmalı. |
| `501` | İşlem masaüstü hostu gerektiriyor; CLI sunucusunda desteklenmiyor. |
| `503` | Çok fazla eş zamanlı istek. |

Uzun işlemler (kurulum, `composer install`, sürüm) yanıt dönmeden önce tamamlanır. İstemci tarafında zaman aşımını buna göre (birkaç dakika) ayarlayın. Eş zamanlı durum sorguları engellenmez; her istek kendi iş parçacığında çalışır.

## Yollar

Ön ek: `http://127.0.0.1:<port>/api/v1`. `{id}` yerine proje **kimliği** (UUID) veya proje **adı** yazılabilir.

### Genel

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| GET | `/health` | Sürüm, `busy`, `anyRunning`, `recoveryIssue` (jeton gerekmez) |
| GET | `/` | Yol listesi |
| GET | `/openapi.json` | Kimlik doğrulamalı OpenAPI 3.1 sözleşmesi; doğrudan JSON belge |
| GET | `/capabilities` | `desktop`, `apiVersion`, kimlik doğrulama ve istek sınırları |
| GET | `/api` | Dinleyici ve jeton kayıt durumu |
| PUT | `/api` | `{ "enabled": true, "port": 18800 }`; yalnızca API ayarlarını değiştirir, diğer tercihler korunur |
| POST | `/api/token` | Yeni jetonu `{token}` olarak bir kez döndürür; mevcut jeton geçersiz olur |
| DELETE | `/api/token` | Jetonu iptal eder; yeniden erişim için arayüz/CLI'dan jeton üretin |
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
| POST | `/settings/validate` | Tam `Settings`; kaydetmeden doğrular ve döndürür |
| POST | `/recovery` | Önceki geçerli yapılandırmayı kurtarır; kurtarma yoksa mevcut ayarlara dokunmaz |

Sunucu çalışırken port ve sunucu ayarları değiştirilemez (`400`); klasör, açılış ve API tercihleri kaydedilebilir. `api.enabled` veya `api.port` değişirse dinleyici yeni ayara geçer; mevcut isteğin yanıtı açık bağlantı üzerinden gönderilir. Sonraki istekler için yeni portu kullanın. API kapatılmışsa arayüz/CLI'dan yeniden açın.

### Masaüstü, görünüm ve güncelleme

Bu uçlar için `capabilities.data.desktop` değeri `true` olmalıdır.

| Yöntem | Yol | Gövde / Sonuç |
| --- | --- | --- |
| GET | `/desktop` | `{preferences, autostart, issue, trayAvailable, quitting}` |
| PUT | `/desktop` | `{"preferences":{"closeToTray":true,"startMinimized":false},"autostart":false}` |
| GET | `/desktop/appearance` | `{theme}`: `system`, `light`, `dark`; ilk taşıma öncesinde `null` olabilir |
| PUT | `/desktop/appearance` | `{"theme":"dark"}`; kaydeder ve açık arayüze iletir |
| POST | `/desktop/show` | Pencereyi gösterir |
| POST | `/desktop/hide` | Pencereyi tepsiye gizler; tepsi yoksa hata döner |
| POST | `/desktop/menu` | Tepsi menüsünü açar |
| POST | `/desktop/navigate` | `{"page":"projects"}`; `overview`, `packages`, `projects`, `logs`, `services`, `api`, `settings`, `updates` |
| POST | `/desktop/exit` | `202`; servisleri durdurur ve uygulamadan çıkar |
| POST | `/desktop/restart` | `202`; servisleri durdurur ve uygulamayı yeniden başlatır |
| GET | `/updates` | İmzalı güncelleme kaynağını kontrol eder; `{available, currentVersion, version?, notes?}` |
| GET | `/updates/status` | API üzerinden başlatılmış güncelleme: `phase`, varsa `version`, `downloaded`, `total`, `error` |
| POST | `/updates/install` | `{"version":"1.2.0","confirm":true}`; belirtilen yayın sürümünü indirir, imzasını doğrular, `202` sonrasında kurucuyu çalıştırır |

Güncellemeden önce `POST /services/all/stop` çağırın ve çalışan işlemin tamamlanmasını bekleyin. Sürüm `GET /updates` çıktısıyla tam eşleşmelidir; keyfi URL kabul edilmez. İndirme boyunca istek açık kalır; birkaç dakikalık zaman aşımı kullanın ve başka bağlantıdan `/updates/status` sorgulayın. Durumlar `idle`, `downloading`, `ready`, `installing`, `failed` değerleridir. İndirme/imza hatası `400` döner. `202` sonrası API kapanır; kurulum hatası yerel uygulama günlüğüne ve arayüze bildirilir, bu durumda ServerBond'ı yeniden başlatın. Yeni süreçte durum `idle` olur; kurulum sonucunu `/health` sürümüyle denetleyin.

Tema `config/appearance.json` dosyasında saklanır. İlk arayüz açılışı mevcut localStorage tercihini taşır; önceden API ile kaydedilen tercih korunur. Masaüstü API'si uygulamayı kapatabilir; kapalı uygulamayı HTTP ile açamazsınız, bunun için Windows başlangıç ayarını veya uygulama kısayolunu kullanın.

### Windows ve kısayollar

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| GET | `/permissions` | Güvenlik duvarı/Defender izin durumu |
| POST | `/permissions/grant` | `{"defender":false}`; izinleri uygular, Windows UAC istemi çıkabilir |
| POST | `/permissions/ensure` | Daha önce kaydedilmiş izin tercihini uygular |
| POST | `/system/open-home` | ServerBond veri klasörünü açar |
| POST | `/system/runtime-download` | Visual C++ Runtime indirme sayfasını açar |
| POST | `/projects/{id}/open` | Projeyi varsayılan tarayıcıda açar |
| POST | `/projects/{id}/terminal` | Proje terminalini açar |
| POST | `/services/{id}/open` | `id`: `phpmyadmin` veya `mail` |

### Hizmetler ve PHP

| Yöntem | Yol | Açıklama |
| --- | --- | --- |
| GET | `/services` | `{id, name, actions, state}` nesneleri; desteklenen işlemleri `actions` üzerinden alın |
| GET | `/services/{id}` | Tek hizmetin aynı biçimde durumu |
| GET | `/php` | Katalogdaki tüm PHP sürümleri ve kurulum durumları |
| POST | `/services/{id}/{action}` | `id`: `all`, `php`, `mysql`, `caddy`, `composer`, `phpmyadmin`, `mail`, `postgres`, `redis`, `tunnel`, `node`. `action`: `install`, `repair`, `start`, `stop`, `restart`. |
| POST | `/php/{version}/select` | Varsayılan PHP sürümünü seçer (kurulu değilse indirir) |
| POST | `/php/{version}/repair` | PHP sürümünü onarır |

`node` ve `composer` yalnızca `install`/`repair` kabul eder. `phpmyadmin`: `install`/`repair`/`open`; `all`: `install`/`start`/`stop`/`restart`. `mail` ayrıca `open` destekler. `cloudflared`, `tunnel` için; `mailpit`, `mail` için takma addır. Desteklenmeyen işlem, başarılıymış gibi sessizce geçilmez (`400`); bilinmeyen hizmet sorgusu `404` döner. `restart`, `all` için tüm sunucuyu, tek hizmet için durdur-başlat sırasını uygular.

Cloudflared masaüstü uygulaması açıldığında eksikse arka planda SHA-256 doğrulanarak kurulur. Mevcut sağlıklı kurulum korunur; bozuk kurulum otomatik değiştirilmez, onarım istenir. İndirme başarısızsa uygulama çalışmaya devam eder; hata `GET /services/tunnel` ve `/logs/serverbond` üzerinden okunabilir. Sonraki açılışta eksik kurulum yeniden denenir. Kurulum tüneli başlatmaz ve jeton gerektirmez. Tünel başlatmak için token/auto-start uçlarını kullanın.

### Projeler

| Yöntem | Yol | Gövde / Açıklama |
| --- | --- | --- |
| GET | `/projects` | Durumlu proje listesi |
| POST | `/projects` | `{ "name": "magaza", "path": "C:\\\\dev\\\\magaza" }` — mevcut Laravel kökünü ekler |
| POST | `/projects/create` | `{ "name": "magaza", "parent": "C:\\\\dev" }` — Composer ile yeni Laravel |
| POST | `/projects/import` | `{ "url": "https://github.com/owner/repo", "name?": "…", "branch?": "main" }` — klonlar ve ekler |
| GET | `/github` | GitHub hesabı ve jeton kayıt durumu; jetonun kendisini içermez |
| POST | `/github/import` | `{ "repository": "owner/repo", "name": "magaza", "branch": "main" }`; `name`/`branch` isteğe bağlı, GitHub URL'si de kabul edilir |
| GET | `/projects/discover` | Çalışma alanındaki Laravel kökleri |
| POST | `/projects/import-folders` | `{ "paths": ["C:\\\\dev\\\\a", "C:\\\\dev\\\\b"] }` |
| GET | `/projects/{id}` | Tek proje |
| DELETE | `/projects/{id}` | Listeden kaldırır; dosyalar ve veritabanı korunur |
| POST | `/projects/{id}/php` | `{ "version": "8.3.33" }` |
| POST | `/projects/{id}/php/repair` | `{ "version": "8.3.33" }`; projenin PHP kurulumunu onarır |

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
| GET | `/mysql/credentials` | `{ "text": "…" }`; MySQL bağlantı parolası |
| GET | `/postgres/credentials` | `{ "text": "…" }`; PostgreSQL bağlantı parolası |
| POST | `/tunnel/token` | `{ "token": "…" }` |
| POST | `/tunnel/apply` | `{ "token": "…" }`; kaydeder ve tüneli başlatır |
| DELETE | `/tunnel/token` | — |
| POST | `/tunnel/auto-start` | `{ "autoStart": true }` |
| POST | `/github/token` | `{ "token": "ghp_…" }` |
| DELETE | `/github/token` | — |
| POST | `/https/{trust\|untrust}` | Yerel CA'yı Windows kullanıcı deposuna ekler/kaldırır |

Genel durum yanıtları parolaları ve jetonları içermez. Açıkça çağrılan `/mysql/credentials`, `/postgres/credentials` ve proje `/env` uçları sır içerebilir; `/api/token` yeni jetonu bir kez döndürür. HTTP yanıtları `Cache-Control: no-store` taşır. Bu yanıtları ve Authorization başlığını günlüklere yazmayın.

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

# Sözleşmeyi istemcinize aktarın (jeton dosyaya yazılmaz).
Invoke-RestMethod http://127.0.0.1:18800/api/v1/openapi.json -Headers $h |
  ConvertTo-Json -Depth 100 | Set-Content -Encoding UTF8 serverbond-openapi.json

# Temayı API üzerinden değiştirin.
Invoke-RestMethod -Method Put http://127.0.0.1:18800/api/v1/desktop/appearance `
  -Headers $h -ContentType "application/json" -Body '{"theme":"dark"}'
```

curl:

```bash
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:18800/api/v1/projects
curl -X PUT -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" \
  -d '{"content":"APP_ENV=production\nAPP_DEBUG=false\n"}' \
  http://127.0.0.1:18800/api/v1/projects/magaza/env
```

Tipik dağıtım betiği: `GET /health` → `POST /projects/{ad}/deploy` → HTTP durumunu ve `ok` alanını denetle. Başarı kaydı `data` içinde döner; hata halinde `error` okunur, ayrıntılı geçmiş `/projects/{ad}/releases` üzerinden alınır.

## Uygulama notları

- Kod: `crates/serverbond-core/src/api.rs`. Yönlendirme `route()` ve `project_route()` içindeki desen eşlemeleridir. `crates/serverbond-core/api-routes.json` hem Rust hem arayüz için ortak yol kataloğudur; yeni uç eklerken katalog ve `api/openapi.rs` şeması birlikte güncellenir. Settings, PHP profilleri, işçi, zamanlayıcı ve release gövdelerinin alan tipleri/varsayılanları serileştirilmiş modellerden gelir; port çakışması gibi alanlar arası kontroller yine Manager'da yapılır.
- `crates/serverbond-core/api-coverage.json` masaüstü komutları ve güncelleme eklentilerinin HTTP karşılıklarını listeler. `api_coverage` testi Tauri'nin kayıtlı komutlarıyla bu listeyi karşılaştırır ve her karşılığın OpenAPI sözleşmesinde bulunmasını zorunlu tutar. İç gezinme olayının tüketimi bir yönetim işlemi olmadığı için açıkça ayrılmıştır. Bu yapısal kontrol, gerçek HTTP davranış testleriyle birlikte çalışır.
- Masaüstü köprüsü: çekirdekte `DesktopApi`, Tauri'de `src-tauri/src/api.rs`; çekirdeğe Tauri bağımlılığı eklenmez. Arayüzdeki dosya seçimi/clipboard işlemleri API'de dosya yolu gövdeleri/JSON yanıtlarıyla karşılanır; pencerenin iç gezinme olayı ayrı bir sunucu işlemi değildir.
- Sunucu `tiny_http` üzerinde çalışır; kabul döngüsü `serverbond-api` iş parçacığında, her istek `serverbond-api-request` iş parçacığındadır. `ApiServer` düşürüldüğünde `unblock()` ile döngü kapanır ve iş parçacığı birleştirilir.
- `Manager::ensure_api` ayarı dinleyiciyle eşitler: açılışta (Tauri `setup`), her `save_settings` sonrasında ve API'nin kendi `PUT /settings` yolunda çağrılır. `shutdown()` dinleyiciyi kapatır.
- Testler: `crates/serverbond-core/tests/api.rs` geçici veri klasöründe gerçek dinleyici açar; sözleşmedeki uçların yetkilendirmesini, doğrulamanın ayarları değiştirmemesini, HTTP jeton yenilemesini, host yönlendirmesini ve yanıt sonrası işlemleri doğrular. Host test dublörüdür; gerçek UAC, kurucu ve pencere işlemlerini çalıştırmaz. Tauri birim testleri güncelleme onayını, gezinme sınırlarını ve tema taşımasını denetler.
