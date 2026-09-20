import { useState } from "react";
import { ShieldCheck } from "lucide-react";
import { call } from "../api";
import type { PermissionState, Run } from "../types";

export default function PermissionSettings({
  permissions,
  busy,
  run,
}: {
  permissions: PermissionState;
  busy: boolean;
  run: Run;
}) {
  const [defender, setDefender] = useState(false);
  const pending = permissions.pending.length;
  return (
    <section className="settings-section permission-section">
      <h2>Windows izinleri</h2>
      <p className="section-note">
        F4Box gündelik işini yönetici yetkisi olmadan yapar: servisler yalnızca
        127.0.0.1 üzerinde dinler ve dosyalar kendi veri klasörüne yazılır. Tek
        seferlik izin, Windows’un sonradan soru sormasını engellemek içindir.
        Aşağıdaki işlem tek bir Windows onay penceresi açar.
      </p>
      <ul className="permission-list">
        <li>
          Güvenlik duvarında F4Box’ın çalıştırdığı programlar için özel ve etki
          alanı profillerinde izin kuralı
        </li>
        <li>Veri klasöründe Windows kullanıcınıza tam erişim</li>
        <li>İsteğe bağlı: Microsoft Defender’da veri klasörü istisnası</li>
      </ul>
      <p>
        {permissions.granted ? (
          <>
            Verildi · <strong>{permissions.appliedAt}</strong> ·{" "}
            {permissions.programs.length} program
            {permissions.defenderExclusion ? " · Defender istisnası var" : ""}
          </>
        ) : (
          <>Henüz tek seferlik izin verilmedi.</>
        )}
      </p>
      {pending > 0 && permissions.granted && (
        <p className="settings-feedback" role="status">
          {pending} yeni program izin listesinde değil (yeni kurulan PHP sürümü
          veya cloudflared). Yeniden izin verdiğinizde eklenir.
        </p>
      )}
      {permissions.failed.length > 0 && (
        <ul className="permission-list error" role="alert">
          {permissions.failed.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
      <label className="setting-toggle">
        <input
          type="checkbox"
          checked={defender}
          disabled={busy}
          onChange={(e) => setDefender(e.target.checked)}
        />
        <span>
          Microsoft Defender’ı veri klasöründe devre dışı bırak (Composer ve PHP
          hızlanır, tarama koruması bu klasörde kalkar)
        </span>
      </label>
      <button
        type="button"
        className="button primary"
        disabled={busy}
        onClick={() =>
          void run("Windows yetkisi isteniyor…", async () => {
            await call<PermissionState>("grant_permissions", { defender });
            return "İzin işlemi tamamlandı. Ayrıntılar bu kartta ve günlüklerde.";
          })
        }
      >
        <ShieldCheck size={17} />
        {permissions.granted ? "İzinleri yenile" : "Tek seferlik izin ver"}
      </button>
      {permissions.applied.length > 0 && (
        <ul className="permission-list done">
          {permissions.applied.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
    </section>
  );
}
