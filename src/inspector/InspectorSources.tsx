import type { ReactElement } from "react";
import type { CorrelationStatus, Flow } from "../shared/lib/types";

const STATUS_LABEL: Record<CorrelationStatus, string> = { ok: "OK", warn: "Partiel", off: "N/A" };
const STATUS_CLASS: Record<CorrelationStatus, string> = { ok: "ok", warn: "wait", off: "off" };

export function InspectorSources({ flow }: { flow: Flow }): ReactElement {
  return (
    <div className="card">
      <div className="section-title">Sources de corrélation</div>
      {flow.sources.map((s) => (
        <div key={s.name} style={{ display: "flex", alignItems: "center", gap: 10, padding: "8px 0", borderBottom: "1px solid var(--border-s)", fontSize: ".82rem" }}>
          <span className={`ks-sub-status ${STATUS_CLASS[s.status]}`}>{STATUS_LABEL[s.status]}</span>
          <span style={{ fontWeight: 500, width: 100 }}>{s.name}</span>
          <span style={{ color: "var(--t3)" }}>{s.detail}</span>
        </div>
      ))}
    </div>
  );
}
