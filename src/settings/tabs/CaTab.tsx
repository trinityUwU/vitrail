import type { ReactElement } from "react";
import { Clipboard } from "lucide-react";
import { Badge } from "../../shared/components/Badge";
import { Button } from "../../shared/components/Button";
import { useToast } from "../../shared/hooks/useToast";
import { vitrailApi } from "../../shared/lib/vitrail-api";
import { logger } from "../../shared/lib/logger";
import type { Settings } from "../../shared/lib/types";

export function CaTab({ settings, onRotated }: { settings: Settings; onRotated: () => void }): ReactElement {
  const { showToast } = useToast();

  const handleRegen = async (): Promise<void> => {
    try {
      await vitrailApi.rotateCa();
      showToast("Régénération de la CA lancée — Vitrail sera temporairement désactivé");
      onRotated();
    } catch (error) {
      logger.error({ error }, "Échec de rotation de la CA");
    }
  };

  return (
    <div className="settings-section active">
      <div className="setting-row">
        <div>
          <div className="setting-label">Empreinte SHA-256 de la CA</div>
          <div className="setting-desc">Certificat racine utilisé pour le déchiffrement local</div>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span className="mono" style={{ fontSize: ".78rem", color: "var(--t2)" }}>{settings.caFingerprint}</span>
          <Button variant="ghost" size="sm" onClick={() => showToast("Copié dans le presse-papiers")}>
            <Clipboard />
          </Button>
        </div>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Statut trust store</div>
          <div className="setting-desc">Installation du certificat dans le magasin de confiance système</div>
        </div>
        <Badge variant={settings.caTrustStoreInstalled ? "ok" : "attrib"}>
          {settings.caTrustStoreInstalled ? "Installé" : "Non installé"}
        </Badge>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Régénérer la CA</div>
          <div className="setting-desc" style={{ color: "var(--danger)" }}>
            Attention : désactive Vitrail le temps de l'opération. Les applications utilisant l'ancienne CA
            devront être reconnectées.
          </div>
        </div>
        <Button variant="danger" size="sm" onClick={() => void handleRegen()}>Régénérer</Button>
      </div>
    </div>
  );
}
