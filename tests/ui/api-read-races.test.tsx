import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
import { call } from "../../src/api";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  invoke.mockReset();
  vi.useFakeTimers();
});
afterEach(() => vi.useRealTimers());

describe("timed out desktop reads", () => {
  it.each(["resolve", "reject"] as const)(
    "keeps sharing the replacement request when the original %ss late",
    async (result) => {
      const original = deferred<string>();
      const replacement = deferred<string>();
      invoke
        .mockReturnValueOnce(original.promise)
        .mockReturnValue(replacement.promise);
      const first = call<string>("read_log", { id: "php" }).catch(
        (error) => error,
      );
      await vi.advanceTimersByTimeAsync(10_000);
      expect(await first).toBeInstanceOf(Error);

      const second = call<string>("read_log", { id: "php" });
      if (result === "resolve") original.resolve("old log");
      else original.reject(new Error("old failed read"));
      await vi.advanceTimersByTimeAsync(0);
      const third = call<string>("read_log", { id: "php" });
      expect(invoke).toHaveBeenCalledTimes(2);

      replacement.resolve("current log");
      expect(await second).toBe("current log");
      expect(await third).toBe("current log");
    },
  );

  it("never times out or shares a mutation while the desktop operation is running", async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    invoke
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const settled = vi.fn();
    const save = call<string>("save_settings", { settings: {} });
    void save.then(settled);
    const anotherSave = call<string>("save_settings", { settings: {} });
    await vi.advanceTimersByTimeAsync(30_000);
    expect(invoke).toHaveBeenCalledTimes(2);
    expect(settled).not.toHaveBeenCalled();
    first.resolve("first save");
    second.resolve("second save");
    expect(await save).toBe("first save");
    expect(await anotherSave).toBe("second save");
  });
});
