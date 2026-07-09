import type { ReactElement } from "react";
import { useState } from "react";
import { Pencil, Trash2 } from "lucide-react";
import { Toggle } from "../shared/components/Toggle";
import { Badge } from "../shared/components/Badge";
import { Button } from "../shared/components/Button";
import { AlertRuleForm } from "./AlertRuleForm";
import { useAlertEvents } from "./useAlertEvents";
import type { AlertRule } from "../shared/lib/types";

interface AlertRuleCardProps {
  rule: AlertRule;
  onToggle: (id: string) => void;
  onUpdate: (id: string, name: string, description: string, criteria: string) => void;
  onDelete: (id: string) => void;
}

export function AlertRuleCard({ rule, onToggle, onUpdate, onDelete }: AlertRuleCardProps): ReactElement {
  const [editing, setEditing] = useState(false);
  const { events } = useAlertEvents(rule.id);

  if (editing) {
    return (
      <AlertRuleForm
        initial={rule}
        onSubmit={(name, description, criteria) => {
          onUpdate(rule.id, name, description, criteria);
          setEditing(false);
        }}
        onCancel={() => setEditing(false)}
      />
    );
  }

  return (
    <div className="alert-rule">
      <div className="alert-rule-header">
        <Toggle on={rule.active} onToggle={() => onToggle(rule.id)} label="Activer/désactiver la règle" />
        <span className="alert-rule-name">{rule.name}</span>
        {rule.active ? <Badge variant="ok">Active</Badge> : <Badge variant="attrib">Désactivée</Badge>}
        <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
          <Button variant="ghost" size="sm" onClick={() => setEditing(true)}>
            <Pencil />
          </Button>
          <Button variant="ghost" size="sm" onClick={() => onDelete(rule.id)}>
            <Trash2 />
          </Button>
        </div>
      </div>
      <div style={{ fontSize: ".78rem", color: "var(--t3)", marginBottom: 8 }}>
        {rule.description}
        <br />
        <span className="mono" style={{ fontSize: ".72rem" }}>Critère : {rule.criteria}</span>
      </div>
      {events.length === 0 ? (
        <div style={{ fontSize: ".78rem", color: "var(--t4)", fontStyle: "italic" }}>Aucun déclenchement</div>
      ) : (
        events.map((e) => (
          <div className="alert-trigger" key={e.id}>
            <span className="alert-trigger-time mono">{e.time}</span>
            <span>{e.summary}</span>
          </div>
        ))
      )}
    </div>
  );
}
