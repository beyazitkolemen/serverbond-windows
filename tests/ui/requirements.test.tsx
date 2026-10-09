import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import Requirements from "../../src/components/Requirements";
import { call } from "../../src/api";
import type { Requirement, Run } from "../../src/types";

vi.mock("../../src/api", () => ({ call: vi.fn(), desktop: true }));
const missing: Requirement = {
  id: "vc-runtime",
  label: "Visual C++ çalışma zamanı",
  status: "error",
  detail: "Eksik",
  helpUrl: "https://aka.ms/vs/17/release/vc_redist.x64.exe",
};
const run: Run = async (_label, action) => {
  try {
    await action();
  } catch {
    /* App displays mutation errors. */
  }
};
beforeEach(() => vi.mocked(call).mockReset());

it("installs through IPC and rechecks the actual runtime before showing ready", async () => {
  let installed = false;
  vi.mocked(call).mockImplementation(async (command) => {
    if (command === "install_windows_runtime") {
      installed = true;
      return;
    }
    return [{ ...missing, status: installed ? "ok" : "error" }];
  });
  render(<Requirements busy={false} run={run} />);
  fireEvent.click(await screen.findByRole("button", { name: "Otomatik kur" }));
  await screen.findByText("Hazır");
  expect(call).toHaveBeenCalledWith("install_windows_runtime");
  expect(screen.queryByRole("button", { name: "Otomatik kur" })).toBeNull();
});

it("rechecks after a declined installation and leaves retry available", async () => {
  vi.mocked(call).mockImplementation(async (command) => {
    if (command === "install_windows_runtime") throw Error("UAC declined");
    return [missing];
  });
  render(<Requirements busy={false} run={run} />);
  fireEvent.click(await screen.findByRole("button", { name: "Otomatik kur" }));
  await waitFor(() =>
    expect(
      vi.mocked(call).mock.calls.filter(([name]) => name === "requirements"),
    ).toHaveLength(2),
  );
  expect(screen.queryByText("Hazır")).toBeNull();
  expect(screen.getByRole("button", { name: "Otomatik kur" })).toBeTruthy();
});
