# MCP bağlantısı

ServerBond, yerel proje, hizmet ve masaüstü yönetimini MCP araçları olarak sunar. API’nin bütün OpenAPI işlemleri aynı şemalarla araçlara dönüştürülür; iş kuralları, çalışma kilidi ve hata kontrolleri REST ile ortaktır. Bu özellik güncel `main` dalındadır; yayımlanmış v1.2.0 paketinde yoktur.

## Bağlanma

1. **API ve MCP → Bağlantı ve erişim** sayfasında API’yi ve **MCP erişimini aç** anahtarını etkinleştirip kaydedin.
2. API jetonu oluşturun. Mevcut jetonunuz varsa onu kullanın; yenilemek önceki bağlantıları geçersiz kılar.
3. **MCP bağlantısı → İstemci yapılandırması** örneğini kopyalayın. Yeni oluşturulan jeton bu örneğe eklenir; aksi durumda `<API_JETONU>` alanını doldurun.
4. HTTP MCP destekleyen istemcinize ekleyin. ServerBond açık kalmalıdır.

[Cursor](https://cursor.com/docs/mcp) gibi `mcpServers` yapılandırmasını kullanan istemciler için:

```json
{
  "mcpServers": {
    "serverbond": {
      "url": "http://127.0.0.1:18800/mcp",
      "headers": {
        "Authorization": "Bearer <API_JETONU>"
      }
    }
  }
}
```

Diğer istemcilerde **Streamable HTTP** seçin; aynı URL ve Authorization başlığını girin. Port değiştiğinde uygulamadaki güncel adresi kullanın. Yalnızca stdio veya eski HTTP+SSE destekleyen istemciler doğrudan bağlanamaz. Uzak/bulut istemcileri bilgisayarınızdaki `127.0.0.1` adresine erişemez.

## Araçlar

Araç adları HTTP yöntemi ve yoldan türetilir; yol parametreleri `by_` ile belirtilir. İstek gövdesi `body` alanında, yol ve sorgu parametreleri üst düzeyde taşınır.

| İşlem           | Araç                                    | Argüman                                                           |
| --------------- | --------------------------------------- | ----------------------------------------------------------------- |
| Sunucu durumu   | `serverbond_get_status`                 | `{}`                                                              |
| Hizmet listesi  | `serverbond_get_services`               | `{}`                                                              |
| Hizmet başlatma | `serverbond_post_services_by_id_start`  | `{"id":"redis"}`                                                  |
| Proje listesi   | `serverbond_get_projects`               | `{}`                                                              |
| Proje ekleme    | `serverbond_post_projects`              | `{"body":{"name":"magaza","path":"C:\\ServerBond\\www\\magaza"}}` |
| Proje günlüğü   | `serverbond_get_projects_by_id_logs`    | `{"id":"magaza","source":"php"}`                                  |
| Sürüm dağıtımı  | `serverbond_post_projects_by_id_deploy` | `{"id":"magaza"}`                                                 |

PHP sürümleri, `.env`, kuyruk/zamanlayıcı, sürüm tarifleri, veritabanı yedekleme/geri yükleme, GitHub, tünel, API/jeton, uygulama ayarları ve güncellemeler de araç kataloğundadır. Güncel liste ve JSON şemaları `tools/list` ile alınır. Ayar nesnelerini değiştirmeden önce ilgili okuma aracından mevcut tam değerleri alın.

`serverbond_get_capabilities` masaüstü host desteğini gösterir. CLI ile sunulan API’de pencere/tema/güncelleme araçları `501` işlem hatası döndürür; Tauri uygulamasında aynı araçlar masaüstü köprüsünü kullanır. Uygulamayı kapatan veya yükleyiciyi başlatan işlemler, MCP yanıtı gönderildikten sonra uygulanır.

## Erişim ve çalışma biçimi

- API ve MCP ayrı tercihlerdir; MCP için ikisi de açık olmalıdır. Önceki ayarlarda MCP kapalı kabul edilir.
- Dinleyici yalnızca `127.0.0.1` üzerindedir. API ile aynı Bearer jetonu, port, 1 MB gövde ve 8 eşzamanlı istek sınırını kullanır. Her MCP isteği yeniden doğrulanır.
- Jeton tüm yönetim işlemlerine erişir; bazı okuma araçları parola veya `.env` içeriği döndürebilir. İstemci yapılandırmasını depoya eklemeyin, istemcinin araç onaylarını açık tutun.
- Host ve Origin yerel adresle sınırlandırılır. Farklı siteler için CORS izni verilmez. Tarayıcıdan gelen `null` Origin reddedilir.
- Argümanlar JSON Schema ile doğrulanır. Bilinmeyen araç/parametre hataları JSON-RPC hatası; iş kuralı hataları `isError: true` araç sonucudur. Sonuç hem `content` hem `structuredContent.data` içinde bulunur.
- İşlem günlüğüne yalnızca araç adı ve başarı durumu yazılır; MCP argümanları/sonuçları kaydedilmez.
- Uzun kurulum/dağıtım işlemleri eşzamanlıdır. İstemci zaman aşımını artırın; bağlantı kesilmesi veya iptal bildirimi başlamış işlemi geri almaz. Durumu `serverbond_get_status` ile takip edin, yazma isteğini körlemesine tekrarlamayın.

## Protokol ve doğrulama

[MCP Streamable HTTP](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) kullanılır: `/mcp` üzerinde POST ve JSON yanıtları, oturumsuz çalışma. `initialize`, `ping`, `tools/list`, `tools/call` ve bildirim kabulü desteklenir. Protokol sürümleri: `2025-06-18`, `2025-11-25`. SSE akışı, kaynak/istem şablonları, görevler ve stdio taşıması ilan edilmez; GET ve DELETE `405` döndürür. Bildirimlere boş `202` yanıtı verilir.

POST isteklerinde `Content-Type: application/json`, `Accept: application/json, text/event-stream` ve anlaşılmış `MCP-Protocol-Version` başlığı kullanılır. Desteklenmeyen sürüm başlığı `400` döndürür.

```powershell
cargo test -p serverbond-core --test api mcp::
npm run test:mcp
```

İlk komut gerçek yerel HTTP istekleriyle yetkilendirme, şema, proje/hizmet işlemleri, erişim kısıtları ve ertelenmiş masaüstü yanıtlarını test eder. İkincisi resmi TypeScript MCP SDK’sını geçici veri klasöründe çalışan Rust API’sine bağlar. Kullanıcının kurulumuna veya hizmetlerine dokunmaz. Windows ve Linux CI işlerinde çalışır.
