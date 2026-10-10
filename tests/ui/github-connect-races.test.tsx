import { act, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import GithubConnect from "../../src/components/GithubConnect";
import { githubService } from "../../src/services";
import type { GithubAuthFlow } from "../../src/types";

vi.mock("../../src/services", () => ({
  githubService: {
    saveClientId: vi.fn(),
    authStart: vi.fn(),
    authPoll: vi.fn(),
    authCancel: vi.fn(),
    authOpen: vi.fn(),
  },
}));
const flow: GithubAuthFlow = {
  flowId: "flow-a",
  userCode: "CODE1234",
  verificationUri: "https://github.com/login/device",
  expiresAt: Math.floor(Date.now() / 1000) + 600,
  interval: 5,
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(githubService.authOpen).mockResolvedValue();
});

it("starts one GitHub device flow when clicks arrive before a busy render", async () => {
  const finishes: Array<(value: GithubAuthFlow) => void> = [];
  vi.mocked(githubService.authStart).mockImplementation(
    () =>
      new Promise((resolve) => {
        finishes.push(resolve);
      }),
  );
  render(
    <GithubConnect
      github={{ tokenSaved: false, login: null, oauthClientId: "client" }}
    />,
  );
  const button = screen.getByRole("button", { name: "GitHub ile giriş yap" });
  act(() => {
    button.click();
    button.click();
  });
  try {
    expect(githubService.authStart).toHaveBeenCalledTimes(1);
  } finally {
    await act(async () => {
      finishes.forEach((finish) => finish(flow));
    });
  }
});

it("does not duplicate cancellation and unlocks retry after cancellation fails", async () => {
  vi.mocked(githubService.authStart).mockResolvedValue(flow);
  let fail!: (reason: Error) => void;
  vi.mocked(githubService.authCancel)
    .mockImplementationOnce(
      () =>
        new Promise((_, reject) => {
          fail = reject;
        }),
    )
    .mockResolvedValue();
  render(
    <GithubConnect
      github={{ tokenSaved: false, login: null, oauthClientId: "client" }}
    />,
  );
  await act(async () => {
    screen.getByRole("button", { name: "GitHub ile giriş yap" }).click();
  });
  const cancel = screen.getByRole("button", { name: "Girişi iptal et" });
  act(() => {
    cancel.click();
    cancel.click();
  });
  expect(githubService.authCancel).toHaveBeenCalledTimes(1);
  await act(async () => {
    fail(new Error("temporary cancellation failure"));
  });
  expect(screen.getByRole("alert").textContent).toContain(
    "temporary cancellation failure",
  );
  await act(async () => {
    cancel.click();
  });
  expect(githubService.authCancel).toHaveBeenCalledTimes(2);
  expect(
    screen.getByRole("button", { name: "GitHub ile giriş yap" }),
  ).toBeTruthy();
});

it("allows a new login attempt after a failed start", async () => {
  vi.mocked(githubService.authStart)
    .mockRejectedValueOnce(new Error("temporary login failure"))
    .mockResolvedValueOnce(flow);
  render(
    <GithubConnect
      github={{ tokenSaved: false, login: null, oauthClientId: "client" }}
    />,
  );
  await act(async () => {
    screen.getByRole("button", { name: "GitHub ile giriş yap" }).click();
  });
  await act(async () => {
    screen.getByRole("button", { name: "GitHub ile giriş yap" }).click();
  });
  expect(githubService.authStart).toHaveBeenCalledTimes(2);
  expect(screen.getByText("CODE1234")).toBeTruthy();
});
