import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
import ReleasePane from "../../src/components/ProjectRelease";
import {
  defaultRelease,
  type Project,
  type ReleaseRecord,
  type Run,
} from "../../src/types";

describe("release history IPC integration", () => {
  it("requests fresh history after deploy instead of sharing the pending pre-deployment read", async () => {
    let finishOld!: (records: ReleaseRecord[]) => void;
    let finishFresh!: (records: ReleaseRecord[]) => void;
    const old = new Promise<ReleaseRecord[]>((resolve) => {
      finishOld = resolve;
    });
    const fresh = new Promise<ReleaseRecord[]>((resolve) => {
      finishFresh = resolve;
    });
    const deployed: ReleaseRecord = {
      sha: "new-sha",
      branch: "main",
      startedAt: "today",
      durationMs: 1000,
      success: true,
      output: "new deployment output",
    };
    let reads = 0;
    invoke.mockImplementation((command: string) => {
      if (command === "list_project_releases")
        return ++reads === 1 ? old : fresh;
      if (command === "deploy_project") return Promise.resolve(deployed);
      throw new Error(`Unexpected IPC: ${command}`);
    });
    const run: Run = async (_label, action) => {
      await action();
      return true;
    };
    const project = {
      id: "history-integration",
      name: "demo",
      release: defaultRelease(),
    } as Project;
    render(<ReleasePane project={project} busy={false} run={run} />);
    fireEvent.click(screen.getByRole("button", { name: "Çalıştır" }));
    await screen.findByText("new deployment output");
    await waitFor(() => expect(reads).toBe(2));
    await act(async () => finishFresh([deployed]));
    expect(screen.getByText("Son sürüm başarılı · today")).toBeTruthy();
    await act(async () =>
      finishOld([
        {
          ...deployed,
          sha: "old-sha",
          startedAt: "yesterday",
          output: "stale output",
        },
      ]),
    );
    expect(screen.queryByText("stale output")).toBeNull();
    expect(screen.getByText("new deployment output")).toBeTruthy();
    expect(screen.getByText("Son sürüm başarılı · today")).toBeTruthy();
  });
});
