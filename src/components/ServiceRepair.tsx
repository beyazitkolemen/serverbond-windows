import { useEffect, useId, useRef, useState } from "react";
import { ShieldCheck, Wrench } from "lucide-react";
import type { Run } from "../types";

export default function ServiceRepair({
  name,
  installed,
  repairable,
  running = false,
  issue,
  busy,
  keeps,
  run,
  action,
  blocked = false,
  blockedReason,
}: {
  name: string;
  installed: boolean;
  repairable: boolean;
  running?: boolean;
  issue: string | null;
  busy: boolean;
  keeps: string[];
  run: Run;
  action: () => Promise<unknown>;
  blocked?: boolean;
  blockedReason?: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const [open, setOpen] = useState(false);
  const needed = repairable && !installed;
  useEffect(() => {
    if (!open) return;
    dialog.current?.showModal();
  }, [open]);
  if (!repairable && !needed) return null;
  const status = needed
    ? "Kurulum eksik"
    : issue
      ? "Çalışma hatası"
      : "Program dosyaları doğrulandı";
  const confirm = () => {
    setOpen(false);
    dialog.current?.close();
    void run(`${name} onarılıyor…`, action);
  };
  return (
    <section
      className={`service-repair${needed ? " needed" : ""}`}
      aria-label={`${name} onarımı`}
    >
      <div className="service-repair-head">
        <h3>Onarım</h3>
        <span className={`service-status${needed || issue ? " issue" : ""}`}>
          <span className={`status-dot ${needed || issue ? "red" : "green"}`} />
          {status}
        </span>
      </div>
      {needed && issue ? (
        <p role="alert" className="settings-feedback">
          {issue}
        </p>
      ) : null}
      <p className="section-note">
        Program dosyaları SHA-256 doğrulanmış paketten yeniden kurulur. Yeni
        paket hazır olmadan mevcut klasör değiştirilmez. Önceki kopya{" "}
        <code>bin/…/sürüm-before-repair-…</code> olarak saklanır.
      </p>
      <ul className="service-repair-keeps">
        {keeps.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
      <div className="settings-actions">
        <button
          type="button"
          className={`button ${needed ? "primary" : "secondary"}`}
          disabled={busy || blocked}
          title={blocked ? blockedReason : undefined}
          onClick={() => setOpen(true)}
        >
          {needed ? <Wrench size={16} /> : <ShieldCheck size={16} />}
          {needed ? "Kurulumu onar" : "Program dosyalarını onar"}
        </button>
      </div>
      {open ? (
        <dialog
          ref={dialog}
          className="modal"
          aria-labelledby={titleId}
          onCancel={(event) => {
            event.preventDefault();
            setOpen(false);
          }}
        >
          <div className="modal-header">
            <h2 id={titleId}>{name} onarılsın mı?</h2>
          </div>
          <p className="dialog-copy">
            {running
              ? "Hizmet durdurulur, paket doğrulanır ve program dosyaları yenilenir."
              : "Paket doğrulanır ve program dosyaları yenilenir."}{" "}
            Veri, jeton, parola ve proje <code>.env</code> dosyalarına
            dokunulmaz.
          </p>
          <ol className="service-repair-steps">
            <li>Çalışıyorsa süreç durdurulur.</li>
            <li>Önbellek veya indirme SHA-256 ile doğrulanır.</li>
            <li>Yeni paket evre klasöründe açılır, sonra atomik taşınır.</li>
            <li>
              Önceki program klasörü <code>before-repair</code> adıyla korunur.
            </li>
          </ol>
          <div className="modal-actions">
            <button
              type="button"
              className="button secondary"
              onClick={() => setOpen(false)}
            >
              Vazgeç
            </button>
            <button type="button" className="button primary" onClick={confirm}>
              <Wrench size={16} />
              Onar
            </button>
          </div>
        </dialog>
      ) : null}
    </section>
  );
}
