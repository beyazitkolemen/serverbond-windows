import { LoaderCircle } from "lucide-react";
import type { InstallProgress } from "../types";
import { formatBytes } from "../updates";

const labels: Record<InstallProgress["phase"], string> = {
  preparing: "Kurulum hazırlanıyor",
  downloading: "İndiriliyor",
  verifying: "SHA-256 doğrulanıyor",
  extracting: "Dosyalar açılıyor",
  installing: "Kurulum tamamlanıyor",
  permissions: "İzinler uygulanıyor",
};

export default function InstallationProgress({
  progress,
}: {
  progress: InstallProgress;
}) {
  const { name, version, phase, completed, total } = progress;
  const percent =
    total && total > 0
      ? Math.min(100, Math.floor((completed / total) * 100))
      : undefined;
  const quantity =
    phase === "extracting"
      ? `${completed.toLocaleString("tr-TR")}${total ? ` / ${total.toLocaleString("tr-TR")}` : ""} dosya`
      : phase === "downloading" || phase === "verifying"
        ? `${formatBytes(completed)}${total ? ` / ${formatBytes(total)}` : ""}`
        : "";
  return (
    <section className="installation-progress" aria-label="Kurulum durumu">
      <div className="installation-progress-heading" role="status">
        <LoaderCircle className="spin" size={18} aria-hidden="true" />
        <strong>
          {name} <span>{version}</span>
        </strong>
        <span>{labels[phase]}</span>
      </div>
      <div
        className={`installation-progress-track${percent === undefined ? " indeterminate" : ""}`}
        role="progressbar"
        aria-label={`${name}: ${labels[phase]}`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
        aria-valuetext={
          percent === undefined
            ? labels[phase]
            : `${labels[phase]}: %${percent}, ${quantity}`
        }
      >
        <span
          style={percent === undefined ? undefined : { width: `${percent}%` }}
        />
      </div>
      <div className="installation-progress-meta">
        <span>{quantity || "Arka planda çalışıyor"}</span>
        <span>{percent === undefined ? "Sürüyor…" : `%${percent}`}</span>
      </div>
    </section>
  );
}
