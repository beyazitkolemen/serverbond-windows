// Arayüz görüntülerini yeniler. Gerçek bir kurulum çalıştırmaz: üretim derlemesini
// başsız Chrome'da açar, salt okunur tarayıcı önizlemesine
// scripts/ai/screenshot-data.json örnek verisini enjekte eder ve her ekranı PNG
// olarak kaydeder. Windows penceresi, görev çubuğu ve gerçek süreç kimlikleri bu
// görüntülerde yoktur.
//
//   npm run build && node scripts/ai/screenshots.mjs
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { readFile, rm, stat, writeFile } from "node:fs/promises";
import { dirname, extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "../..");
const dist = join(root, "dist");
const output = join(root, "docs/screenshots");
const width = 1440;

const shots = [
  { file: "01-genel-bakis.png", nav: "Genel bakış", height: 1480 },
  { file: "02-bilesenler.png", nav: "Bileşenler", height: 1240 },
  {
    file: "03-projeler-kuyruk.png",
    nav: "Projeler",
    projectTab: "Kuyruklar",
    expand: [".project-worker-more summary"],
    height: 1400,
  },
  { file: "16-hizmetler.png", nav: "Hizmetler", height: 1100 },
  {
    file: "04-eposta.png",
    nav: "Hizmetler",
    service: "E-posta",
    openSettings: true,
    height: 1360,
  },
  {
    file: "05-tunel.png",
    nav: "Hizmetler",
    service: "Tünel",
    openSettings: true,
    height: 1400,
  },
  {
    file: "06-sistem.png",
    nav: "Ayarlar",
    tab: "Sistem",
    height: 2100,
  },
  { file: "07-php-ayarlari.png", nav: "Ayarlar", tab: "PHP", height: 1560 },
  {
    file: "11-postgresql.png",
    nav: "Hizmetler",
    service: "PostgreSQL",
    height: 1800,
  },
  {
    file: "15-redis.png",
    nav: "Hizmetler",
    service: "Redis",
    height: 1560,
  },
  {
    file: "12-github.png",
    nav: "Hizmetler",
    service: "GitHub",
    openSettings: true,
    height: 1180,
  },
  {
    file: "13-proje-github.png",
    nav: "Projeler",
    actions: [
      [".heading-actions .button.primary", "Proje ekle"],
      [".tabs button", "GitHub"],
    ],
    height: 1100,
  },
  {
    file: "08-web-https.png",
    nav: "Ayarlar",
    tab: "Web sunucusu",
    height: 1680,
  },
  {
    file: "09-proje-gunlukleri.png",
    nav: "Projeler",
    projectTab: "Günlükler",
    height: 1200,
  },
  {
    file: "10-proje-surum.png",
    nav: "Projeler",
    projectTab: "Sürüm",
    height: 1480,
  },
  {
    file: "14-proje-env.png",
    nav: "Projeler",
    projectTab: "Ortam",
    height: 1280,
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
      shot.openSettings ? [".service-gear", "Ayarlar"] : null,
      shot.tab ? [".settings-tabs button", shot.tab] : null,
      shot.projectTab ? [".project-detail-tabs button", shot.projectTab] : null,
      ...(Array.isArray(shot.actions) ? shot.actions : []),
      ...(Array.isArray(shot.expand)
        ? shot.expand.map((selector) => [selector, null])
        : shot.expand
          ? [[shot.expand, null]]
          : []),
    ].filter(Boolean),
  )};
  let index = 0;
  const advance = () => {
    if (index >= steps.length) return;
    const [selector, text] = steps[index];
    if (click(selector, text)) index += 1;
    setTimeout(advance, 100);
  };
  setTimeout(advance, 300);
});
</script>`;
  return html.replace("</head>", `${script}</head>`);
}

async function serve() {
  const server = createServer(async (request, response) => {
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

function capture(url, file, height) {
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
        `--user-data-dir=${join(root, "target/screenshot-profile")}`,
        `--window-size=${width},${height}`,
        "--virtual-time-budget=6000",
        `--screenshot=${file}`,
        url,
      ],
      { stdio: "ignore" },
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
const harnessFile = join(dist, "__screenshot.html");
try {
  for (const shot of shots) {
    await writeFile(harnessFile, harness(html, sample, shot));
    await capture(
      `http://127.0.0.1:${port}/__screenshot.html`,
      join(output, shot.file),
      shot.height,
    );
    console.log(`yazıldı: docs/screenshots/${shot.file}`);
  }
} finally {
  await rm(harnessFile, { force: true });
  server.close();
}
