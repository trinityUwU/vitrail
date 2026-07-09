import type { ReactElement } from "react";
import { Pencil, Trash2 } from "lucide-react";
import { Toggle } from "../shared/components/Toggle";
import { Badge } from "../shared/components/Badge";
import { Button } from "../shared/components/Button";
import { useToast } from "../shared/hooks/useToast";
import type { AlertRule } from "../shared/lib/types";

interface AlertRuleCardProps {
  rule: AlertRule;
  onToggle: (id: string) => void;
}

export function AlertRuleCard({ rule, onToggle }: AlertRuleCardProps): ReactElement {
  const { showToast } = useToast();

  return (
    <div className="alert-rule">
      <div className="alert-rule-header">
        <Toggle on={rule.active} onToggle={() => onToggle(rule.id)} label="Activer/désactiver la règle" />
        <span className="alert-rule-name">{rule.name}</span>
        {rule.active ? <Badge variant="ok">Active</Badge> : <Badge variant="attrib">Désactivée</Badge>}
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          <Button variant="ghost" size="sm" onClick={() => showToast("Fonctionnalité disponible dans la version complète")}>
            <Pencil />
          </Button>
          <Button variant="ghost" size="sm" onClick={() => showToast("Fonctionnalité disponible dans la version complète")}>
            <Trash2 />
          </Button>
        </div>
      </div>
      <div style={{ fontSize: ".78rem", color: "var(--t3)", marginBottom: 8 }}>
        {rule.description}
        <br />
        <span className="mono" style={{ fontSize: ".72rem" }}>Critère : {rule.criteria}</span>
      </div>
      {rule.triggerCount === 0 ? (
        <div style={{ fontSize: ".78rem", color: "var(--t4)", fontStyle: "italic" }}>Aucun déclenchement</div>
      ) : (
        <div style={{ fontSize: ".72rem", fontWeight: 600, color: "var(--t3)" }}>
          {rule.triggerCount} déclenchement(s) enregistré(s)
        </div>
      )}
    </div>
  );
}
