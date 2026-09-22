import {
  act,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { useDraft } from "../../src/hooks/useDraft";
import {
  NavigationGuard,
  useNavigationGuard,
  useUnsavedChanges,
} from "../../src/hooks/useNavigationGuard";
import { useProjectJobs } from "../../src/components/ProjectJobs";
import ReleasePane from "../../src/components/ProjectRelease";
import NumberField from "../../src/components/NumberField";
import LogViewer from "../../src/components/LogViewer";
import ProjectEnv from "../../src/components/ProjectEnv";
import { defaultRelease, type Project, type Run } from "../../src/types";
import { call } from "../../src/api";
vi.mock("../../src/api", () => ({ call: vi.fn(), desktop: false }));
const project = {
  id: "project-one",
  name: "demo",
  workers: [],
  schedule: { enabled: false, autoStart: false },
  release: defaultRelease(),
} as unknown as Project;
const run: Run = async (_label, action) => {
  await action();
  return true;
};
beforeEach(() => {
  vi.mocked(call).mockReset();
  vi.mocked(call).mockResolvedValue([]);
});

describe("snapshot drafts", () => {
  it("follows clean snapshots, protects edits and resets to the newest source", () => {
    const { result, rerender } = renderHook(({ port }) => useDraft({ port }), {
      initialProps: { port: 8000 },
    });
    rerender({ port: 9000 });
    expect(result.current.values.port).toBe(9000);
    expect(result.current.dirty).toBe(false);
    act(() => result.current.setValues({ port: 9100 }));
    rerender({ port: 9200 });
    expect(result.current.values.port).toBe(9100);
    act(() => result.current.reset());
    expect(result.current.values.port).toBe(9200);
    expect(result.current.dirty).toBe(false);
  });
  it("updates clean queue settings changed by Cloud without discarding a dirty draft", () => {
    const { result, rerender } = renderHook(
      ({ value }) => useProjectJobs(value),
      { initialProps: { value: project } },
    );
    const changed = {
      ...project,
      schedule: { enabled: true, autoStart: true },
    };
    rerender({ value: changed });
    expect(result.current.schedule.enabled).toBe(true);
    expect(result.current.dirty).toBe(false);
    act(() => result.current.setSchedule({ enabled: true, autoStart: false }));
    rerender({ value: project });
    expect(result.current.schedule).toEqual({
      enabled: true,
      autoStart: false,
    });
    act(() => result.current.reset());
    expect(result.current.schedule.enabled).toBe(false);
  });
  it("preserves raw Artisan text when an external release recipe changes", async () => {
    const { rerender } = render(
      <ReleasePane project={project} busy={false} run={run} />,
    );
    const field = screen.getByRole("textbox", {
      name: /Ek Artisan/,
    }) as HTMLTextAreaElement;
    fireEvent.change(field, {
      target: { value: "config:cache\n\nroute:cache  " },
    });
    rerender(
      <ReleasePane
        project={{
          ...project,
          release: { ...defaultRelease(), extraArtisan: ["view:cache"] },
        }}
        busy={false}
        run={run}
      />,
    );
    expect(field.value).toBe("config:cache\n\nroute:cache  ");
    fireEvent.click(screen.getByRole("button", { name: "Vazgeç" }));
    expect(field.value).toBe("view:cache");
    await waitFor(() => expect(call).toHaveBeenCalled());
  });
  it("returns to a clean recipe after saving normalized Artisan text", async () => {
    const { rerender } = render(
      <ReleasePane project={project} busy={false} run={run} />,
    );
    const field = screen.getByRole("textbox", {
      name: /Ek Artisan/,
    }) as HTMLTextAreaElement;
    fireEvent.change(field, { target: { value: "config:cache  \n" } });
    fireEvent.click(
      screen.getByRole("button", { name: "Kaydet", exact: true }),
    );
    await waitFor(() => expect(field.value).toBe("config:cache"));
    rerender(
      <ReleasePane
        project={{
          ...project,
          release: { ...defaultRelease(), extraArtisan: ["config:cache"] },
        }}
        busy={false}
        run={run}
      />,
    );
    rerender(
      <ReleasePane
        project={{
          ...project,
          release: { ...defaultRelease(), extraArtisan: ["view:cache"] },
        }}
        busy={false}
        run={run}
      />,
    );
    expect(field.value).toBe("view:cache");
  });
});

describe("navigation protection", () => {
  function Editor({ dirty, leave }: { dirty: boolean; leave: () => void }) {
    useUnsavedChanges(dirty, "Ortam dosyası");
    const navigate = useNavigationGuard();
    return <button onClick={() => navigate(leave)}>Başka proje</button>;
  }
  it("lets the user keep editing or explicitly discard before navigating", () => {
    const leave = vi.fn();
    const { rerender } = render(
      <NavigationGuard>
        <Editor dirty leave={leave} />
      </NavigationGuard>,
    );
    screen.getByText("Başka proje").focus();
    fireEvent.click(screen.getByText("Başka proje"));
    expect(leave).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog").getAttribute("open")).toBe("");
    fireEvent.click(screen.getByText("Düzenlemeye dön"));
    expect(document.activeElement).toBe(screen.getByText("Başka proje"));
    expect(leave).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("Başka proje"));
    fireEvent.click(screen.getByText("Kaydetmeden devam et"));
    expect(leave).toHaveBeenCalledTimes(1);
    rerender(
      <NavigationGuard>
        <Editor dirty={false} leave={leave} />
      </NavigationGuard>,
    );
    fireEvent.click(screen.getByText("Başka proje"));
    expect(leave).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("dialog")).toBeNull();
  });
  it("does not block navigation after the dirty editor unmounts", () => {
    const leave = vi.fn();
    const { rerender } = render(
      <NavigationGuard>
        <Editor dirty leave={leave} />
      </NavigationGuard>,
    );
    rerender(
      <NavigationGuard>
        <Editor dirty={false} leave={leave} />
      </NavigationGuard>,
    );
    fireEvent.click(screen.getByText("Başka proje"));
    expect(leave).toHaveBeenCalledOnce();
  });
});

