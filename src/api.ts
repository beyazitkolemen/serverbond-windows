import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Snapshot } from "./types";
import catalog from "../crates/serverbond-core/catalog.json";
import phpVersions from "../crates/serverbond-core/php-versions.json";
import tools from "../crates/serverbond-core/tools.json";
import apiRoutes from "../crates/serverbond-core/api-routes.json";

const toolVersion = (id: string) =>
  tools.find((tool) => tool.id === id)?.version ?? "";

export const desktop = isTauri();

// Browser preview is explicitly read-only; installation always runs in the Rust desktop process.
const preview: Snapshot = {
  phpVersions: phpVersions.map((p) => ({
    ...p,
    installed: false,
    running: false,
    pid: null,
    repairable: false,
    issue: null,
  })),
  packages: catalog.map((p) => ({
    ...p,
    installed: false,
    running: false,
    pid: null,
    repairable: false,
    issue: null,
  })),
  projects: [],
  logs: ["ServerBond hazır. Kurulum başlatılabilir."],
  home: "C:\\ServerBond",
  settings: {
    webPort: 8088,
    mysqlPort: 13306,
    phpPort: 19000,
    php: {
      timezone: "Europe/Istanbul",
      memoryMb: 512,
      uploadMb: 64,
      postMb: 64,
      executionSeconds: 120,
      inputSeconds: -1,
      inputVars: 1000,
      displayErrors: false,
      logErrors: true,
      opcache: true,
      opcacheMb: 128,
      extensions: [
        "curl",
        "fileinfo",
        "mbstring",
        "openssl",
        "pdo_mysql",
        "mysqli",
        "sodium",
        "pdo_sqlite",
        "sqlite3",
        "intl",
        "zip",
      ],
      extraIni: "",
    },
    phpVersions: {},
    mysql: {
      bufferPoolMb: 128,
      maxConnections: 151,
      maxPacketMb: 64,
      waitSeconds: 28800,
      collation: "utf8mb4_unicode_ci",
      sqlMode:
        "ONLY_FULL_GROUP_BY,STRICT_TRANS_TABLES,NO_ZERO_IN_DATE,NO_ZERO_DATE,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION",
      slowQueryLog: false,
      longQuerySeconds: 10,
    },
    web: {
      hostPattern: "{name}.localhost",
      compression: false,
      accessLog: false,
      readSeconds: 120,
      connectSeconds: 3,
      https: false,
      httpsPort: 8443,
    },
    phpmyadmin: { enabled: true, language: "tr", rows: 25, loginSeconds: 1440 },
    tunnel: { autoStart: false },
    mail: {
      smtpPort: 1025,
      webPort: 8025,
      autoStart: false,
      relayPhpMail: true,
      maxMessages: 500,
    },
    postgres: { port: 15432, autoStart: false },
    redis: { port: 16379, autoStart: false },
    api: { enabled: false, port: 18800, mcpEnabled: false },
    projectsDir: "",
    backupsDir: "",
    startOnLaunch: false,
  },
  tunnel: {
    version: toolVersion("cloudflared"),
    installed: false,
    repairable: false,
    running: false,
    pid: null,
    tokenSaved: false,
    autoStart: false,
    issue: null,
  },
  mail: {
    version: toolVersion("mailpit"),
    installed: false,
    repairable: false,
    running: false,
    pid: null,
    smtpPort: 1025,
    webPort: 8025,
    autoStart: false,
    relayPhpMail: true,
    issue: null,
  },
  postgres: {
    version: toolVersion("postgres"),
    installed: false,
    repairable: false,
    running: false,
    pid: null,
    port: 15432,
    autoStart: false,
    passwordSaved: false,
    issue: null,
  },
  redis: {
    version: toolVersion("redis"),
    installed: false,
    repairable: false,
    running: false,
    pid: null,
    port: 16379,
    autoStart: false,
    issue: null,
  },
  github: {
    tokenSaved: false,
    login: null,
  },
  node: {
    version: toolVersion("node"),
    installed: false,
    repairable: false,
    directory: null,
    issue: null,
  },
  permissions: {
    granted: false,
    appliedAt: null,
    applied: [],
    failed: [],
    programs: [],
    defenderExclusion: false,
    pending: [],
    helper: false,
    declined: false,
  },
  busy: false,
  installProgress: null,
  anyRunning: false,
  recoveryIssue: null,
  restartRequired: false,
};

const pendingReads = new Map<string, Promise<unknown>>();

