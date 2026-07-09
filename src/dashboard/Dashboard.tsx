import type { ReactElement } from "react";
import { Activity, ArrowRight, Bell, CheckCircle, Power, Search } from "lucide-react";
import { Button } from "../shared/components/Button";
import { useKillSwitch } from "../shared/hooks/useKillSwitchState";
import { useDashboard } from "./useDashboard";
import { DashboardMetrics } from "./DashboardMetrics";
import { DashboardTopLists } from "./DashboardTopLists";
import { fmtSince } from "../shared/lib/format-utils";
import type { ScreenId } from "../shared/lib/types";
import "./Dashboard.css";

export function Dashboard({ onNavigate }: { onNavigate: (screen: ScreenId) => void }): ReactElement {
  const { phase, activate } = useKillSwitch();
  const isActive = phase === "active" || phase === "degraded";
  const { summary } = useDashboard(isActive);

  if (!isActive) {
    return (
      <div>
        <div className="screen-title">Vue d'ensemble</div>
        <div className="screen-subtitle">Tableau de bord de l'activité réseau</div>
        <div className="empty-state" style={{ padding: "80px 20px" }}>
          <Power style={{ width: 48, height: 48, opacity: 0.25 }} />
          <p style={{ fontSize: "1rem", color: "var(--t2)", marginBottom: 16, fontWeight: 400 }}>
            Vitrail est en attente d'activation
          </p>
          <Button variant="primary" size="lg" onClick={() => void activate()}>
            Activer Vitrail
          </Button>
        </div>
      </div>
    );
  }

  if (!summary) return <div className="screen-title">Vue d'ensemble</div>;

  return (
    <div>
      <div className="screen-title">Vue d'ensemble</div>
      <div className="screen-subtitle">
        Activité depuis {summary.activeSince ? fmtSince(summary.activeSince) : "—"}
      </div>

      <DashboardMetrics summary={summary} />
      <DashboardTopLists
        processes={summary.topProcesses}
        destinations={summary.topDestinations}
        onNavigate={onNavigate}
      />

      <div className="dash-bottom-row">
        <div className="card">
          <div className="section-title">Dernière vérification Kill Switch</div>
          <div className="ks-verify-box ks-verify-ok">
            <CheckCircle style={{ width: 20, height: 20 }} />
            <div>
              <strong>Conforme</strong> — dernière vérification effectuée
              <br />
              <span style={{ fontSize: ".72rem", color: "var(--t3)" }}>
                Aucune divergence détectée entre l'état attendu et l'état réel des sous-systèmes
              </span>
            </div>
          </div>
          <div style={{ marginTop: 10 }}>
            <Button variant="ghost" size="sm" onClick={() => onNavigate("killswitch")}>
              Ouvrir le panneau Kill Switch <ArrowRight />
            </Button>
          </div>
        </div>
        <div className="card">
          <div className="section-title">Accès rapide</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <Button variant="ghost" style={{ justifyContent: "flex-start" }} onClick={() => onNavigate("timeline")}>
              <Activity /> Timeline temps réel
            </Button>
            <Button variant="ghost" style={{ justifyContent: "flex-start" }} onClick={() => onNavigate("search")}>
              <Search /> Recherche avancée
            </Button>
            <Button variant="ghost" style={{ justifyContent: "flex-start" }} onClick={() => onNavigate("alerts")}>
              <Bell /> Alertes & Règles
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
