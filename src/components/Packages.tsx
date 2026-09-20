import {
  Download,
  Play,
  Square,
  Check,
  Wrench,
  ExternalLink,
} from "lucide-react";
import { useEffect, useState } from "react";
import { call } from "../api";
import type { PackageStatus, Run } from "../types";

export default function Packages({
  packages,
  phpVersions,
  busy,
  run,
  detailed = false,
  running,
  pmaEnabled,
  onOpen,
}: {
  packages: PackageStatus[];
  phpVersions: PackageStatus[];
  busy: boolean;
  run: Run;
  detailed?: boolean;
  running: boolean;
  pmaEnabled: boolean;
  onOpen?: () => void;
}) {
  const active = packages.find((p) => p.id === "php");
  const [version, setVersion] = useState(active?.version ?? "");
  useEffect(() => {
    setVersion(active?.version ?? "");
  }, [active?.version]);
  const selected = phpVersions.find((p) => p.version === version);
  const alreadyActive = selected?.installed && version === active?.version;
  const needsRepair = Boolean(selected?.repairable && !selected.installed);
  const databaseReady = ["php", "mysql", "caddy"].every((id) =>
    packages.some((p) => p.id === id && p.running),
  );
  return (
    <section aria-labelledby="packages-heading">
      <div className="section-heading">
        <h2
          id="packages-heading"
          className={detailed ? "visually-hidden" : undefined}
        >
          Bileşenler
        </h2>
        {detailed ? (
          <span className="muted">Windows x64 · Doğrulanmış paketler</span>
        ) : null}
        {onOpen ? (
          <button type="button" className="section-link" onClick={onOpen}>
            Tümünü yönet
          </button>
        ) : null}
      </div>
      {detailed ? (
        <div className="php-selector">
          <div className="php-selector-copy">
            <label htmlFor="php-version">Varsayılan PHP sürümü</label>
            <p>
              Yeni projeler için kullanılacak Windows x64 NTS paketi indirilir.
            </p>
            <small>
              Varsayılan: PHP {active?.version} · Mevcut projelerin sürümü proje
              kartından seçilir.
            </small>
          </div>
          <div className="php-selector-controls">
            <select
              id="php-version"
              value={version}
              disabled={busy}
              onChange={(event) => setVersion(event.target.value)}
            >
              {phpVersions.map((p) => (
                <option key={p.version} value={p.version}>
                  PHP {p.version}
                  {p.installed ? " · Kurulu" : ""}
                </option>
              ))}
            </select>
            <button
              className="button secondary"
              disabled={
                busy || !selected || alreadyActive || (needsRepair && running)
              }
              title={
                needsRepair && running
                  ? "Onarmak için önce ortamı durdurun"
                  : undefined
              }
              onClick={() =>
                void run(`PHP ${version} hazırlanıyor…`, () =>
                  call(needsRepair ? "repair_php" : "select_php", { version }),
                )
              }
            >
              {alreadyActive ? <Check size={16} /> : <Download size={16} />}
              {alreadyActive
                ? "Kullanılıyor"
                : needsRepair
                  ? "Onar ve kullan"
                  : selected?.installed
                    ? "Bu sürümü kullan"
                    : "İndir ve kullan"}
            </button>
          </div>
        </div>
      ) : null}
      {packages.some((p) => p.issue) ? (
        <div className="package-issues" role="status">
          {packages
            .filter((p) => p.issue)
            .map((p) => (
              <p key={p.id}>
                <strong>{p.name}:</strong> {p.issue}
              </p>
            ))}
        </div>
      ) : null}
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>Bileşen</th>
              <th>Açıklama</th>
              <th>Sürüm</th>
              <th>Durum</th>
              <th className="actions-heading">İşlem</th>
            </tr>
          </thead>
          <tbody>
            {packages.map((p) => (
              <tr key={p.id}>
                <td>
                  <div className="package-name">
                    <span className="package-monogram" aria-hidden="true">
                      {{
                        php: "php",
                        mysql: "my",
                        caddy: "C",
                        composer: "Co",
                        phpmyadmin: "pA",
                      }[p.id] ?? p.name.slice(0, 2)}
                    </span>
                    <strong>{p.name}</strong>
                  </div>
                </td>
                <td className="package-description">
                  {p.description}
                  {detailed ? (
                    <small>
                      {p.license}
                      {p.pid ? ` · PID ${p.pid}` : ""}
                    </small>
                  ) : null}
                </td>
                <td className="version">{p.version}</td>
                <td>
                  <span
                    className={`service-status ${p.running ? "running" : ""}`}
                  >
                    <span
                      className={`status-dot ${p.running ? "green" : ""}`}
                    />
                    {p.running
                      ? "Çalışıyor"
                      : p.installed
                        ? "Kurulu"
                        : p.repairable
                          ? "Kurulum eksik"
                          : "Kurulu değil"}
                  </span>
                </td>
                <td className="table-action">
                  <div className="package-buttons">
                    {!p.installed && !p.repairable ? (
                      <button
                        disabled={busy}
                        className="button secondary small"
                        onClick={() =>
                          void run(`${p.name} kuruluyor…`, () =>
                            call("install", { id: p.id }),
                          )
                        }
                      >
                        <Download size={15} />
                        İndir
                      </button>
                    ) : !p.installed ? null : p.id === "phpmyadmin" ? (
                      <button
                        className="button secondary small"
                        disabled={busy || !databaseReady || !pmaEnabled}
                        title={
                          !pmaEnabled
                            ? "phpMyAdmin Hizmetler ekranında kapalı"
                            : databaseReady
                              ? "phpMyAdmin'i tarayıcıda aç"
                              : "Önce PHP, MySQL ve web sunucusunu başlatın"
                        }
                        onClick={() =>
                          void run("phpMyAdmin açılıyor…", () =>
                            call("open_phpmyadmin"),
                          )
                        }
                      >
                        <ExternalLink size={15} /> Aç
                      </button>
                    ) : p.id === "composer" ? (
                      <span className="installed-label">
                        <Check size={16} />
                        Hazır
                      </span>
                    ) : (
                      <button
                        disabled={busy}
                        className="button secondary small"
                        onClick={() =>
                          void run(
                            `${p.name} ${p.running ? "durduruluyor" : "başlatılıyor"}…`,
                            () =>
                              call("service", {
                                id: p.id,
                                action: p.running ? "stop" : "start",
                              }),
                          )
                        }
                      >
                        {p.running ? <Square size={13} /> : <Play size={14} />}{" "}
                        {p.running ? "Durdur" : "Başlat"}
                      </button>
                    )}
                    {p.repairable && (detailed || !p.installed) ? (
                      <button
                        className="button secondary small"
                        disabled={busy || running}
                        title={
                          running
                            ? "Onarmak için ortamı durdurun"
                            : "Program dosyalarını doğrulanmış paketten yeniden kur"
                        }
                        aria-label={`${p.name} onar`}
                        onClick={() =>
                          void run(`${p.name} onarılıyor…`, () =>
                            call("repair", { id: p.id }),
                          )
                        }
                      >
                        <Wrench size={14} /> Onar
                      </button>
                    ) : null}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {detailed &&
      packages.some((p) => p.id === "phpmyadmin" && p.installed) ? (
        <p className="section-note">
          phpMyAdmin girişi: kullanıcı <strong>root</strong>; parola Ayarlar →
          Sistem → MySQL bağlantısı bölümünde. F4Box'ın MySQL portu otomatik
          kullanılır. PHP, MySQL ve web sunucusu çalışırken açılabilir.
        </p>
      ) : null}
      {detailed ? (
        <p className="section-note">
          Paketler resmî kaynaklardan indirilir ve SHA-256 ile doğrulanır.
          Program dosyaları ve MySQL verileri ayrı klasörlerde saklanır. Onar
          işlemi için ortamı durdurun; önceki program klasörü saklanır.
        </p>
      ) : null}
    </section>
  );
}
