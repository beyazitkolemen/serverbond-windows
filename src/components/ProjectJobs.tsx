import { useEffect, useRef, useState } from "react";
import {
  Play,
  Square,
  Plus,
  Trash2,
  ListTodo,
  RotateCw,
  CircleAlert,
  RotateCcw,
  Eraser,
} from "lucide-react";
import { call } from "../api";
import type { Project, ProjectSchedule, QueueWorker, Run } from "../types";
import NumberField from "./NumberField";

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
    maxJobs: 0,
    maxTime: 0,
    enabled: true,
    autoStart: true,
  };
}

function normalizeWorker(worker: QueueWorker): QueueWorker {
  return {
    ...emptyWorker(worker.name),
    ...worker,
    maxJobs: worker.maxJobs ?? 0,
    maxTime: worker.maxTime ?? 0,
  };
}

export type ProjectJobsModel = ReturnType<typeof useProjectJobs>;

export function useProjectJobs(project: Project) {
  const [workers, setWorkers] = useState<QueueWorker[]>(() =>
    (project.workers ?? []).map((worker) =>
      normalizeWorker(structuredClone(worker)),
    ),
  );
  const [schedule, setSchedule] = useState<ProjectSchedule>(
    () => project.schedule ?? { enabled: false, autoStart: false },
  );
  const [tasks, setTasks] = useState("");
  const [failed, setFailed] = useState("");
  const sourceWorkers = JSON.stringify(
    (project.workers ?? []).map(normalizeWorker),
  );
  const sourceSchedule = JSON.stringify(
    project.schedule ?? { enabled: false, autoStart: false },
  );
  const dirty =
    JSON.stringify(workers) !== sourceWorkers ||
    JSON.stringify(schedule) !== sourceSchedule;
  const wasDirty = useRef(false);
  useEffect(() => {
    wasDirty.current = dirty;
  }, [dirty]);
  useEffect(() => {
    setWorkers(
      (project.workers ?? []).map((worker) =>
        normalizeWorker(structuredClone(worker)),
      ),
    );
    setSchedule(project.schedule ?? { enabled: false, autoStart: false });
    setTasks("");
    setFailed("");
    wasDirty.current = false;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project.id]);
  useEffect(() => {
    // Edits made outside this pane (CLI, API) replace a clean draft; unsaved
    // edits are kept so nothing typed is lost.
    if (wasDirty.current) return;
    setWorkers(
      (project.workers ?? []).map((worker) =>
        normalizeWorker(structuredClone(worker)),
      ),
    );
    setSchedule(project.schedule ?? { enabled: false, autoStart: false });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceWorkers, sourceSchedule]);
  const runningWorkers = (project.workerStates ?? []).filter(
    (item) => item.running > 0,
  ).length;
  const update = (index: number, patch: Partial<QueueWorker>) =>
    setWorkers((list) =>
      list.map((item, i) => (i === index ? { ...item, ...patch } : item)),
    );
  const locked = (title?: string) =>
    dirty ? "Önce kuyruk ve zamanlayıcı ayarlarını kaydedin" : title;
  return {
    workers,
    setWorkers,
    schedule,
    setSchedule,
    tasks,
    setTasks,
    failed,
    setFailed,
    dirty,
    runningWorkers,
    update,
    locked,
  };
}

export function JobsSaveBar({
  project,
  busy,
  run,
  jobs,
}: {
  project: Project;
  busy: boolean;
  run: Run;
  jobs: ProjectJobsModel;
}) {
  return (
    <>
      <span className="muted">
        {jobs.dirty ? "Kaydedilmemiş değişiklik var." : "Ayarlar güncel."}
      </span>
      <button
        type="button"
        className="button primary small"
        disabled={busy || !jobs.dirty}
        onClick={() =>
          void run("Kuyruk ayarları kaydediliyor…", () =>
            call("save_project_jobs", {
              id: project.id,
              workers: jobs.workers,
              schedule: jobs.schedule,
            }),
          )
        }
      >
        Kaydet
      </button>
    </>
  );
}

export function ScheduleSection({
  project,
  busy,
  run,
  jobs,
}: {
  project: Project;
  busy: boolean;
  run: Run;
  jobs: ProjectJobsModel;
}) {
  const { schedule, setSchedule, dirty, locked, tasks, setTasks } = jobs;
  return (
    <div className="project-jobs-panel">
      <p className="section-note">
        Zamanlayıcı her dakika Laravel <code>schedule:run</code> komutunu
        yürütür. Tanımlar projenin <code>routes/console.php</code> veya
        <code>app/Console</code> dosyalarındadır.
      </p>
      {!schedule.enabled ? (
        <button
          type="button"
          className="button secondary small"
          disabled={busy}
          onClick={() => setSchedule({ enabled: true, autoStart: true })}
        >
          Zamanlayıcıyı etkinleştir
        </button>
      ) : null}
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
              busy || dirty || (!project.scheduleRunning && !schedule.enabled)
            }
            title={dirty ? "Önce zamanlayıcı ayarlarını kaydedin" : undefined}
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
            disabled={busy || dirty || !schedule.enabled}
            title={locked(
              !schedule.enabled
                ? "Zamanlayıcıyı etkinleştirip kaydedin"
                : undefined,
            )}
            onClick={() =>
              void run("Zamanlayıcı yeniden başlatılıyor…", () =>
                call("restart_project_schedule", { id: project.id }),
              )
            }
          >
            <RotateCw size={14} />
            Yeniden başlat
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
            Görevleri listele
          </button>
        </div>
      </div>
      {project.scheduleIssue ? (
        <p className="project-job-issue">{project.scheduleIssue}</p>
      ) : null}
      {tasks ? <pre className="project-console">{tasks}</pre> : null}
      <div className="project-jobs-footer">
        <JobsSaveBar project={project} busy={busy} run={run} jobs={jobs} />
      </div>
    </div>
  );
}

