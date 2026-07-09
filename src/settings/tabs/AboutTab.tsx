import type { ReactElement } from "react";
import { Button } from "../../shared/components/Button";
import type { ScreenId } from "../../shared/lib/types";

export function AboutTab({ onNavigate }: { onNavigate: (screen: ScreenId) => void }): ReactElement {
  return (
    <div className="settings-section active">
      <div className="setting-row"><div className="setting-label">Version de Vitrail</div><span style={{ fontWeight: 500 }}>0.1.0-alpha</span></div>
      <div className="setting-row"><div className="setting-label">OpenSnitch</div><span style={{ fontWeight: 500 }}>non détecté (EPIC 1)</span></div>
      <div className="setting-row"><div className="setting-label">PolarProxy</div><span style={{ fontWeight: 500 }}>non détecté (EPIC 4)</span></div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Journal système</div>
          <div className="setting-desc">Logs bruts des sous-systèmes</div>
        </div>
        <Button size="sm" onClick={() => onNavigate("logs")}>Ouvrir</Button>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Confidentialité</div>
          <div className="setting-desc">Transparence sur le traitement des données</div>
        </div>
        <Button size="sm" onClick={() => onNavigate("privacy")}>Ouvrir</Button>
      </div>
    </div>
  );
}