describe("honest input and asynchronous feedback", () => {
  it("keeps invalid numeric input visible and blocks native form submission", () => {
    const change = vi.fn();
    render(
      <form>
        <NumberField label="Port" value={8088} min={1024} onChange={change} />
      </form>,
    );
    const field = screen.getByRole("spinbutton") as HTMLInputElement;
    fireEvent.change(field, { target: { value: "1" } });
    fireEvent.blur(field);
    expect(field.value).toBe("1");
    expect(field.checkValidity()).toBe(false);
    expect(field.getAttribute("aria-invalid")).toBe("true");
    expect(change).not.toHaveBeenCalled();
    fireEvent.reset(field.form!);
    expect(field.value).toBe("8088");
    fireEvent.change(field, { target: { value: "8080" } });
    expect(change).toHaveBeenCalledWith(8080);
  });
  it("ignores a late log response from the previous source", async () => {
    let resolveOld!: (value: string) => void;
    const load = vi.fn((source: string) =>
      source === "php"
        ? new Promise<string>((resolve) => {
            resolveOld = resolve;
          })
        : Promise.resolve("new mysql log"),
    );
    render(
      <LogViewer
        label="Günlükler"
        sources={[
          { id: "php", label: "PHP" },
          { id: "mysql", label: "MySQL" },
        ]}
        load={load}
      />,
    );
    fireEvent.click(screen.getByRole("tab", { name: "MySQL" }));
    await screen.findByText("new mysql log");
    await act(async () => resolveOld("stale php log"));
    expect(screen.queryByText("stale php log")).toBeNull();
  });
  it("clears old log lines if a removed source falls back to an unreadable source", async () => {
    const load = vi.fn((source: string) =>
      source === "worker"
        ? Promise.resolve("old worker data")
        : Promise.reject(new Error("read failed")),
    );
    const { rerender } = render(
      <LogViewer
        label="Günlükler"
        initialSource="worker"
        sources={[
          { id: "php", label: "PHP" },
          { id: "worker", label: "İşçi" },
        ]}
        load={load}
      />,
    );
    await screen.findByText("old worker data");
    rerender(
      <LogViewer
        label="Günlükler"
        sources={[{ id: "php", label: "PHP" }]}
        load={load}
      />,
    );
    await screen.findByText("read failed");
    expect(screen.queryByText("old worker data")).toBeNull();
  });
  it("does not claim an env read is still loading after it failed", async () => {
    vi.mocked(call).mockRejectedValue(new Error("Access denied"));
    render(<ProjectEnv project={project} busy={false} run={run} />);
    await screen.findByRole("alert");
    expect(screen.queryByText(".env okunuyor…")).toBeNull();
  });
  it("reports a release history read failure instead of an empty history", async () => {
    vi.mocked(call).mockRejectedValue(new Error("disk error"));
    render(<ReleasePane project={project} busy={false} run={run} />);
    await screen.findByRole("alert");
    expect(screen.queryByText("Henüz sürüm çalıştırılmadı")).toBeNull();
  });
});
