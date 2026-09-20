import { Download, Wrench } from "lucide-react";
import { call } from "../api";
import type { NodeState, Run } from "../types";

export default function NodeSettings({
  node,
  busy,
  run,
}: {
  node: NodeState;
  busy: boolean;
  run: Run;
}) {
  return (
    <section className="settings-section">
      <h2>Node.js</h2>
      <p className="section-note">
        Laravel ön yüzü için <code>npm</code> ve <code>npx</code> gerekir. Sabit
        Node.js {node.version} LTS paketi resmî <code>SHASUMS256.txt</code>{" "}
        özetiyle doğrulanarak F4Box klasörüne kurulur. Sistem PATH'i
        değiştirilmez; sürüm yalnızca proje terminallerinde görünür.
      </p>
      <div className="tunnel-status">
        <span className={`status-dot ${node.installed ? "on" : "off"}`} />
        <div>
          <strong>
            {node.installed
              ? `Node.js ${node.version} kurulu`
              : "Node.js kurulu değil"}
          </strong>
          <p className="section-note">
            {node.directory ??
              "Kurulumdan sonra proje terminalinde kullanılır."}
          </p>
        </div>
      </div>
      <div className="settings-actions">
        {!node.installed ? (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() =>
              void run("Node.js indiriliyor…", () =>
                call("node", { action: "install" }),
              )
            }
          >
            <Download size={16} />
            Node.js kur
          </button>
        ) : (
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() =>
              void run("Node.js onarılıyor…", () =>
                call("node", { action: "repair" }),
              )
            }
          >
            <Wrench size={16} />
            Onar
          </button>
        )}
      </div>
      <p className="section-note">
        Kurduktan sonra proje kartındaki <strong>Terminal</strong> düğmesiyle
        açılan pencerede <code>npm install</code> ve <code>npm run dev</code>{" "}
        çalışır. Açık terminalleri kapatıp yeniden açın. Komut satırından:{" "}
        <code>f4box node install|repair|status</code>.
      </p>
    </section>
  );
}
