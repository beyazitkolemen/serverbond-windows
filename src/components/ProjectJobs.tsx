import { useState } from "react";
import {
  Play,
  Square,
  Plus,
  Trash2,
  ListTodo,
  ChevronDown,
} from "lucide-react";
import { call } from "../api";
import type { Project, ProjectSchedule, QueueWorker, Run } from "../types";

function emptyWorker(name = "default"): QueueWorker {
  return {
    id: crypto.randomUUID(),
    name,
    connection: "default",
    queue: "default",
    processes: 1,
    timeout: 60,
    sleep: 3,
    maxTries: 1,
    memory: 128,
    backoff: 0,
    enabled: true,
    autoStart: true,
  };
}

export default function ProjectJobs({
  project,
  busy,
  run,
}: {
  project: Project;
  busy: boolean;
  run: Run;
}) {
  const [open, setOpen] = useState(false);
  const [workers, setWorkers] = useState<QueueWorker[]>(() =>
    structuredClone(project.workers ?? []),
  );
  const [schedule, setSchedule] = useState<ProjectSchedule>(
    () => project.schedule ?? { enabled: false, autoStart: false },
  );
  const [tasks, setTasks] = useState("");
  const dirty =
    JSON.stringify(workers) !== JSON.stringify(project.workers ?? []) ||
    JSON.stringify(schedule) !== JSON.stringify(project.schedule);
  const runningWorkers = (project.workerStates ?? []).filter(
    (item) => item.running > 0,
  ).length;
  const summary = [
    project.scheduleRunning
      ? "Zamanlayıcı çalışıyor"
      : schedule.enabled
        ? "Zamanlayıcı hazır"
        : "Zamanlayıcı kapalı",
    runningWorkers
      ? `${runningWorkers} işçi çalışıyor`
      : workers.length
        ? `${workers.length} kuyruk işçisi`
        : "Kuyruk kapalı",
  ].join(" · ");
  const update = (index: number, patch: Partial<QueueWorker>) =>
    setWorkers((list) =>
      list.map((item, i) => (i === index ? { ...item, ...patch } : item)),
    );
  return (
    <div className="project-jobs">
      <button
        type="button"
        className="project-jobs-toggle"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <span>
          <strong>Kuyruk ve zamanlama</strong>
          <small>{summary}</small>
        </span>
        <ChevronDown size={18} className={open ? "is-open" : undefined} />
      </button>
      {open && (
        <div className="project-jobs-panel">
          <p className="section-note">
            İşçiler Supervisor gibi `queue:work` çalıştırır. Zamanlayıcı her
            dakika Laravel `schedule:run` komutunu yürütür.
          </p>
          {!workers.length && !schedule.enabled && (
            <button
              type="button"
              className="button secondary small"
              disabled={busy}
              onClick={() => {
                setWorkers([emptyWorker()]);
                setSchedule({ enabled: true, autoStart: true });
              }}
            >
              Laravel kuyruk ve zamanlamayı hazırla
            </button>
          )}
          <div className="project-job-row">
            <label className="setting-toggle">
              <input
                type="checkbox"
                checked={schedule.enabled}
                onChange={(e) =>
                  setSchedule((current) => ({
                    ...current,
                    enabled: e.target.checked,
                  }))
                }
              />
              <span>Laravel schedule</span>
            </label>
            <label className="setting-toggle">
              <input
                type="checkbox"
                checked={schedule.autoStart}
                disabled={!schedule.enabled}
                onChange={(e) =>
                  setSchedule((current) => ({
                    ...current,
                    autoStart: e.target.checked,
                  }))
                }
              />
              <span>Ortamla başlat</span>
            </label>
            <div className="project-job-actions">
              <span
                className={`service-status ${project.scheduleRunning ? "running" : ""}`}
              >
                <span
                  className={`status-dot ${project.scheduleRunning ? "green" : ""}`}
                />
                {project.scheduleRunning ? "Çalışıyor" : "Durdu"}
              </span>
              <button
                type="button"
                className="button secondary small"
                disabled={
                  busy ||
                  dirty ||
                  (!project.scheduleRunning && !schedule.enabled)
                }
                title={
                  dirty
                    ? "Önce zamanlayıcı ayarlarını kaydedin"
                    : undefined
                }
                onClick={() =>
                  void run(
                    project.scheduleRunning
                      ? "Zamanlayıcı durduruluyor…"
                      : "Zamanlayıcı başlatılıyor…",
                    () =>
                      call(
                        project.scheduleRunning
                          ? "stop_project_schedule"
                          : "start_project_schedule",
                        { id: project.id },
                      ),
                  )
                }
              >
                {project.scheduleRunning ? (
                  <Square size={14} />
                ) : (
                  <Play size={14} />
                )}
                {project.scheduleRunning ? "Durdur" : "Başlat"}
              </button>
              <button
                type="button"
                className="button secondary small"
                disabled={busy}
                onClick={() =>
                  void run("Zamanlanmış görevler okunuyor…", async () =>
                    setTasks(
                      await call<string>("list_project_schedule", {
                        id: project.id,
                      }),
                    ),
                  )
                }
              >
                <ListTodo size={14} />
                Görevler
              </button>
            </div>
          </div>
          {project.scheduleIssue && (
            <p className="project-job-issue">{project.scheduleIssue}</p>
          )}
          {tasks && <pre className="project-console">{tasks}</pre>}
          {workers.map((worker, index) => {
            const state = (project.workerStates ?? []).find(
              (item) => item.id === worker.id,
            );
            const running = (state?.running ?? 0) > 0;
            return (
              <div className="project-worker" key={worker.id}>
                <div className="project-worker-head">
                  <strong>{worker.name.trim() || "İşçi"}</strong>
                  <div className="project-job-actions">
                    <span
                      className={`service-status ${running ? "running" : ""}`}
                    >
                      <span
                        className={`status-dot ${running ? "green" : ""}`}
                      />
                      {running
                        ? `${state?.running ?? 0}/${worker.processes} çalışıyor`
                        : "Durdu"}
                    </span>
                    <button
                      type="button"
                      className="button secondary small"
                      disabled={
                        busy ||
                        dirty ||
                        (!running && !worker.enabled)
                      }
                      title={
                        dirty
                          ? "Önce kuyruk ayarlarını kaydedin"
                          : !worker.enabled
                            ? "İşçiyi etkinleştirip kaydedin"
                            : undefined
                      }
                      onClick={() =>
                        void run(
                          running ? "İşçi durduruluyor…" : "İşçi başlatılıyor…",
                          () =>
                            call(
                              running
                                ? "stop_project_worker"
                                : "start_project_worker",
                              { id: project.id, workerId: worker.id },
                            ),
                        )
                      }
                    >
                      {running ? <Square size={14} /> : <Play size={14} />}
                      {running ? "Durdur" : "Başlat"}
                    </button>
                    <button
                      type="button"
                      className="icon-button danger"
                      disabled={busy || running}
                      aria-label={`${worker.name} işçisini sil`}
                      onClick={() =>
                        setWorkers((list) => list.filter((_, i) => i !== index))
                      }
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </div>
                <div className="project-worker-grid">
                  <label>
                    Ad
                    <input
                      value={worker.name}
                      onChange={(e) => update(index, { name: e.target.value })}
                    />
                  </label>
                  <label>
                    Bağlantı
                    <input
                      value={worker.connection}
                      onChange={(e) =>
                        update(index, { connection: e.target.value })
                      }
                    />
                  </label>
                  <label>
                    Kuyruklar
                    <input
                      value={worker.queue}
                      onChange={(e) => update(index, { queue: e.target.value })}
                    />
                  </label>
                  <label>
                    Süreç
                    <input
                      type="number"
                      min={1}
                      max={8}
                      value={worker.processes}
                      onChange={(e) =>
                        update(index, { processes: Number(e.target.value) })
                      }
                    />
                  </label>
                </div>
                <details className="project-worker-more">
                  <summary>Gelişmiş</summary>
                  <div className="project-worker-grid">
                    <label>
                      Zaman aşımı (sn)
                      <input
                        type="number"
                        min={1}
                        max={86400}
                        value={worker.timeout}
                        onChange={(e) =>
                          update(index, { timeout: Number(e.target.value) })
                        }
                      />
                    </label>
                    <label>
                      Bellek (MB)
                      <input
                        type="number"
                        min={32}
                        max={2048}
                        value={worker.memory}
                        onChange={(e) =>
                          update(index, { memory: Number(e.target.value) })
                        }
                      />
                    </label>
                    <label>
                      Bekleme (sn)
                      <input
                        type="number"
                        min={1}
                        max={60}
                        value={worker.sleep}
                        onChange={(e) =>
                          update(index, { sleep: Number(e.target.value) })
                        }
                      />
                    </label>
                    <label>
                      Deneme
                      <input
                        type="number"
                        min={0}
                        max={1000}
                        value={worker.maxTries}
                        onChange={(e) =>
                          update(index, { maxTries: Number(e.target.value) })
                        }
                      />
                    </label>
                    <label>
                      Geri offset (sn)
                      <input
                        type="number"
                        min={0}
                        max={3600}
                        value={worker.backoff}
                        onChange={(e) =>
                          update(index, { backoff: Number(e.target.value) })
                        }
                      />
                    </label>
                  </div>
                </details>
                <div className="project-job-row">
                  <label className="setting-toggle">
                    <input
                      type="checkbox"
                      checked={worker.enabled}
                      onChange={(e) =>
                        update(index, { enabled: e.target.checked })
                      }
                    />
                    <span>Etkin</span>
                  </label>
                  <label className="setting-toggle">
                    <input
                      type="checkbox"
                      checked={worker.autoStart}
                      onChange={(e) =>
                        update(index, { autoStart: e.target.checked })
                      }
                    />
                    <span>Ortamla başlat</span>
                  </label>
                </div>
                {state?.issue && (
                  <p className="project-job-issue">{state.issue}</p>
                )}
              </div>
            );
          })}
          <div className="project-jobs-footer">
            <button
              type="button"
              className="button secondary small"
              disabled={busy || workers.length >= 8}
              onClick={() =>
                setWorkers((list) => [
                  ...list,
                  emptyWorker(
                    list.length ? `worker-${list.length + 1}` : "default",
                  ),
                ])
              }
            >
              <Plus size={15} />
              İşçi ekle
            </button>
            <button
              type="button"
              className="button primary small"
              disabled={busy || !dirty}
              onClick={() =>
                void run("Kuyruk ayarları kaydediliyor…", () =>
                  call("save_project_jobs", {
                    id: project.id,
                    workers,
                    schedule,
                  }),
                )
              }
            >
              Kaydet
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
