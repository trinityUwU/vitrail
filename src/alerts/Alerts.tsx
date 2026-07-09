import type { ReactElement } from "react";
import { Bell, Plus } from "lucide-react";
import { Button } from "../shared/components/Button";
import { EmptyState } from "../shared/components/EmptyState";
import { useToast } from "../shared/hooks/useToast";
import { useAlerts } from "./useAlerts";
import { AlertRuleCard } from "./AlertRuleCard";
import "./Alerts.css";

export function Alerts(): ReactElement {
  const { rules, toggleRule } = useAlerts();
  const { showToast } = useToast();

  return (
    <div>
      <div className="screen-title">Alertes & Règles</div>
      <div className="screen-subtitle">Signalement proactif basé sur des critères personnalisés</div>
      <div style={{ marginBottom: 16 }}>
        <Button variant="primary" onClick={() => showToast("Fonctionnalité disponible dans la version complète")}>
          <Plus /> Créer une règle
        </Button>
      </div>
      {rules.length === 0 ? (
        <EmptyState
          icon={Bell}
          message="Aucune règle définie. Créez-en une pour être notifié proactivement lorsqu'un événement réseau correspond à vos critères."
        />
      ) : (
        rules.map((rule) => <AlertRuleCard key={rule.id} rule={rule} onToggle={(id) => void toggleRule(id)} />)
      )}
    </div>
  );
}
