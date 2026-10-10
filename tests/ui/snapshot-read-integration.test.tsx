import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Snapshot } from "../../src/types";
import sample from "../../scripts/ai/screenshot-data.json";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: async () => () => {} }));
vi.mock("../../src/updates", () => ({ checkForAppUpdate: async () => null }));
vi.mock("../../src/hooks/useTheme", () => ({
  useTheme: () => ({ error: "" }),
}));
import { snapshotRepository } from "../../src/repositories/snapshot";
import App from "../../src/App";

beforeEach(() => {
  invoke.mockReset();
});
afterEach(() => vi.useRealTimers());

it("shares concurrent polls but reads a new snapshot after a mutation", async () => {
  let completeOld!: (snapshot: Snapshot) => void;
  const old = new Promise<Snapshot>((resolve) => {
    completeOld = resolve;
  });
  const before = { busy: true } as Snapshot;
  const after = { busy: false } as Snapshot;
  invoke.mockReturnValueOnce(old).mockResolvedValueOnce(after);
  const poll = snapshotRepository.get();
  const concurrentPoll = snapshotRepository.get();
  expect(invoke).toHaveBeenCalledTimes(1);
  const afterMutation = snapshotRepository.get({ fresh: true });
  try {
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(await afterMutation).toBe(after);
  } finally {
    completeOld(before);
    await Promise.all([poll, concurrentPoll, afterMutation]);
  }
});

it("does not retain a failed snapshot promise for the next poll", async () => {
  invoke.mockRejectedValueOnce(new Error("temporary failure"));
  const next = { busy: false } as Snapshot;
  await expect(snapshotRepository.get()).rejects.toThrow("temporary failure");
  invoke.mockResolvedValueOnce(next);
  await expect(snapshotRepository.get()).resolves.toBe(next);
  expect(invoke).toHaveBeenCalledTimes(2);
});

it("paints the post-mutation snapshot without waiting for an older workspace poll", async () => {
  vi.useFakeTimers();
  const before = {
    ...sample,
    anyRunning: true,
    busy: false,
  } as unknown as Snapshot;
  const after = { ...before, anyRunning: false };
  let finishOld!: (snapshot: Snapshot) => void;
  const old = new Promise<Snapshot>((resolve) => {
    finishOld = resolve;
  });
  let reads = 0;
  invoke.mockImplementation((command) => {
    if (command === "snapshot")
      return ++reads === 2
        ? old
        : Promise.resolve(reads === 1 ? before : after);
    if (command === "desktop_navigation") return Promise.resolve(null);
    if (command === "service") return Promise.resolve();
    throw new Error(`Unexpected IPC command ${command}`);
  });
  render(<App />);
  await act(async () => {});
  await act(async () => {
    await vi.advanceTimersByTimeAsync(5000);
  });
  try {
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Sunucu durdur" }));
    });
    expect(reads).toBeGreaterThanOrEqual(3);
    expect(screen.getByRole("button", { name: "Sunucu başlat" })).toBeTruthy();
  } finally {
    await act(async () => {
      finishOld(before);
    });
  }
  expect(screen.getByRole("button", { name: "Sunucu başlat" })).toBeTruthy();
});
