import type { ReactElement } from "react";
import { useState } from "react";
import { Bell, Plus } from "lucide-react";
import { Button } from "../shared/components/Button";
import { EmptyState } from "../shared/components/EmptyState";
import { useAlerts } from "./useAlerts";
import { AlertRuleCard } from "./AlertRuleCard";
import { AlertRuleForm } from "./AlertRuleForm";
import "./Alerts.css";

export function Alerts(): ReactElement {
  const { rules, toggleRule, createRule, updateRule, deleteRule } = useAlerts();
  const [creating, setCreating] = useState(false);

  return (
    <div>
      <div className="screen-title">Alertes & Règles</div>
      <div className="screen-subtitle">Signalement proactif basé sur des critères personnalisés</div>
      <div style={{ marginBottom: 16 }}>
        <Button variant="primary" onClick={() => setCreating((v) => !v)}>
          <Plus /> Créer une règle
        </Button>
      </div>
      {creating && (
        <AlertRuleForm
          onSubmit={(name, description, criteria) => {
            void createRule(name, description, criteria);
            setCreating(false);
          }}
          onCancel={() => setCreating(false)}
        />
      )}
      {rules.length === 0 ? (
        <EmptyState
          icon={Bell}
          message="Aucune règle définie. Créez-en une pour être notifié proactivement lorsqu'un événement réseau correspond à vos critères."
        />
      ) : (
        rules.map((rule) => (
          <AlertRuleCard
            key={rule.id}
            rule={rule}
            onToggle={(id) => void toggleRule(id)}
            onUpdate={(id, name, description, criteria) => void updateRule(id, name, description, criteria)}
            onDelete={(id) => void deleteRule(id)}
          />
        ))
      )}
    </div>
  );
}
