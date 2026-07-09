import type { ReactElement } from "react";
import { useState } from "react";
import { Button } from "../shared/components/Button";

export function EmergencyStop({ onConfirm }: { onConfirm: () => void }): ReactElement {
  const [armed, setArmed] = useState(false);

  const handleClick = (): void => {
    if (armed) {
      onConfirm();
      setArmed(false);
      return;
    }
    setArmed(true);
    setTimeout(() => setArmed(false), 3000);
  };

  return (
    <div className="ks-emergency">
      <div className="ks-emergency-title">Arrêt d'urgence</div>
      <div className="ks-emergency-desc">
        Force la désactivation immédiate de tous les sous-systèmes sans séquence orchestrée. À utiliser
        uniquement si la désactivation normale échoue.
      </div>
      <Button variant="danger" style={{ fontWeight: armed ? 700 : 500 }} onClick={handleClick}>
        {armed ? "Confirmer l'arrêt d'urgence" : "Arrêt d'urgence"}
      </Button>
    </div>
  );
}
