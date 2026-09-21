# Ekran görüntüleri

Güncel `main` arayüzü, 21 Eylül 2026. Görüntüler 1440 piksel genişlikte uygulamanın salt okunur tarayıcı önizlemesinden alınmıştır. Proje adları, portlar ve süreç bilgileri örnek veridir; Windows pencere çerçevesi dahil değildir. Yayımlanmış v1.2.0 paketi bu arayüz değişikliklerinin tamamını içermez.

Kaynak sürüm: [`a14159c`](https://github.com/beyazitkolemen/serverbond-windows/tree/a14159c). Görüntüler `C:\ServerBond` veri klasörünü ve `C:\ServerBond\www` varsayılan proje yolunu gösterir; farklı konumdaki mevcut projeler de korunur.

## Genel bakış

Sunucu durumu, bileşenler ve projeler.

![Genel bakış](01-genel-bakis.png)

## Proje detayı

Solda aramalı proje seçici ve bölüm menüsü; sağda proje bilgileri ve işlemler.

![Proje detayı](17-proje-detay.png)

## Hizmetler

Her hizmet tek satırda; durum ve ayarlar ayrı bölümlerde.

![Hizmetler](16-hizmetler.png)

## API

Bağlantı ve erişim yönetimi ile aranabilir uç nokta rehberi.

![API uç nokta rehberi](18-api.png)

## Koyu tema

![Koyu temada proje detayı](19-koyu-tema.png)

## Diğer ekranlar

<details>
<summary>Bileşenler</summary>

![Bileşenler](02-bilesenler.png)

</details>

<details>
<summary>Kuyruk işçileri</summary>

![Kuyruk işçileri](03-projeler-kuyruk.png)

</details>

<details>
<summary>Sürüm tarifi ve geçmişi</summary>

![Sürüm tarifi ve geçmişi](10-proje-surum.png)

</details>

<details>
<summary>Ortam değişkenleri</summary>

![Ortam değişkenleri](14-proje-env.png)

</details>

<details>
<summary>Proje günlükleri</summary>

![Proje günlükleri](09-proje-gunlukleri.png)

</details>

<details>
<summary>E-posta ayarları</summary>

![E-posta ayarları](04-eposta.png)

</details>

<details>
<summary>Cloudflare tüneli ayarları</summary>

![Cloudflare tüneli ayarları](05-tunel.png)

</details>

<details>
<summary>PostgreSQL</summary>

![PostgreSQL](11-postgresql.png)

</details>

<details>
<summary>Redis</summary>

![Redis](15-redis.png)

</details>

<details>
<summary>GitHub hesabı</summary>

![GitHub hesabı](12-github.png)

</details>

<details>
<summary>GitHub’dan proje ekleme</summary>

![GitHub’dan proje ekleme](13-proje-github.png)

</details>

<details>
<summary>Sistem ayarları</summary>

![Sistem ayarları](06-sistem.png)

</details>

<details>
<summary>PHP ayarları</summary>

![PHP ayarları](07-php-ayarlari.png)

</details>

<details>
<summary>Web sunucusu ve HTTPS</summary>

![Web sunucusu ve HTTPS](08-web-https.png)

</details>

## Görüntüleri yenileme

Chrome kurulu bir geliştirme ortamında:

```sh
npm run build
node scripts/ai/screenshots.mjs
```

Windows’ta Chrome yolu gerekirse PowerShell ile belirtilir:

```powershell
$env:CHROME = "C:\Program Files\Google\Chrome\Application\chrome.exe"
node scripts/ai/screenshots.mjs
```

Betik üretim derlemesini `scripts/ai/screenshot-data.json` örnek verisiyle açar. Menü adımlarından biri bulunamazsa işlemi hata ile durdurur; gerçek servisleri veya kullanıcı projelerini çalıştırmaz.
