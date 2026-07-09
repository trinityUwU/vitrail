import type { ReactElement } from "react";
import { useState } from "react";
import { Check, X } from "lucide-react";
import { Button } from "../shared/components/Button";
import type { AlertRule } from "../shared/lib/types";

interface AlertRuleFormProps {
  initial?: AlertRule;
  onSubmit: (name: string, description: string, criteria: string) => void;
  onCancel: () => void;
}

export function AlertRuleForm({ initial, onSubmit, onCancel }: AlertRuleFormProps): ReactElement {
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [criteria, setCriteria] = useState(initial?.criteria ?? "");

  const handleSubmit = (): void => {
    if (!name.trim() || !criteria.trim()) return;
    onSubmit(name.trim(), description.trim(), criteria.trim());
  };

  return (
    <div className="card" style={{ marginBottom: 12 }}>
      <div style={{ display: "grid", gap: 10 }}>
        <input className="input" placeholder="Nom de la règle..." value={name} onChange={(e) => setName(e.target.value)} autoFocus />
        <input className="input" placeholder="Description..." value={description} onChange={(e) => setDescription(e.target.value)} />
        <input className="input" placeholder="Critère (ex: processus = nouveau)..." value={criteria} onChange={(e) => setCriteria(e.target.value)} />
        <div style={{ display: "flex", gap: 8 }}>
          <Button variant="primary" size="sm" onClick={handleSubmit}>
            <Check /> Enregistrer
          </Button>
          <Button variant="ghost" size="sm" onClick={onCancel}>
            <X /> Annuler
          </Button>
        </div>
      </div>
    </div>
  );
}
