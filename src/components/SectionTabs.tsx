import type { ReactNode } from "react";

/** One keyboard stop per tab list; arrow keys activate and focus adjacent tabs. */
export default function SectionTabs<T extends string>({
  id,
  label,
  items,
  value,
  onChange,
  orientation = "horizontal",
}: {
  id: string;
  label: string;
  items: readonly {
    id: T;
    label: string;
    icon?: ReactNode;
    indicator?: ReactNode;
  }[];
  value: T;
  onChange: (value: T) => void;
  orientation?: "horizontal" | "vertical";
}) {
  return (
    <div
      className="section-tabs"
      role="tablist"
      aria-label={label}
      aria-orientation={orientation}
    >
      {items.map((item, index) => (
        <button
          key={item.id}
          id={`${id}-${item.id}`}
          type="button"
          role="tab"
          aria-selected={value === item.id}
          aria-controls={`${id}-panel`}
          tabIndex={value === item.id ? 0 : -1}
          onClick={() => onChange(item.id)}
          onKeyDown={(event) => {
            const next =
              event.key ===
              (orientation === "vertical" ? "ArrowDown" : "ArrowRight")
                ? (index + 1) % items.length
                : event.key ===
                    (orientation === "vertical" ? "ArrowUp" : "ArrowLeft")
                  ? (index - 1 + items.length) % items.length
                  : event.key === "Home"
                    ? 0
                    : event.key === "End"
                      ? items.length - 1
                      : -1;
            if (next < 0) return;
            event.preventDefault();
            onChange(items[next].id);
            const button =
              event.currentTarget.parentElement?.querySelectorAll<HTMLButtonElement>(
                '[role="tab"]',
              )[next];
            button?.focus({ preventScroll: true });
            button?.scrollIntoView({ block: "nearest", inline: "nearest" });
          }}
        >
          {item.icon}
          <span>{item.label}</span>
          {item.indicator}
        </button>
      ))}
    </div>
  );
}
