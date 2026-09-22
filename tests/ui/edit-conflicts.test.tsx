import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { call } from "../../src/api";
import { useDraft } from "../../src/hooks/useDraft";
import { NavigationGuard } from "../../src/hooks/useNavigationGuard";
import ProjectEnv from "../../src/components/ProjectEnv";
import ReleasePane from "../../src/components/ProjectRelease";
import {
  ScheduleSection,
  useProjectJobs,
} from "../../src/components/ProjectJobs";
import { defaultRelease, type Project, type Run } from "../../src/types";

vi.mock("../../src/api", () => ({ call: vi.fn(), desktop: false }));
const project = {
  id: "demo",
  name: "Demo",
  workers: [],
  schedule: { enabled: false, autoStart: false },
  release: defaultRelease(),
} as unknown as Project;
const run: Run = async (_label, action) => {
  try {
    await action();
    return true;
  } catch {
    return false;
  }
};
beforeEach(() => {
  vi.mocked(call).mockReset().mockResolvedValue([]);
});

describe("draft conflict baselines", () => {
  it("keeps the original baseline through multiple remote changes and resets explicitly", () => {
    const { result, rerender } = renderHook(({ port }) => useDraft({ port }), {
      initialProps: { port: 8000 },
    });
    act(() => result.current.setValues({ port: 8100 }));
    rerender({ port: 8200 });
    rerender({ port: 8300 });
    expect(result.current.values.port).toBe(8100);
    expect(result.current.baseline.port).toBe(8000);
    expect(result.current.conflicted).toBe(true);
    act(() => result.current.reset());
    expect(result.current.values.port).toBe(8300);
    expect(result.current.baseline.port).toBe(8300);
    expect(result.current.conflicted).toBe(false);
  });
  it("advances the baseline when a saved draft is acknowledged by the snapshot", () => {
    const { result, rerender } = renderHook(({ port }) => useDraft({ port }), {
      initialProps: { port: 8000 },
    });
    act(() => result.current.setValues({ port: 8100 }));
    rerender({ port: 8100 });
    expect(result.current.baseline.port).toBe(8100);
    expect(result.current.dirty).toBe(false);
    rerender({ port: 8200 });
    expect(result.current.values.port).toBe(8200);
    expect(result.current.conflicted).toBe(false);
  });
});

describe("desktop saves carry what the user originally read", () => {
  it("lets the user discard whitespace-only Artisan edits", async () => {
    render(<ReleasePane project={project} busy={false} run={run} />);
    await waitFor(() => expect(call).toHaveBeenCalled());
    const field = screen.getByRole("textbox", {
      name: /Ek Artisan/,
    }) as HTMLTextAreaElement;
    fireEvent.change(field, { target: { value: "  \n" } });
    fireEvent.click(screen.getByText("Vazgeç"));
    expect(field.value).toBe("");
  });
  it("sends the original environment file and preserves the draft on conflict", async () => {
    const file = { exists: true, content: "ORIGINAL=1", example: null };
    vi.mocked(call).mockImplementation(async (command) => {
      if (command === "read_project_env") return file;
      throw new Error(".env değişti");
    });
    render(<ProjectEnv project={project} busy={false} run={run} />);
    await screen.findByText("Dosya mevcut.");
    const field = screen.getByRole("textbox") as HTMLTextAreaElement;
    fireEvent.change(field, { target: { value: "LOCAL=1" } });
    fireEvent.click(screen.getByText("Kaydet"));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("save_project_env", {
        id: project.id,
        content: "LOCAL=1",
        expected: file,
      }),
    );
    expect(field.value).toBe("LOCAL=1");
    expect(screen.getByText("Kaydedilmemiş değişiklikler")).toBeTruthy();
  });
  it("does not discard an env draft while the user declines reloading", async () => {
    vi.mocked(call).mockResolvedValue({
      exists: true,
      content: "ORIGINAL=1",
      example: null,
    });
    render(
      <NavigationGuard>
        <ProjectEnv project={project} busy={false} run={run} />
      </NavigationGuard>,
    );
    await screen.findByText("Dosya mevcut.");
    const field = screen.getByRole("textbox") as HTMLTextAreaElement;
    fireEvent.change(field, { target: { value: "LOCAL=1" } });
    fireEvent.click(screen.getByText("Dosyayı yeniden yükle"));
    fireEvent.click(screen.getByText("Düzenlemeye dön"));
    expect(field.value).toBe("LOCAL=1");
    vi.mocked(call).mockResolvedValue({
      exists: true,
      content: "REMOTE=1",
      example: null,
    });
    fireEvent.click(screen.getByText("Dosyayı yeniden yükle"));
    fireEvent.click(screen.getByText("Kaydetmeden devam et"));
    await waitFor(() => expect(field.value).toBe("REMOTE=1"));
  });
  it("blocks a stale release recipe and sends the chosen recipe when running", async () => {
    const { rerender } = render(
      <ReleasePane project={project} busy={false} run={run} />,
    );
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("list_project_releases", {
        id: project.id,
      }),
    );
    const field = screen.getByRole("textbox", {
      name: /Ek Artisan/,
    }) as HTMLTextAreaElement;
    fireEvent.change(field, { target: { value: "view:cache" } });
    const remote = {
      ...project,
      release: { ...defaultRelease(), branch: "remote" },
    };
    rerender(<ReleasePane project={remote} busy={false} run={run} />);
    expect(
      (screen.getByRole("button", { name: "Çalıştır" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
    expect(field.value).toBe("view:cache");
    fireEvent.click(screen.getByText("Vazgeç"));
    vi.mocked(call).mockImplementation(async (command) =>
      command === "deploy_project" ? { output: "done" } : [],
    );
    fireEvent.click(screen.getByRole("button", { name: "Çalıştır" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("deploy_project", {
        id: project.id,
        expected: remote.release,
      }),
    );
  });
  it("passes a queue baseline without replacing it with the edited schedule", async () => {
    function Editor() {
      const jobs = useProjectJobs(project);
      return (
        <ScheduleSection project={project} busy={false} run={run} jobs={jobs} />
      );
    }
    render(<Editor />);
    fireEvent.click(screen.getByText("Zamanlayıcıyı etkinleştir"));
    fireEvent.click(screen.getByRole("button", { name: "Ayarları kaydet" }));
    await waitFor(() =>
      expect(call).toHaveBeenCalledWith("save_project_jobs", {
        id: project.id,
        workers: [],
        schedule: { enabled: true, autoStart: true },
        expected: { workers: [], schedule: project.schedule },
      }),
    );
  });
});
