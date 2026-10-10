import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import GithubRepositoryPicker from "../../src/components/GithubRepositoryPicker";
import { githubService } from "../../src/services";
import type { GithubBranchPage } from "../../src/types";

vi.mock("../../src/services", () => ({
  githubService: { branches: vi.fn(), repositories: vi.fn() },
}));
vi.mock("../../src/api", () => ({ desktop: false, call: vi.fn() }));
beforeEach(() => {
  vi.mocked(githubService.branches).mockReset();
  vi.mocked(githubService.repositories).mockReset();
  vi.mocked(githubService.repositories).mockResolvedValue({
    repositories: [],
    nextPage: null,
  });
});

describe("GitHub branch request mode", () => {
  it("allows a manual branch request after leaving an unfinished automatic request", async () => {
    let finishAutomatic!: (value: GithubBranchPage) => void;
    vi.mocked(githubService.branches)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishAutomatic = resolve;
          }),
      )
      .mockResolvedValueOnce({
        branches: [{ name: "main", protected: false }],
        defaultBranch: "main",
        nextPage: null,
      });
    render(
      <GithubRepositoryPicker
        github={{ tokenSaved: true, login: "demo" }}
        repository="demo/repo"
        branch=""
        disabled={false}
        onRepository={vi.fn()}
        onBranch={vi.fn()}
      />,
    );
    await waitFor(() =>
      expect(githubService.branches).toHaveBeenCalledTimes(1),
    );
    expect(
      (
        screen.getByRole("button", {
          name: "Dalları getir",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Adres ile ekle" }));
    const request = screen.getByRole("button", {
      name: "Dalları getir",
    }) as HTMLButtonElement;
    expect(request.disabled).toBe(false);
    expect(screen.queryByText("Dallar yükleniyor…")).toBeNull();
    fireEvent.click(request);
    await screen.findByRole("option", { name: "main · Varsayılan" });
    await act(async () =>
      finishAutomatic({
        branches: [{ name: "stale", protected: false }],
        defaultBranch: "stale",
        nextPage: null,
      }),
    );
    expect(screen.queryByRole("option", { name: /stale/ })).toBeNull();
    expect(githubService.branches).toHaveBeenCalledTimes(2);
  });
});