export async function call<T = void>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (desktop) {
    // Only read operations time out in the UI. A mutation may still be running
    // in Rust: never unlock its button or suggest retrying on a UI timeout.
    if (
      ![
        "snapshot",
        "read_log",
        "read_project_log",
        "read_project_worker_log",
        "read_project_schedule_log",
        "list_project_schedule",
        "list_failed_jobs",
        "settings_previous",
        "settings_defaults",
        "desktop_status",
        "api_status",
        "api_documentation",
        "discover_projects",
        "list_project_releases",
        "project_git_status",
        "read_project_env",
      ].includes(command)
    ) {
      return invoke<T>(command, args);
    }
    const key = JSON.stringify([command, args]);
    let pending = pendingReads.get(key);
    if (!pending) {
      pending = invoke<T>(command, args);
      pendingReads.set(key, pending);
      void pending.then(
        () => pendingReads.delete(key),
        () => pendingReads.delete(key),
      );
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        pending as Promise<T>,
        new Promise<never>((_, reject) => {
          timer = setTimeout(() => {
            // Forget the stalled invoke so the next poll issues a fresh request
            // instead of re-attaching to a promise that may never settle.
            if (pendingReads.get(key) === pending) pendingReads.delete(key);
            reject(
              new Error(
                "Uygulama yanıt vermiyor. Bağlantı yeniden denenecek; çalışan bir işlemi tekrar başlatmayın.",
              ),
            );
          }, 10_000);
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }
  if (command === "snapshot") {
    // scripts/ai/screenshots.mjs renders the interface with sample data.
    const sample = (globalThis as { __SERVERBOND_PREVIEW__?: Snapshot })
      .__SERVERBOND_PREVIEW__;
    return structuredClone(sample ?? preview) as T;
  }
  if (command === "read_project_log") {
    const source = String(args?.source ?? "php");
    if (source === "schedule") {
      return [
        "[2026-09-20 21:15:00] Running scheduled command: inspire",
        "[2026-09-20 21:15:00] Processed:  inspire",
        "[2026-09-20 21:16:00] Running scheduled command: app:prune-tokens",
      ].join("\n") as T;
    }
    if (source.startsWith("worker:")) {
      return [
        "[2026-09-20 21:15:05] Processing: App\\Jobs\\SendOrderMail",
        "[2026-09-20 21:15:06] Processed:  App\\Jobs\\SendOrderMail",
        "[2026-09-20 21:15:12] Processing: App\\Jobs\\IndexProduct",
        "[2026-09-20 21:15:13] Processed:  App\\Jobs\\IndexProduct",
      ].join("\n") as T;
    }
    return [
      "[20-Sep-2026 21:15:02 Europe/Istanbul] NOTICE: PHP 8.4.25 Development Server started",
      "127.0.0.1:19012 [21:15:03] GET /",
      "127.0.0.1:19012 [21:15:04] GET /css/app.css",
      '[20-Sep-2026 21:15:08 Europe/Istanbul] PHP Warning:  Undefined array key "page" in app/Http/Controllers/CatalogController.php on line 42',
      "127.0.0.1:19012 [21:15:11] GET /siparisler",
    ].join("\n") as T;
  }
  if (
    command === "read_log" ||
    command === "read_project_worker_log" ||
    command === "read_project_schedule_log"
  )
    return "Günlükler masaüstü uygulamasında görüntülenir." as T;
  if (command === "list_project_schedule")
    return "  * * * * *  php artisan inspire  Next Due: 1 minute from now" as T;
  if (command === "list_failed_jobs") return "Başarısız kuyruk işi yok." as T;
  if (command === "project_git_status")
    return {
      present: true,
      branch: "main",
      sha: "6f12c4a",
    } as T;
  if (command === "api_status")
    return {
      enabled: false,
      port: 18800,
      listening: false,
      tokenSaved: false,
      baseUrl: "http://127.0.0.1:18800/api/v1",
      mcpEnabled: false,
      mcpUrl: "http://127.0.0.1:18800/mcp",
    } as T;
  if (command === "api_documentation")
    return { routes: apiRoutes, document: null } as T;
  if (command === "read_project_env")
    return {
      exists: true,
      content: [
        "APP_NAME=Magaza",
        "APP_ENV=production",
        "APP_DEBUG=false",
        "APP_URL=http://magaza.localhost:8088",
        "",
        "DB_CONNECTION=mysql",
        "DB_HOST=127.0.0.1",
        "DB_PORT=13306",
        "DB_DATABASE=magaza",
        "DB_USERNAME=root",
        "DB_PASSWORD=",
      ].join("\n"),
      example: "APP_NAME=Laravel\nAPP_ENV=local\n",
    } as T;
  if (command === "list_project_releases")
    return [
      {
        startedAt: "2026-09-20 21:18:04",
        durationMs: 18420,
        branch: "main",
        sha: "6f12c4a",
        success: true,
        output: [
          "--- git ---",
          "Already up to date.",
          "--- composer ---",
          "Nothing to install, update or remove",
          "--- migrate ---",
          "Nothing to migrate.",
          "--- optimize ---",
          "Cached events cleared successfully.",
          "--- config:cache ---",
          "Configuration cached successfully.",
          "--- jobs ---",
          "Yeniden başlatıldı: kuyruk: default, zamanlayıcı",
        ].join("\n"),
      },
    ] as T;
  throw new Error(
    "Bu işlem için ServerBond masaüstü uygulamasını açın. Tarayıcı görünümü yalnızca önizlemedir.",
  );
}

export async function chooseFolder(): Promise<string | null> {
  if (!desktop)
    throw new Error("Klasör seçimi masaüstü uygulamasında kullanılabilir.");
  const { open } = await import("@tauri-apps/plugin-dialog");
  return open({
    directory: true,
    multiple: false,
    title: "Proje klasörünü seçin",
  });
}

export async function chooseSqlFile(): Promise<string | null> {
  if (!desktop)
    throw new Error("Dosya seçimi masaüstü uygulamasında kullanılabilir.");
  const { open } = await import("@tauri-apps/plugin-dialog");
  return open({
    directory: false,
    multiple: false,
    title: "SQL yedeğini seçin",
    filters: [{ name: "SQL", extensions: ["sql"] }],
  });
}
