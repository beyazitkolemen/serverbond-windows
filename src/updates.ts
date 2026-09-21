import { call, desktop } from "./api";

export type UpdateInfo = {
  version: string;
  notes: string;
  currentVersion: string;
  installMode: "automatic" | "manual";
  releaseUrl: string;
  installerUrl: string | null;
};

export type UpdateProgress = {
  downloaded: number;
  total: number;
};

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

let pendingCheck: Promise<UpdateInfo | null> | null = null;

export async function checkForAppUpdate(): Promise<UpdateInfo | null> {
  if (!desktop) return null;
  // Launch, settings and tray may request the same check concurrently.
  if (!pendingCheck) {
    pendingCheck = call<UpdateInfo & { available: boolean }>("app_update_check")
      .then((result) => (result.available ? result : null))
      .finally(() => {
        pendingCheck = null;
      });
  }
  return pendingCheck;
}

export async function installAppUpdate(
  expectedVersion: string,
  onProgress?: (progress: UpdateProgress) => void,
  beforeInstall?: () => Promise<void>,
): Promise<void> {
  if (!desktop) {
    throw new Error("Güncelleme masaüstü uygulamasında kullanılabilir.");
  }
  const { check } = await import("@tauri-apps/plugin-updater");
  const { relaunch } = await import("@tauri-apps/plugin-process");
  const update = await check();
  if (!update) {
    throw new Error("Yüklenecek güncelleme bulunamadı.");
  }
  try {
    if (update.version !== expectedVersion) {
      throw new Error("Yayın sürümü değişti. Güncellemeyi yeniden denetleyin.");
    }
    let downloaded = 0;
    let total = 0;
    await update.download((event) => {
      switch (event.event) {
        case "Started":
          total = event.data.contentLength ?? 0;
          onProgress?.({ downloaded, total });
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          onProgress?.({ downloaded, total });
          break;
        default:
          break;
      }
    });
    // Download and signature verification finish before interrupting services.
    await beforeInstall?.();
    await update.install();
    await relaunch();
  } finally {
    await update.close().catch(() => undefined);
  }
}
