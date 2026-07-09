import type { ReactElement } from "react";
import { ArrowDown, ArrowUp, EyeOff, Radio } from "lucide-react";
import type { DashboardSummary } from "../shared/lib/types";

export function DashboardMetrics({ summary }: { summary: DashboardSummary }): ReactElement {
  return (
    <div className="dash-metrics">
      <div className="dash-metric">
        <div className="dash-metric-icon" style={{ background: "var(--accent-l)", color: "var(--accent)" }}>
          <Radio />
        </div>
        <div className="metric-value">{summary.activeConnections}</div>
        <div className="metric-label">Connexions actives</div>
      </div>
      <div className="dash-metric">
        <div className="dash-metric-icon" style={{ background: "#EDF4FF", color: "#3B6FD4" }}>
          <ArrowDown />
        </div>
        <div className="metric-value">{summary.totalInMb.toFixed(1)} Mo</div>
        <div className="metric-label">Débit entrant cumulé</div>
      </div>
      <div className="dash-metric">
        <div className="dash-metric-icon" style={{ background: "#FFF4ED", color: "#D4823B" }}>
          <ArrowUp />
        </div>
        <div className="metric-value">{summary.totalOutMb.toFixed(1)} Mo</div>
        <div className="metric-label">Débit sortant cumulé</div>
      </div>
      <div className="dash-metric">
        <div className="dash-metric-icon" style={{ background: "var(--meta-l)", color: "var(--meta)" }}>
          <EyeOff />
        </div>
        <div className="metric-value">{summary.metaOnlyCount}</div>
        <div className="metric-label">Flux en métadonnées seules</div>
      </div>
    </div>
  );
}
