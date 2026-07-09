import type { ReactElement } from "react";
import { Clock, FileText, Trash2 } from "lucide-react";
import { Button } from "../shared/components/Button";
import { EmptyState } from "../shared/components/EmptyState";
import { useToast } from "../shared/hooks/useToast";
import { fmtDate, fmtVol } from "../shared/lib/format-utils";
import { useSessions } from "./useSessions";

export function History(): ReactElement {
  const { sessions } = useSessions();
  const { showToast } = useToast();

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
          <div className="card" style={{ display: "flex", alignItems: "center", gap: 20 }} key={s.id}>
            <div style={{ flex: 1 }}>
              <div style={{ fontWeight: 500, fontSize: ".9rem", marginBottom: 4 }}>{fmtDate(s.startedAt)}</div>
              <div style={{ fontSize: ".78rem", color: "var(--t3)" }}>Session du {fmtDate(s.startedAt)} au {fmtDate(s.endedAt)}</div>
            </div>
            <div style={{ display: "flex", gap: 24, textAlign: "center" }}>
              <div><div className="metric-value" style={{ fontSize: "1.1rem" }}>{fmtVol(s.volumeMb)}</div><div className="metric-label">Volume</div></div>
              <div><div className="metric-value" style={{ fontSize: "1.1rem" }}>{s.processCount}</div><div className="metric-label">Processus</div></div>
              <div><div className="metric-value" style={{ fontSize: "1.1rem", color: s.alertCount ? "var(--warn)" : "var(--t3)" }}>{s.alertCount}</div><div className="metric-label">Alertes</div></div>
            </div>
            <div style={{ display: "flex", gap: 6 }}>
              <Button size="sm" onClick={() => showToast("Export lancé")}><FileText /> Rapport</Button>
              <Button variant="ghost" size="sm" onClick={() => showToast("Action de purge effectuée")}><Trash2 /></Button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
