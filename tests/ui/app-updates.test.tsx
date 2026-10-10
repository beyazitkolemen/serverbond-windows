import { beforeEach, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { installAppUpdate } from "../../src/updates";
import UpdateSettings from "../../src/components/UpdateSettings";

const updater = vi.hoisted(() => ({ check: vi.fn(), relaunch: vi.fn() }));
vi.mock("../../src/api", () => ({
  desktop: true,
  call: async () => ({ available: false }),
}));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: updater.check }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: updater.relaunch }));
beforeEach(() => {
  vi.resetAllMocks();
});

function update(version = "1.3.99") {
  return {
    version,
    download: vi.fn().mockResolvedValue(undefined),
    install: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

it("never interrupts services or installs a release whose version changed", async () => {
  const current = update("1.4.0");
  updater.check.mockResolvedValue(current);
  const interrupt = vi.fn();
  await expect(
    installAppUpdate("1.3.99", undefined, interrupt),
  ).rejects.toThrow("Yayın sürümü değişti");
  expect(current.download).not.toHaveBeenCalled();
  expect(interrupt).not.toHaveBeenCalled();
  expect(current.install).not.toHaveBeenCalled();
  expect(updater.relaunch).not.toHaveBeenCalled();
  expect(current.close).toHaveBeenCalledOnce();
});

it("a failed download or signature verification leaves services running", async () => {
  const current = update();
  current.download.mockRejectedValue(
    new Error("signature verification failed"),
  );
  updater.check.mockResolvedValue(current);
  const interrupt = vi.fn();
  await expect(
    installAppUpdate(current.version, undefined, interrupt),
  ).rejects.toThrow("signature verification failed");
  expect(interrupt).not.toHaveBeenCalled();
  expect(current.install).not.toHaveBeenCalled();
  expect(updater.relaunch).not.toHaveBeenCalled();
  expect(current.close).toHaveBeenCalledOnce();
});

it("does not install when stopping services fails", async () => {
  const current = update();
  updater.check.mockResolvedValue(current);
  const interrupt = vi.fn().mockRejectedValue(new Error("service stop failed"));
  await expect(
    installAppUpdate(current.version, undefined, interrupt),
  ).rejects.toThrow("service stop failed");
  expect(current.download).toHaveBeenCalledOnce();
  expect(current.install).not.toHaveBeenCalled();
  expect(updater.relaunch).not.toHaveBeenCalled();
  expect(current.close).toHaveBeenCalledOnce();
});

it("installs only after verification and reports progress before restarting", async () => {
  const current = update();
  const stages: string[] = [];
  const progress = vi.fn();
  current.download.mockImplementation(async (notify) => {
    stages.push("download-verified");
    notify({ event: "Started", data: { contentLength: 100 } });
    notify({ event: "Progress", data: { chunkLength: 100 } });
  });
  current.install.mockImplementation(async () => {
    stages.push("install");
  });
  current.close.mockImplementation(async () => {
    stages.push("close");
  });
  updater.check.mockResolvedValue(current);
  updater.relaunch.mockImplementation(async () => {
    stages.push("relaunch");
  });
  await installAppUpdate(current.version, progress, async () => {
    stages.push("stop-services");
  });
  expect(stages).toEqual([
    "download-verified",
    "stop-services",
    "install",
    "relaunch",
    "close",
  ]);
  expect(progress).toHaveBeenLastCalledWith({ downloaded: 100, total: 100 });
});

it("offers an installer link for unsigned releases without automatic installation", () => {
  const installerUrl =
    "https://github.com/beyazitkolemen/serverbond-windows/releases/download/v1.3.99/ServerBond_1.3.99_x64-setup.exe";
  render(
    <UpdateSettings
      busy={false}
      running={false}
      run={async () => true}
      available={{
        version: "1.3.99",
        currentVersion: "1.3.98",
        notes: "Manual release",
        installMode: "manual",
        releaseUrl:
          "https://github.com/beyazitkolemen/serverbond-windows/releases/tag/v1.3.99",
        installerUrl,
      }}
      onAvailable={() => {}}
      openUpdates={0}
    />,
  );
  expect(
    screen
      .getByRole("link", { name: "1.3.99 kurulumunu indir" })
      .getAttribute("href"),
  ).toBe(installerUrl);
  expect(
    screen.queryByRole("button", { name: /sürümünü kur ve yeniden başlat/ }),
  ).toBeNull();
  expect(screen.getByText(/Bu yayın elle kurulur/)).toBeTruthy();
  expect(updater.check).not.toHaveBeenCalled();
});
