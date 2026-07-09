import type { ReactElement } from "react";
import { useState } from "react";
import { Clock, FileText, Trash2 } from "lucide-react";
import { Button } from "../shared/components/Button";
import { EmptyState } from "../shared/components/EmptyState";
import { useToast } from "../shared/hooks/useToast";
import { logger } from "../shared/lib/logger";
import { fmtDate, fmtVol } from "../shared/lib/format-utils";
import { useSessions } from "./useSessions";
import { useSessionDetail } from "./useSessionDetail";
import { SessionDetailView } from "./SessionDetailView";
import { downloadSessionReport } from "./history-report";
import { vitrailApi } from "../shared/lib/vitrail-api";

export function History(): ReactElement {
  const { sessions, deleteSession } = useSessions();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const { detail } = useSessionDetail(selectedId);
  const { showToast } = useToast();

  const handleReport = async (id: string): Promise<void> => {
    try {
      const sessionDetail = selectedId === id && detail ? detail : await vitrailApi.getSessionDetail(id);
      if (!sessionDetail) return;
      downloadSessionReport(sessionDetail);
      showToast("Export lancé");
    } catch (error) {
      logger.error({ error, id }, "Échec de génération du rapport");
    }
  };

  const handleDelete = async (id: string): Promise<void> => {
    await deleteSession(id);
    if (selectedId === id) setSelectedId(null);
    showToast("Action de purge effectuée");
  };

  if (sessions.length === 0) {
    return (
      <div>
        <div className="screen-title">Historique</div>
        <div className="screen-subtitle">Sessions passées — rétrospective</div>
        <EmptyState icon={Clock} message="Aucune session passée. Les sessions apparaîtront ici après chaque cycle d'activation/désactivation." />
      </div>
    );
  }

  return (
    <div>
      <div className="screen-title">Historique</div>
      <div className="screen-subtitle">Sessions passées — rétrospective</div>
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {sessions.map((s) => (
          <div key={s.id}>
            <div
              className="card"
              style={{ display: "flex", alignItems: "center", gap: 20, cursor: "pointer" }}
              onClick={() => setSelectedId((prev) => (prev === s.id ? null : s.id))}
            >
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 500, fontSize: ".9rem", marginBottom: 4 }}>{fmtDate(s.startedAt)}</div>
                <div style={{ fontSize: ".78rem", color: "var(--t3)" }}>Session du {fmtDate(s.startedAt)} au {fmtDate(s.endedAt)}</div>
              </div>
              <div style={{ display: "flex", gap: 24, textAlign: "center" }}>
                <div><div className="metric-value" style={{ fontSize: "1.1rem" }}>{fmtVol(s.volumeMb)}</div><div className="metric-label">Volume</div></div>
                <div><div className="metric-value" style={{ fontSize: "1.1rem" }}>{s.processCount}</div><div className="metric-label">Processus</div></div>
                <div><div className="metric-value" style={{ fontSize: "1.1rem", color: s.alertCount ? "var(--warn)" : "var(--t3)" }}>{s.alertCount}</div><div className="metric-label">Alertes</div></div>
              </div>
              <div style={{ display: "flex", gap: 6 }} onClick={(e) => e.stopPropagation()}>
                <Button size="sm" onClick={() => void handleReport(s.id)}><FileText /> Rapport</Button>
                <Button variant="ghost" size="sm" onClick={() => void handleDelete(s.id)}><Trash2 /></Button>
              </div>
            </div>
            {selectedId === s.id && detail && detail.session.id === s.id && <SessionDetailView detail={detail} />}
          </div>
        ))}
      </div>
    </div>
  );
}
