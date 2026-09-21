import { useCallback, useState } from "react";
import { ChevronDown } from "lucide-react";
import { call } from "../api";
import type { Project } from "../types";
import LogViewer, { type LogSource } from "./LogViewer";

function sourcesFor(project: Project): LogSource[] {
  return [
    { id: "php", label: `PHP ${project.phpVersion}` },
    { id: "schedule", label: "Zamanlayıcı" },
    ...(project.workers ?? []).map((worker) => ({
      id: `worker:${worker.id}`,
      label: worker.name.trim() || "İşçi",
    })),
  ];
}

export default function ProjectLogs({
  project,
  initialSource,
  embedded = false,
}: {
  project: Project;
  initialSource?: string;
  embedded?: boolean;
}) {
  const [open, setOpen] = useState(Boolean(initialSource) || embedded);
  const sources = sourcesFor(project);
  const load = useCallback(
    (source: string) =>
      call<string>("read_project_log", { id: project.id, source }),
    [project.id],
  );
  const extra = (project.workers ?? []).length;
  const summary = [
    "PHP",
    project.scheduleRunning || project.schedule?.enabled ? "zamanlayıcı" : null,
    extra ? `${extra} kuyruk işçisi` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  const viewer = (
    <LogViewer
      key={project.id}
      sources={sources}
      load={load}
      label={`${project.name} günlükleri`}
      initialSource={initialSource}
    />
  );
  if (embedded) return <div className="project-logs is-embedded">{viewer}</div>;
  return (
    <div className="project-logs">
      <button
        type="button"
        className="project-jobs-toggle project-logs-toggle"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span>
          <strong>Günlükler</strong>
          <small>{summary || "Proje süreç kayıtları"}</small>
        </span>
        <ChevronDown size={18} className={open ? "is-open" : undefined} />
      </button>
      {open ? viewer : null}
    </div>
  );
}