export function QueueSection({
  project,
  busy,
  run,
  jobs,
}: {
  project: Project;
  busy: boolean;
  run: Run;
  jobs: ProjectJobsModel;
}) {
  const { workers, setWorkers, dirty, locked, update, failed, setFailed } =
    jobs;
  return (
    <div className="project-jobs-panel">
      <p className="section-note">
        İşçiler Supervisor gibi <code>queue:work</code> çalıştırır. Azami iş
        veya süre dolunca işçi çıkar; otomatik yeniden başlamaz.
      </p>
      {!workers.length ? (
        <button
          type="button"
          className="button secondary small"
          disabled={busy}
          onClick={() => setWorkers([emptyWorker()])}
        >
          Varsayılan işçiyi ekle
        </button>
      ) : null}
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
                <span className={`service-status ${running ? "running" : ""}`}>
                  <span className={`status-dot ${running ? "green" : ""}`} />
                  {running
                    ? `${state?.running ?? 0}/${worker.processes} çalışıyor`
                    : "Durdu"}
                </span>
                <button
                  type="button"
                  className="button secondary small"
                  disabled={busy || dirty || (!running && !worker.enabled)}
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
                  className="button secondary small"
                  disabled={busy || dirty || !worker.enabled}
                  title={locked(
                    !worker.enabled
                      ? "İşçiyi etkinleştirip kaydedin"
                      : undefined,
                  )}
                  onClick={() =>
                    void run("İşçi yeniden başlatılıyor…", () =>
                      call("restart_project_worker", {
                        id: project.id,
                        workerId: worker.id,
                      }),
                    )
                  }
                >
                  <RotateCw size={14} />
                  Yeniden başlat
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
              <NumberField
                label="Süreç"
                min={1}
                max={8}
                value={worker.processes}
                onChange={(n) => update(index, { processes: n })}
              />
            </div>
            <details className="project-worker-more">
              <summary>Gelişmiş</summary>
              <div className="project-worker-grid">
                <NumberField
                  label="Zaman aşımı (sn)"
                  min={1}
                  max={86400}
                  value={worker.timeout}
                  onChange={(n) => update(index, { timeout: n })}
                />
                <NumberField
                  label="Bellek (MB)"
                  min={32}
                  max={2048}
                  value={worker.memory}
                  onChange={(n) => update(index, { memory: n })}
                />
                <NumberField
                  label="Bekleme (sn)"
                  min={1}
                  max={60}
                  value={worker.sleep}
                  onChange={(n) => update(index, { sleep: n })}
                />
                <NumberField
                  label="Deneme"
                  min={0}
                  max={1000}
                  value={worker.maxTries}
                  onChange={(n) => update(index, { maxTries: n })}
                />
                <NumberField
                  label="Geri offset (sn)"
                  min={0}
                  max={3600}
                  value={worker.backoff}
                  onChange={(n) => update(index, { backoff: n })}
                />
                <NumberField
                  label="Azami iş (0=sınırsız)"
                  min={0}
                  max={1000000}
                  value={worker.maxJobs}
                  onChange={(n) => update(index, { maxJobs: n })}
                />
                <NumberField
                  label="Azami süre (sn)"
                  min={0}
                  max={604800}
                  value={worker.maxTime}
                  onChange={(n) => update(index, { maxTime: n })}
                />
              </div>
            </details>
            <div className="project-job-row">
              <label className="setting-toggle">
                <input
                  type="checkbox"
                  checked={worker.enabled}
                  onChange={(e) => update(index, { enabled: e.target.checked })}
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
            {state?.issue ? (
              <p className="project-job-issue">{state.issue}</p>
            ) : null}
          </div>
        );
      })}
      <div className="project-failed">
        <div className="project-job-row">
          <strong>Başarısız kuyruk işleri</strong>
          <div className="project-job-actions">
            <button
              type="button"
              className="button secondary small"
              disabled={busy}
              onClick={() =>
                void run("Başarısız işler okunuyor…", async () =>
                  setFailed(
                    await call<string>("list_failed_jobs", {
                      id: project.id,
                    }),
                  ),
                )
              }
            >
              <CircleAlert size={14} />
              Listele
            </button>
            <button
              type="button"
              className="button secondary small"
              disabled={busy}
              onClick={() =>
                void run(
                  "Başarısız işler yeniden kuyruğa alınıyor…",
                  async () =>
                    setFailed(
                      await call<string>("retry_failed_jobs", {
                        id: project.id,
                        job: "all",
                      }),
                    ),
                )
              }
            >
              <RotateCcw size={14} />
              Yeniden kuyruğa al
            </button>
            <button
              type="button"
              className="button secondary small"
              disabled={busy}
              onClick={() => {
                if (
                  !window.confirm(
                    "Tüm başarısız kuyruk işleri silinecek. Devam edilsin mi?",
                  )
                ) {
                  return;
                }
                void run("Başarısız işler temizleniyor…", async () =>
                  setFailed(
                    await call<string>("flush_failed_jobs", {
                      id: project.id,
                    }),
                  ),
                );
              }}
            >
              <Eraser size={14} />
              Temizle
            </button>
          </div>
        </div>
        {failed ? <pre className="project-console">{failed}</pre> : null}
      </div>
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
          <Plus size={16} />
          İşçi ekle
        </button>
        <JobsSaveBar project={project} busy={busy} run={run} jobs={jobs} />
      </div>
    </div>
  );
}
