// Arayüz görüntülerini yeniler. Gerçek bir kurulum çalıştırmaz: üretim derlemesini
// başsız Chrome'da açar, salt okunur tarayıcı önizlemesine
// scripts/ai/screenshot-data.json örnek verisini enjekte eder ve her ekranı PNG
// olarak kaydeder. Windows penceresi, görev çubuğu ve gerçek süreç kimlikleri bu
// görüntülerde yoktur.
//
//   npm run build && node scripts/ai/screenshots.mjs
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import {
  basename,
  dirname,
  extname,
  join,
  normalize,
  resolve,
} from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const dist = join(root, "dist");
const output = join(root, "docs/screenshots");
const width = 1440;
let captureStatus;
let captureHtml;

const shots = [
  { file: "17-proje-detay.png", nav: "Projeler" },
  {
    file: "18-api.png",
    viewportOnly: true,
    nav: "API ve MCP",
    actions: [['.api-page [role="tab"]', "Uç nokta rehberi"]],
  },
  { file: "19-koyu-tema.png", nav: "Projeler", theme: "dark" },
  { file: "01-genel-bakis.png", nav: "Genel bakış" },
  { file: "02-bilesenler.png", nav: "Bileşenler" },
  {
    file: "03-projeler-kuyruk.png",
    nav: "Projeler",
    projectTab: "Kuyruklar",
    expand: [".project-worker-more summary"],
  },
  { file: "16-hizmetler.png", nav: "Hizmetler" },
  {
    file: "04-eposta.png",
    nav: "Hizmetler",
    service: "E-posta",
    openSettings: true,
  },
  {
    file: "05-tunel.png",
    nav: "Hizmetler",
    service: "Tünel",
    openSettings: true,
  },
  {
    file: "06-sistem.png",
    nav: "Ayarlar",
    tab: "Sistem",
  },
  { file: "07-php-ayarlari.png", nav: "Ayarlar", tab: "PHP" },
  {
    file: "11-postgresql.png",
    nav: "Hizmetler",
    service: "PostgreSQL",
  },
  {
    file: "15-redis.png",
    nav: "Hizmetler",
    service: "Redis",
  },
  {
    file: "12-github.png",
    nav: "Hizmetler",
    service: "GitHub",
    openSettings: true,
  },
  {
    file: "13-proje-github.png",
    nav: "Projeler",
    actions: [
      [".heading-actions .button.primary", "Proje ekle"],
      [".tabs button", "GitHub"],
      [".tabs button", "Adres ile ekle"],
    ],
  },
  {
    file: "08-web-https.png",
    nav: "Ayarlar",
    tab: "Web sunucusu",
  },
  {
    file: "09-proje-gunlukleri.png",
    nav: "Projeler",
    projectTab: "Günlükler",
  },
  {
    file: "10-proje-surum.png",
    nav: "Projeler",
    projectTab: "Sürüm",
  },
  {
    file: "14-proje-env.png",
    nav: "Projeler",
    projectTab: "Ortam",
  },
];

const types = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
};

function harness(html, sample, shot) {
  const script = `<script>
globalThis.__SERVERBOND_PREVIEW__ = ${JSON.stringify(sample)};
addEventListener("DOMContentLoaded", () => {
  const click = (selector, text) => {
    const nodes = [...document.querySelectorAll(selector)];
    const target = text
      ? nodes.find((node) => node.textContent.trim() === text)
      : nodes[0];
    if (target) target.click();
    return Boolean(target);
  };
  const steps = ${JSON.stringify(
    [
      [".nav-item", shot.nav],
      shot.service
        ? [`.service-row[data-service="${shot.service}"]`, null]
        : null,
      shot.openSettings
        ? ['.services-workspace [role="tab"]', "Ayarlar"]
        : null,
      shot.tab ? [".settings-tabs button", shot.tab] : null,
      shot.projectTab
        ? ['.project-detail-nav [role="tab"]', shot.projectTab]
        : null,
      ...(Array.isArray(shot.actions) ? shot.actions : []),
      ...(Array.isArray(shot.expand)
        ? shot.expand.map((selector) => [selector, null])
        : shot.expand
          ? [[shot.expand, null]]
          : []),
    ].filter(Boolean),
  )};
  let index = 0;
  let attempts = 0;
  const advance = () => {
    if (index >= steps.length) {
      document.documentElement.dataset.theme = ${JSON.stringify(shot.theme ?? "light")};
      fetch('/__screenshot-status', {method: 'POST', body: JSON.stringify({ok: true, steps: index, height: document.documentElement.scrollHeight})});
      return;
    }
    const [selector, text] = steps[index];
    if (click(selector, text)) { index += 1; attempts = 0; }
    else if (++attempts > 30) {
      fetch('/__screenshot-status', {method: 'POST', body: JSON.stringify({ok: false, selector, text})});
      return;
    }
    setTimeout(advance, 100);
  };
  setTimeout(advance, 300);
});
</script>`;
  return html.replace(
    "</head>",
    `<style>*,*::before,*::after{animation:none!important;transition:none!important}</style>${script}</head>`,
  );
}

