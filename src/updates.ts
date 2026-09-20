import { desktop } from "./api";
import { APP_VERSION } from "./version";

export type UpdateInfo = {
  version: string;
  notes: string;
  currentVersion: string;
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

export async function checkForAppUpdate(): Promise<UpdateInfo | null> {
  if (!desktop) return null;
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    notes: update.body?.trim() ?? "",
    currentVersion: update.currentVersion || APP_VERSION,
  };
}

export async function installAppUpdate(
  onProgress?: (progress: UpdateProgress) => void,
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
  let downloaded = 0;
  let total = 0;
  await update.downloadAndInstall((event) => {
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
  await relaunch();
}
