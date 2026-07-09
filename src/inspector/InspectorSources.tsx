import type { ReactElement } from "react";
import type { Flow } from "../shared/lib/types";

interface Source {
  name: string;
  status: "ok" | "warn" | "off";
  detail: string;
}

function buildSources(flow: Flow): Source[] {
  return [
    { name: "Attribution", status: "ok", detail: "OpenSnitch" },
    { name: "Capture", status: flow.visibility === "attrib" ? "off" : "ok", detail: flow.visibility === "attrib" ? "Non applicable" : "nftables redirect" },
    { name: "Décryptage", status: flow.visibility === "fully" ? "ok" : flow.visibility === "meta" ? "warn" : "off", detail: flow.visibility === "fully" ? "PolarProxy" : flow.visibility === "meta" ? "Échoué (pinning)" : "Non applicable" },
    { name: "Keylog", status: "off", detail: "Non utilisé pour ce flux" },
  ];
}

const STATUS_LABEL: Record<Source["status"], string> = { ok: "OK", warn: "Partiel", off: "N/A" };
const STATUS_CLASS: Record<Source["status"], string> = { ok: "ok", warn: "wait", off: "off" };

export function InspectorSources({ flow }: { flow: Flow }): ReactElement {
  const sources = buildSources(flow);
  return (
    <div className="card">
      <div className="section-title">Sources de corrélation</div>
      {sources.map((s) => (
        <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 0", borderBottom: "1px solid var(--border-s)", fontSize: ".82rem" }}>
          <span className={`ks-sub-status ${STATUS_CLASS[s.status]}`}>{STATUS_LABEL[s.status]}</span>
          <span style={{ fontWeight: 500, width: 100 }}>{s.name}</span>
          <span style={{ color: "var(--t3)" }}>{s.detail}</span>
        </div>
      ))}
    </div>
  );
}