async function serve() {
  const server = createServer(async (request, response) => {
    // Keep the preview harness out of dist so a desktop build can never
    // accidentally embed fixture data while screenshots are being captured.
    if (request.url === "/__screenshot.html") {
      response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
      response.end(captureHtml);
      return;
    }
    if (request.url === "/__screenshot-status" && request.method === "POST") {
      let body = "";
      for await (const chunk of request) body += chunk;
      captureStatus = JSON.parse(body);
      response.writeHead(204).end();
      return;
    }
    const path = normalize(new URL(request.url, "http://x").pathname);
    const file = join(dist, path === "/" ? "index.html" : path);
    try {
      if (!file.startsWith(dist) || !(await stat(file)).isFile()) {
        throw new Error("yok");
      }
      response.writeHead(200, {
        "content-type": types[extname(file)] ?? "application/octet-stream",
      });
      response.end(await readFile(file));
    } catch {
      response.writeHead(404).end("yok");
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { server, port: server.address().port };
}

function capture(url, file, height, profile) {
  return new Promise((resolve, reject) => {
    const chrome = spawn(
      process.env.CHROME ?? "/usr/bin/google-chrome-stable",
      [
        "--no-sandbox",
        "--headless",
        "--disable-gpu",
        "--disable-dev-shm-usage",
        "--no-first-run",
        "--hide-scrollbars",
        "--force-device-scale-factor=1",
        `--user-data-dir=${profile}`,
        `--window-size=${width},${height}`,
        "--virtual-time-budget=6000",
        `--screenshot=${file}`,
        url,
      ],
      { stdio: "ignore", windowsHide: true },
    );
    chrome.on("error", reject);
    chrome.on("exit", (code) =>
      code === 0 ? resolve() : reject(new Error(`Chrome çıkış kodu ${code}`)),
    );
  });
}

const sample = JSON.parse(
  await readFile(join(root, "scripts/ai/screenshot-data.json"), "utf8"),
);
delete sample.note;
const html = await readFile(join(dist, "index.html"), "utf8").catch(() => {
  throw new Error("dist/index.html yok. Önce npm run build çalıştırın.");
});

const { server, port } = await serve();
const profile = await mkdtemp(join(tmpdir(), "serverbond-screenshots-"));
try {
  for (const shot of shots) {
    captureStatus = undefined;
    captureHtml = harness(html, sample, shot);
    await capture(
      `http://127.0.0.1:${port}/__screenshot.html`,
      join(output, shot.file),
      1000,
      profile,
    );
    if (!captureStatus?.ok)
      throw new Error(
        `${shot.file}: ekran açılamadı: ${JSON.stringify(captureStatus)}`,
      );
    if (!shot.viewportOnly && captureStatus.height > 1000) {
      const height = Math.ceil(captureStatus.height);
      captureStatus = undefined;
      await capture(
        `http://127.0.0.1:${port}/__screenshot.html`,
        join(output, shot.file),
        height,
        profile,
      );
      if (!captureStatus?.ok)
        throw new Error(`${shot.file}: tam sayfa görüntüsü doğrulanamadı`);
    }
    if (!(await stat(join(output, shot.file))).size)
      throw new Error(`${shot.file}: boş görüntü`);
    console.log(`yazıldı: docs/screenshots/${shot.file}`);
  }
} finally {
  server.close();
  if (
    dirname(resolve(profile)) !== resolve(tmpdir()) ||
    !basename(profile).startsWith("serverbond-screenshots-")
  ) {
    throw new Error("Geçici tarayıcı profili beklenen klasörün dışında.");
  }
  await rm(profile, {
    recursive: true,
    force: true,
    maxRetries: 5,
    retryDelay: 200,
  });
}
