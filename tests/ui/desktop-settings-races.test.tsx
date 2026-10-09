import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import type { Run } from "../../src/types";
const call = vi.hoisted(() => vi.fn());
vi.mock("../../src/api", () => ({ desktop: true, call }));
import DesktopSettings from "../../src/components/DesktopSettings";

const original = {
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
afterEach(() => vi.useRealTimers());

it.each(["resolve", "reject"] as const)(
  "keeps saved Windows preferences when a pre-save poll %ss late",
  async (result) => {
    let finishPoll!: (value: typeof original) => void;
    let failPoll!: (error: Error) => void;
    const oldPoll = new Promise<typeof original>((resolve, reject) => {
      finishPoll = resolve;
      failPoll = reject;
    });
    call.mockReset();
    call
      .mockResolvedValueOnce(original)
      .mockReturnValueOnce(oldPoll)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({ ...original, autostart: true });
    vi.useFakeTimers();
    render(<DesktopSettings busy={false} run={run} />);
    await act(async () => {});
    const autostart = screen.getByRole("checkbox", {
      name: "Windows oturumu açıldığında ServerBond'ı çalıştır",
    }) as HTMLInputElement;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(call).toHaveBeenCalledTimes(2);
    fireEvent.click(autostart);
    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "Masaüstü tercihlerini kaydet" }),
      );
    });
    expect(call).toHaveBeenCalledWith("desktop_save", {
      preferences: original.preferences,
      autostart: true,
    });
    expect(autostart.checked).toBe(true);
    await act(async () => {
      if (result === "resolve") finishPoll(original);
      else failPoll(new Error("old desktop poll failed"));
    });
    expect(autostart.checked).toBe(true);
    expect(autostart.closest("fieldset")?.disabled).toBe(false);
    expect(screen.queryByRole("alert")).toBeNull();
  },
);
