import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, it, vi, beforeEach } from "vitest";
import ProjectSetup from "../../src/components/ProjectSetup";
import { projectSetup } from "../../src/services/projectSetup";
import type { PackageStatus, Run } from "../../src/types";
vi.mock("../../src/services/projectSetup", () => ({
  projectSetup: { check: vi.fn(), run: vi.fn() },
}));
beforeEach(() => vi.clearAllMocks());
const versions = [
  { version: "8.4.25", installed: true },
  { version: "8.3.30", installed: false },
] as PackageStatus[];
const run: Run = async (_label, action) => {
  await action();
  return true;
};
describe("project setup", () => {
  it("checks the selected PHP and submits the same setup identity only once", async () => {
    let finish!: () => void;
    vi.mocked(projectSetup.check).mockResolvedValue({
      ready: true,
      phpVersion: "8.3.30",
      sourceInspectionPending: true,
      checks: [],
    });
    vi.mocked(projectSetup.run).mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    render(
      <ProjectSetup
        versions={versions}
        selectedPhp="8.4.25"
        busy={false}
        run={run}
        close={vi.fn()}
        serverError=""
      />,
    );
    fireEvent.change(screen.getByLabelText("Proje adı"), {
      target: { value: "shop" },
    });
    fireEvent.change(screen.getByLabelText("GitHub deposu (ekip/depo)"), {
      target: { value: "team/shop" },
    });
    fireEvent.click(screen.getByText("Devam"));
    fireEvent.change(screen.getByLabelText("PHP sürümü"), {
      target: { value: "8.3.30" },
    });
    fireEvent.click(screen.getByText("Gereksinimleri kontrol et"));
    await screen.findByText("Kurulumu başlat");
    const input = vi.mocked(projectSetup.check).mock.calls[0][0];
    if (input.phpVersion !== "8.3.30") throw Error("Selected PHP lost");
    fireEvent.click(screen.getByText("Kurulumu başlat"));
    fireEvent.click(screen.getByText("Kurulumu başlat"));
    await waitFor(() => {
      if (vi.mocked(projectSetup.run).mock.calls.length !== 1)
        throw Error("Duplicate setup");
    });
    if (
      vi.mocked(projectSetup.run).mock.calls[0][0].requestId !== input.requestId
    )
      throw Error("Identity changed");
    finish();
  });
});
