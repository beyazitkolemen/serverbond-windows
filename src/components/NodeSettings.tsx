import { Download } from "lucide-react";
import { nodeService } from "../services";
import type { NodeState, Run } from "../types";
import ServiceRepair from "./ServiceRepair";

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
              : node.repairable
                ? "Kurulum eksik"
                : "Node.js kurulu değil"}
          </strong>
          <p className="section-note">
            {node.directory ??
              "Kurulumdan sonra proje terminalinde kullanılır."}
          </p>
        </div>
      </div>
      {node.issue && node.installed ? (
        <p role="alert" className="settings-feedback">
          {node.issue}
        </p>
      ) : null}
      {!node.installed && !node.repairable ? (
        <div className="settings-actions">
          <button
            type="button"
            className="button secondary"
            disabled={busy}
            onClick={() =>
              void run("Node.js indiriliyor…", () => nodeService.install())
            }
          >
            <Download size={16} />
            Node.js kur
          </button>
        </div>
      ) : null}
      <ServiceRepair
        name="Node.js"
        installed={node.installed}
        repairable={node.repairable}
        issue={node.issue}
        busy={busy}
        run={run}
        keeps={[
          "Sistem PATH",
          "Proje node_modules ve .env dosyaları",
          "Açık terminaller (yeniden açın)",
        ]}
        action={() => nodeService.repair()}
      />
      <p className="section-note">
        Kurduktan sonra proje kartındaki <strong>Terminal</strong> düğmesiyle
        açılan pencerede <code>npm install</code> ve <code>npm run dev</code>{" "}
        çalışır. Açık terminalleri kapatıp yeniden açın. Komut satırından:{" "}
        <code>f4box node install|repair|status</code>.
      </p>
    </section>
  );
}
