# GitHub bağlantısı

**Hizmetler → GitHub** veya **Proje ekle → GitHub** üzerinden hesabınızı bağlayın. **GitHub ile giriş yap** bir cihaz kodu oluşturur ve GitHub doğrulama sayfasını açar. Ekrandaki kodu GitHub’da onayladığınızda erişim jetonu otomatik kaydedilir. Client Secret kullanılmaz.

## Bir defalık OAuth ayarı

ServerBond için bir [GitHub OAuth App](https://github.com/settings/developers) oluşturun ve uygulama ayarlarında **Enable Device Flow** seçeneğini açın. Uygulamanın **Client ID** değerini ServerBond’daki **OAuth uygulama ayarı** alanına girin. Bu değer gizli değildir; Client Secret veya erişim jetonunu bu alana yazmayın. ServerBond yönlendirme callback’i kullanmaz.

Dağıtıcı, derleme sırasında `SERVERBOND_GITHUB_CLIENT_ID` ortam değişkeniyle varsayılan Client ID sağlayabilir. Kullanıcının uygulamada kaydettiği ayar bu varsayılanın önüne geçer. Hazır bir Client ID içermeyen derlemede ilk girişten önce bu alan doldurulmalıdır.

GitHub’ın [Device Flow belgesi](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow), etkinleştirme ve kullanıcı onayı ayrıntılarını açıklar. Özel depolara erişmek için `repo` kapsamı istenir. Organizasyon politikaları veya SSO için ayrıca GitHub onayı gerekebilir.

## Depo ve dal seçimi

- Bağlı hesabın kendi depoları, ortak çalıştığı depolar ve erişebildiği organizasyon depoları listelenir. Genel/özel, arşiv, fork ve dil bilgileri gösterilir.
- Depolar son güncellenme sırasıyla 50’şer yüklenir. Arama ve hesap/organizasyon filtresi **yüklenmiş depolara** uygulanır; sonraki sayfalar **Daha fazla depo** ile alınır.
- Seçilen deponun varsayılan dalı otomatik seçilir. Dal araması, korumalı dal etiketi ve 100’lük sayfalama desteklenir.
- **Adres ile ekle** ile `owner/repo`, GitHub adresi veya desteklenen başka bir HTTPS Git adresi kullanılabilir. GitHub dalları **Dalları getir** ile listelenir; dal adı elle de girilebilir. Genel GitHub depoları bağlantısız da kullanılabilir.

Giriş iptal edilebilir. Kod süresi dolarsa veya izin reddedilirse yeniden giriş başlatın. Hesap değiştirme sırasında yeni giriş tamamlanana kadar mevcut bağlantı korunur. **Bağlantıyı kaldır** yerel kaydı siler; GitHub tarafındaki uygulama yetkisini iptal etmek için GitHub → Settings → Applications bölümünü kullanın.

## Jeton saklama ve uyumluluk

Erişim ve varsa yenileme jetonu Windows DPAPI ile `config/github-connection.dpapi` dosyasında şifrelenir. Süreli jetonlar kullanım sırasında yenilenir; yenileme süresi dolduğunda tekrar giriş gerekir. Jetonlar arayüze, API durum yanıtına veya MCP araç sonucuna gönderilmez. Eski `github-token.dpapi` kayıtları ve kişisel erişim jetonuyla bağlantı desteklenmeye devam eder.

İnce ayarlı kişisel jeton kullanılıyorsa seçilen depolarda Contents okuma yetkisi gerekir. Klasik kişisel jetonda özel depolar için `repo` gerekir. Jeton Git komutuna URL’ye özel Basic kimlik doğrulama başlığı olarak verilir; klon adresine yazılmaz. Bu kullanım GitHub’ın [resmî checkout uygulamasıyla](https://github.com/actions/checkout/blob/main/src/git-auth-helper.ts) uyumludur. REST API istekleri ayrı olarak Bearer kullanır.

## API ve MCP

Tüm yollar `/api/v1` altındadır ve ServerBond API Bearer jetonu gerektirir. Bu jeton GitHub jetonundan ayrıdır.

| İşlem | Yol | Parametre |
| --- | --- | --- |
| GET | `/github` | Bağlantı durumu, hesap, Client ID, yöntem ve varsa son geçerlilik |
| PUT | `/github/auth/settings` | `{ "clientId": "OAuth_App_Client_ID" }` |
| POST | `/github/auth/start` | Kod, `flowId`, `expiresAt` ve saniye cinsinden `interval` döner |
| POST | `/github/auth/poll` | `{ "flowId": "…" }`; `pending`, `connected`, `denied`, `expired`, `cancelled` |
| POST | `/github/auth/cancel` | `{ "flowId": "…" }` |
| POST | `/github/auth/open` | Yerel varsayılan tarayıcıda GitHub doğrulamasını açar |
| GET | `/github/repositories?page=1` | `repositories`, varsa `nextPage` |
| GET | `/github/branches?repository=owner/repo&page=1` | `branches`, `defaultBranch`, varsa `nextPage` |

İlk anket için `interval`, devamında `retryAfter` kadar bekleyin. Sayfa numarası 1–10000 arasındadır. Giriş akışı uygulama belleğinde tutulur; yeniden başlatma sonrasında tekrar başlatılmalıdır. Aynı anda tek giriş akışı geçerlidir.

MCP etkinse bu işlemler OpenAPI’den üretilen `serverbond_*` araçlarında da bulunur. Örneğin `serverbond_get_github_repositories` için `{ "page": 1 }`, `serverbond_get_github_branches` için `{ "repository": "owner/repo", "page": 1 }` kullanılır. Kullanıcı GitHub onayını kendi tarayıcısında verir.
