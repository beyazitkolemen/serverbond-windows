import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import type { ApiStatus, Run } from "../../src/types";
const apiService = vi.hoisted(() => ({
  status: vi.fn(),
  createToken: vi.fn(),
  forgetToken: vi.fn(),
}));
vi.mock("../../src/services", () => ({ apiService }));
import ApiSettings from "../../src/components/ApiSettings";

const status: ApiStatus = {
  enabled: false,
  port: 18800,
  listening: false,
  tokenSaved: false,
  baseUrl: "http://127.0.0.1:18800/api/v1",
  mcpEnabled: false,
  mcpUrl: "http://127.0.0.1:18800/mcp",
};
const run: Run = async (_label, action) => {
  await action();
  return true;
};
beforeEach(() => {
  apiService.status.mockReset();
  apiService.createToken.mockReset();
});

it.each(["resolve", "reject"] as const)(
  "does not replace the token's fresh status when an older poll %ss",
  async (result) => {
    let finishOld!: (value: ApiStatus) => void;
    let failOld!: (error: Error) => void;
    const oldStatus = new Promise<ApiStatus>((resolve, reject) => {
      finishOld = resolve;
      failOld = reject;
    });
    apiService.status
      .mockReturnValueOnce(oldStatus)
      .mockResolvedValue({ ...status, tokenSaved: true });
    apiService.createToken.mockResolvedValue("new-secret");
    render(
      <ApiSettings
        values={{ enabled: false, port: 18800, mcpEnabled: false }}
        onChange={() => {}}
        busy={false}
        run={run}
        dirty={false}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Jeton oluştur" }));
    await screen.findByRole("button", { name: "Jetonu yenile" });
    await act(async () => {
      if (result === "resolve") finishOld(status);
      else failOld(new Error("old poll failed"));
    });
    await waitFor(() =>
      expect(
        (
          screen.getByRole("button", {
            name: "Jetonu sil",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false),
    );
    expect(screen.getByRole("button", { name: "Jetonu yenile" })).toBeTruthy();
  },
);
