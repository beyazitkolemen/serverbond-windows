import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CloudSettings from "../../src/components/CloudSettings";
import { call } from "../../src/api";
import type { CloudStatus } from "../../src/services/cloud";

vi.mock("../../src/api", () => ({ call: vi.fn(), desktop: true }));
const code = "ABCDEF0123456789ABCDEF0123456789";
const initial: CloudStatus = {
  paired: false,
  defaultUrl: "https://serverbond.on-forge.com",
  connection: "disconnected",
  socketEndpoint: null,
  url: null,
  name: null,
  account: null,
  lastContact: null,
  error: null,
};
const paired: CloudStatus = {
  ...initial,
  paired: true,
  connection: "connecting",
  url: initial.defaultUrl,
  name: "Ofis",
  account: "Hesabım",
};
beforeEach(() => {
  vi.mocked(call).mockReset().mockResolvedValue(initial);
});
async function ready() {
  render(<CloudSettings />);
  await screen.findByText("Eşleştirme bekleniyor");
  return screen.getByLabelText("Bağlantı kodu") as HTMLInputElement;
}
describe("one-code Cloud pairing", () => {
  it("pairs with only the normalized pasted code and waits for a real connection", async () => {
    const field = await ready();
    fireEvent.paste(field, {
      clipboardData: {
        getData: () => "  abcdef01-23456789\nabcdef01-23456789  ",
      },
    });
    expect(field.value).toBe(code);
    expect(document.querySelector("details")?.open).toBe(false);
    vi.mocked(call).mockImplementation(async (command) =>
      command === "cloud_status" ? paired : undefined,
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Cloud’a bağlan", exact: true }),
    );
    await screen.findByText("Ofis");
    expect(call).toHaveBeenCalledWith("cloud_pair", { code });
    expect(screen.getByText("Cloud’a bağlanıyor")).toBeTruthy();
    expect(screen.queryByText("Cloud’a bağlı")).toBeNull();
  });
  it("keeps an existing custom server visible and sends its address", async () => {
    const url = "https://cloud.example.com";
    vi.mocked(call).mockResolvedValue({ ...initial, url });
    const field = await ready();
    expect(screen.getByText(/Kayıtlı özel sunucu:/).textContent).toContain(url);
    fireEvent.change(field, { target: { value: code } });
    fireEvent.click(
      screen.getByRole("button", { name: "Cloud’a bağlan", exact: true }),
    );
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("cloud_pair", { code, url }),
    );
  });
  it("blocks incomplete codes and duplicate submissions while pairing", async () => {
    const field = await ready();
    fireEvent.change(field, { target: { value: "abc" } });
    const button = screen.getByRole("button", {
      name: "Cloud’a bağlan",
      exact: true,
    }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    fireEvent.change(field, { target: { value: code } });
    let finish!: () => void;
    vi.mocked(call).mockImplementation((command) =>
      command === "cloud_pair"
        ? new Promise<void>((resolve) => {
            finish = resolve;
          })
        : Promise.resolve(paired),
    );
    fireEvent.submit(field.form!);
    fireEvent.submit(field.form!);
    expect(
      vi
        .mocked(call)
        .mock.calls.filter(([command]) => command === "cloud_pair"),
    ).toHaveLength(1);
    await act(async () => finish());
    await screen.findByText("Ofis");
  });
  it("does not call successful pairing a failure when status cannot be read", async () => {
    const field = await ready();
    fireEvent.change(field, { target: { value: code } });
    vi.mocked(call).mockImplementation(async (command) => {
      if (command === "cloud_status") throw new Error("read failed");
    });
    fireEvent.submit(field.form!);
    await screen.findByText(
      "İşlem tamamlandı. Bağlantı durumu yeniden denetleniyor.",
    );
    expect(field.value).toBe("");
    expect(screen.queryByText("read failed")).toBeNull();
  });
  it("opens advanced settings when a custom URL is invalid", async () => {
    await ready();
    const field = screen.getByLabelText("Cloud sunucusu") as HTMLInputElement;
    fireEvent.change(field, { target: { value: "invalid" } });
    expect(field.checkValidity()).toBe(false);
    expect(field.closest("details")?.open).toBe(true);
  });
});
