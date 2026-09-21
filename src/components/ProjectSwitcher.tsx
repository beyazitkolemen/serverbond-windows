import { useEffect, useId, useRef, useState } from "react";
import { ChevronsUpDown, Check } from "lucide-react";
import type { Project } from "../types";
import { searchText } from "../search";
import SearchField from "./SearchField";

export default function ProjectSwitcher({
  projects,
  selected,
  onSelect,
}: {
  projects: Project[];
  selected: Project;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const id = useId();
  const close = () => {
    setOpen(false);
    trigger.current?.focus();
  };
  useEffect(() => {
    if (!open) return;
    root.current
      ?.querySelector<HTMLInputElement>('input[type="search"]')
      ?.focus();
    const outside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [open]);
  const visible = projects.filter((project) =>
    searchText(`${project.name} ${project.host} ${project.path}`).includes(
      searchText(query),
    ),
  );
  return (
    <div
      className="project-switcher"
      ref={root}
      onBlur={(event) => {
        if (
          event.relatedTarget &&
          !event.currentTarget.contains(event.relatedTarget as Node)
        )
          setOpen(false);
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
    >
      <button
        ref={trigger}
        type="button"
        className="project-switcher-trigger"
        aria-expanded={open}
        aria-controls={id}
        aria-label={`Proje seç: ${selected.name}`}
        onClick={() => {
          setQuery("");
          setOpen((value) => !value);
        }}
      >
        <span>
          <small>Aktif proje</small>
          <strong>{selected.name}</strong>
        </span>
        <ChevronsUpDown size={16} aria-hidden />
      </button>
      {open && (
        <div
          id={id}
          className="project-switcher-popover"
          role="region"
          aria-label="Proje seçimi"
        >
          <SearchField label="Proje ara" value={query} onChange={setQuery} />
          <div className="project-switcher-results">
            {visible.map((project) => (
              <button
                key={project.id}
                type="button"
                className="project-switcher-option"
                aria-current={project.id === selected.id ? "true" : undefined}
                onClick={() => {
                  onSelect(project.id);
                  close();
                }}
              >
                <span>
                  <strong>{project.name}</strong>
                  <small>{project.host}</small>
                </span>
                {project.id === selected.id && <Check size={16} aria-hidden />}
              </button>
            ))}
            {!visible.length && (
              <div className="search-empty" role="status">
                <strong>Eşleşen proje yok</strong>
                <button
                  type="button"
                  className="section-link"
                  onClick={() => setQuery("")}
                >
                  Aramayı temizle
                </button>
              </div>
            )}
          </div>
          <p className="project-switcher-count">
            {visible.length} / {projects.length} proje
          </p>
        </div>
      )}
    </div>
  );
}
