import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ReleasePane from "../../src/components/ProjectRelease";
import { call } from "../../src/api";
import {
  defaultRelease,
  type Project,
  type ReleaseRecord,
  type Run,
} from "../../src/types";

vi.mock("../../src/api", () => ({ call: vi.fn(), desktop: false }));
const project = {
  id: "demo",
  name: "demo",
  release: defaultRelease(),
} as Project;
const run: Run = async (_label, action) => {
  await action();
  return true;
};
function record(sha: string, startedAt: string): ReleaseRecord {
  return {
    sha,
    startedAt,
    branch: sha,
    durationMs: 3000,
    success: true,
    output: `${sha} output`,
  } as ReleaseRecord;
}
beforeEach(() => vi.mocked(call).mockReset());

describe("release output identity", () => {
  it("shows the selected historical record's branch and SHA alongside its output", async () => {
    const latest = record("new-sha", "today");
    const older = record("old-sha", "yesterday");
    vi.mocked(call).mockResolvedValue([latest, older]);
    const { container } = render(
      <ReleasePane project={project} busy={false} run={run} />,
    );
    await screen.findByText("new-sha output");
    fireEvent.click(screen.getByRole("button", { name: "yesterday" }));
    const output = container.querySelector(".release-console")!;
    expect(output.textContent).toContain("old-sha · old-sha · 3 sn");
    expect(output.textContent).toContain("old-sha output");
    expect(output.textContent).not.toContain("new-sha");
    expect(screen.getByText("Son sürüm başarılı · today")).toBeTruthy();
  });

  it("keeps the completed deployment output when an older initial history read arrives late", async () => {
    let finishInitial!: (records: ReleaseRecord[]) => void;
    const deployed = record("deployed", "today");
    vi.mocked(call)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishInitial = resolve;
          }),
      )
      .mockResolvedValueOnce(deployed)
      .mockResolvedValueOnce([deployed]);
    render(<ReleasePane project={project} busy={false} run={run} />);
    expect(screen.getByText("Sürüm geçmişi okunuyor…")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Çalıştır" }));
    await screen.findByText("deployed output");
    await waitFor(() => expect(call).toHaveBeenCalledTimes(3));
    await act(async () => finishInitial([record("stale", "yesterday")]));
    expect(screen.queryByText("stale output")).toBeNull();
    expect(screen.getByText("deployed output")).toBeTruthy();
    expect(screen.getByText("Son sürüm başarılı · today")).toBeTruthy();
  });

  it("keeps a completed deployment output even if the new history read fails", async () => {
    let finishInitial!: (records: ReleaseRecord[]) => void;
    vi.mocked(call)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finishInitial = resolve;
          }),
      )
      .mockResolvedValueOnce(record("deployed", "today"))
      .mockRejectedValueOnce(new Error("history unavailable"));
    render(<ReleasePane project={project} busy={false} run={run} />);
    fireEvent.click(screen.getByRole("button", { name: "Çalıştır" }));
    await screen.findByText("Sürüm uygulandı ancak geçmiş yenilenemedi.");
    await act(async () => finishInitial([record("stale", "yesterday")]));
    expect(screen.getByText("deployed output")).toBeTruthy();
    expect(screen.queryByText("stale output")).toBeNull();
  });
});
