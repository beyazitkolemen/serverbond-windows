import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SegmentedControl from "../../src/components/SegmentedControl";

const options = [
  { value: "light", label: "Açık" },
  { value: "dark", label: "Koyu" },
  { value: "system", label: "Sistem" },
];

function Theme() {
  const [value, setValue] = useState("dark");
  return (
    <SegmentedControl
      label="Tema"
      value={value}
      onChange={setValue}
      options={options}
    />
  );
}

describe("segmented radio keyboard navigation", () => {
  it("uses one tab stop and moves focus and selection together, including wrapping and Home/End", () => {
    render(<Theme />);
    const radios = screen.getAllByRole("radio") as HTMLButtonElement[];
    expect(radios.map((radio) => radio.tabIndex)).toEqual([-1, 0, -1]);
    radios[1].focus();
    const check = (key: string, index: number) => {
      fireEvent.keyDown(document.activeElement!, { key });
      expect(document.activeElement).toBe(radios[index]);
      expect(radios[index].getAttribute("aria-checked")).toBe("true");
      expect(radios.map((radio) => radio.tabIndex)).toEqual(
        radios.map((_, position) => (position === index ? 0 : -1)),
      );
    };
    check("ArrowRight", 2);
    check("ArrowDown", 0);
    check("ArrowLeft", 2);
    check("ArrowUp", 1);
    check("Home", 0);
    check("End", 2);
    fireEvent.keyDown(radios[2], { key: "Tab" });
    expect(document.activeElement).toBe(radios[2]);
  });

  it("does not change or focus choices while disabled", () => {
    const onChange = vi.fn();
    render(
      <SegmentedControl
        label="Tema"
        value="dark"
        onChange={onChange}
        options={options}
        disabled
      />,
    );
    const radios = screen.getAllByRole("radio") as HTMLButtonElement[];
    for (const key of [
      "ArrowRight",
      "ArrowDown",
      "ArrowLeft",
      "ArrowUp",
      "Home",
      "End",
    ]) {
      fireEvent.keyDown(radios[1], { key });
    }
    fireEvent.click(radios[0]);
    expect(onChange).not.toHaveBeenCalled();
    expect(
      radios.every((radio) => radio.disabled && radio.tabIndex === -1),
    ).toBe(true);
    expect(document.activeElement).toBe(document.body);
  });

  it("keeps the first choice reachable when no value is selected", () => {
    render(
      <SegmentedControl
        label="Tema"
        value="missing"
        onChange={vi.fn()}
        options={options}
      />,
    );
    expect(screen.getAllByRole("radio").map((radio) => radio.tabIndex)).toEqual(
      [0, -1, -1],
    );
  });
});
