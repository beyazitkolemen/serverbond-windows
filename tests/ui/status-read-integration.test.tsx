import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ApiStatus, Run } from "../../src/types";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
import ApiSettings from "../../src/components/ApiSettings";
import DesktopSettings from "../../src/components/DesktopSettings";

const apiStatus: ApiStatus = {
  enabled: false,
  port: 18800,
  listening: false,
  tokenSaved: false,
  baseUrl: "http://127.0.0.1:18800/api/v1",
  mcpEnabled: false,
  mcpUrl: "http://127.0.0.1:18800/mcp",
};
const desktopStatus = {
  preferences: { closeToTray: true, startMinimized: false },
  autostart: false,
  issue: null,
  trayAvailable: true,
  quitting: false,
};
const run: Run = async (_label, action) => {
  await action();
  return true;
};
beforeEach(() => {
  invoke.mockReset();
});
afterEach(() => vi.useRealTimers());

it("API token creation issues a fresh real IPC read instead of joining the pre-mutation poll", async () => {
  let finishOld!: (status: ApiStatus) => void;
  const old = new Promise<ApiStatus>((resolve) => {
    finishOld = resolve;
  });
  let reads = 0;
  invoke.mockImplementation((command) => {
    if (command === "api_status")
      return ++reads === 1
        ? old
        : Promise.resolve({ ...apiStatus, tokenSaved: true });
    if (command === "api_token") return Promise.resolve("new-secret");
    throw new Error(`Unexpected IPC command ${command}`);
  });
  render(
    <ApiSettings
      values={{ enabled: false, port: 18800, mcpEnabled: false }}
      onChange={() => {}}
      busy={false}
      run={run}
      dirty={false}
    />,
  );
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "Jeton oluştur" }));
  });
  try {
    expect(reads).toBe(2);
    expect(screen.getByRole("button", { name: "Jetonu yenile" })).toBeTruthy();
  } finally {
    await act(async () => {
      finishOld(apiStatus);
    });
  }
  expect(
    (screen.getByRole("button", { name: "Jetonu sil" }) as HTMLButtonElement)
      .disabled,
  ).toBe(false);
});

it("Windows preference save issues a fresh real IPC read and ignores the earlier poll", async () => {
  vi.useFakeTimers();
  let finishOld!: (status: typeof desktopStatus) => void;
  const old = new Promise<typeof desktopStatus>((resolve) => {
    finishOld = resolve;
  });
  let reads = 0;
  invoke.mockImplementation((command) => {
    if (command === "desktop_status") {
      reads++;
      return reads === 2
        ? old
        : Promise.resolve({ ...desktopStatus, autostart: reads > 2 });
    }
    if (command === "desktop_save") return Promise.resolve();
    throw new Error(`Unexpected IPC command ${command}`);
  });
  render(<DesktopSettings busy={false} run={run} />);
  await act(async () => {});
  await act(async () => {
    await vi.advanceTimersByTimeAsync(3000);
  });
  const checkbox = screen.getByRole("checkbox", {
    name: "Windows oturumu açıldığında ServerBond'ı çalıştır",
  }) as HTMLInputElement;
  fireEvent.click(checkbox);
  await act(async () => {
    fireEvent.click(
      screen.getByRole("button", { name: "Masaüstü tercihlerini kaydet" }),
    );
  });
  try {
    expect(reads).toBe(3);
  } finally {
    await act(async () => {
      finishOld(desktopStatus);
    });
  }
  expect(checkbox.checked).toBe(true);
  expect(
    screen.queryByRole("button", { name: "Değişiklikleri iptal et" }),
  ).toBeNull();
});
