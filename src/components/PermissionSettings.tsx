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
  const [defender, setDefender] = useState(
    permissions.defenderExclusion || !permissions.granted,
  );
  const pending = permissions.pending.length;
  return (
    <section className="settings-section permission-section">
      <h2>Windows izinleri</h2>
      <p className="section-note">
        F4Box açılırken Windows’tan bir kez tam yetki ister. Onay, güvenlik
        duvarı kurallarını, veri klasörü erişimini ve isteğe bağlı Defender
        istisnasını uygular; ardından bir zamanlanmış görev kurulur. Uygulamanın
        kendisi yükseltilmiş yetkiyle çalışmaz. Sonraki açılışlarda ve yeni
        paket kurulumlarında Windows bir daha soru sormaz.
      </p>
      <ul className="permission-list">
        <li>
          Güvenlik duvarında F4Box’ın çalıştırdığı programlar için özel ve etki
          alanı profillerinde izin kuralı
        </li>
        <li>Veri klasöründe Windows kullanıcınıza tam erişim</li>
        <li>İsteğe bağlı: Microsoft Defender’da veri klasörü istisnası</li>
        <li>
          Tekrar sormamak için “F4Box Permissions” zamanlanmış görevi (en yüksek
          yetki)
        </li>
      </ul>
      <p>
        {permissions.granted ? (
          <>
            Verildi · <strong>{permissions.appliedAt}</strong> ·{" "}
            {permissions.programs.length} program
            {permissions.defenderExclusion ? " · Defender istisnası var" : ""}
            {permissions.helper ? " · yardımcı görev kurulu" : ""}
          </>
        ) : permissions.declined ? (
          <>
            Açılıştaki yetki isteği onaylanmadı. F4Box bir daha kendiliğinden
            sormaz; aşağıdaki düğmeyle yeniden isteyebilirsiniz.
          </>
        ) : (
          <>Henüz Windows onayı alınmadı. Sonraki açılışta bir kez sorulur.</>
        )}
      </p>
      {pending > 0 && permissions.granted && permissions.helper && (
        <p className="settings-feedback" role="status">
          {pending} yeni program bir sonraki sessiz yenilemede eklenecek. Yeni
          bir Windows penceresi açılmaz.
        </p>
      )}
      {pending > 0 && permissions.granted && !permissions.helper && (
        <p className="settings-feedback" role="status">
          {pending} yeni program izin listesinde değil. Aşağıdan yenilediğinizde
          yardımcı görev de kurulur; bundan sonra Windows soru sormaz.
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
        {permissions.granted
          ? "İzinleri yenile"
          : permissions.declined
            ? "Yeniden izin iste"
            : "Tek seferlik izin ver"}
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
