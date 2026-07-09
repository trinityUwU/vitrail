import type { ReactElement } from "react";
import { useState } from "react";
import { Plus, X } from "lucide-react";
import { Button } from "../../shared/components/Button";
import { useExclusionsContext } from "../../shared/hooks/useExclusionsState";

export function ExclusionsTab(): ReactElement {
  const { exclusions, addExclusion, removeExclusion } = useExclusionsContext();
  const [name, setName] = useState("");
  const [type, setType] = useState("processus");

  const handleAdd = (): void => {
    if (!name.trim()) return;
    void addExclusion(name.trim(), type);
    setName("");
  };

  return (
    <div className="settings-section active">
      <div style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <input
          className="input"
          style={{ width: 280 }}
          placeholder="Processus ou domaine à exclure..."
          value={name}
          onChange={(e) => setName(e.target.value)}
        />
        <select className="input select" style={{ width: 130 }} value={type} onChange={(e) => setType(e.target.value)}>
          <option value="processus">Processus</option>
          <option value="domaine">Domaine</option>
        </select>
        <Button variant="primary" size="sm" onClick={handleAdd}>
          <Plus /> Ajouter
        </Button>
      </div>
      <ul className="excl-list">
        {exclusions.map((e) => (
          <li className="excl-item" key={e.name}>
            <span className="excl-item-name">{e.name}</span>
            <span className="excl-item-type">{e.type}</span>
            <Button variant="ghost" size="sm" onClick={() => void removeExclusion(e.name)}>
              <X />
            </Button>
          </li>
        ))}
      </ul>
    </div>
  );
}
