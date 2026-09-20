import { invoke, isTauri } from "@tauri-apps/api/core";
import type { Snapshot } from "./types";
import catalog from "../crates/f4box-core/catalog.json";
import phpVersions from "../crates/f4box-core/php-versions.json";
import tools from "../crates/f4box-core/tools.json";

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
  logs: ["F4Box hazır. Kurulum başlatılabilir."],
  home: "%LOCALAPPDATA%\\F4Box",
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
      displayErrors: true,
      logErrors: true,
      opcache: false,
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
    },
    phpmyadmin: { enabled: true, language: "tr", rows: 25, loginSeconds: 1440 },
    tunnel: { autoStart: false },
    projectsDir: "",
    backupsDir: "",
    startOnLaunch: false,
  },
  tunnel: {
    version: tools[0].version,
    installed: false,
    running: false,
    pid: null,
    tokenSaved: false,
    autoStart: false,
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
  },
  busy: false,
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
        "settings_previous",
        "settings_defaults",
        "desktop_status",
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
          timer = setTimeout(
            () =>
              reject(
                new Error(
                  "Uygulama yanıt vermiyor. Bağlantı yeniden denenecek; çalışan bir işlemi tekrar başlatmayın.",
                ),
              ),
            10_000,
          );
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }
  if (command === "snapshot") {
    // scripts/ai/screenshots.mjs renders the interface with sample data.
    const sample = (globalThis as { __F4BOX_PREVIEW__?: Snapshot })
      .__F4BOX_PREVIEW__;
    return structuredClone(sample ?? preview) as T;
  }
  if (command === "read_log")
    return "Günlükler masaüstü uygulamasında görüntülenir." as T;
  throw new Error(
    "Bu işlem için F4Box masaüstü uygulamasını açın. Tarayıcı görünümü yalnızca önizlemedir.",
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
