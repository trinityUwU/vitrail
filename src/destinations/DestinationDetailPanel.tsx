import type { ReactElement } from "react";
import { Activity, Ban, Tag } from "lucide-react";
import { Button } from "../shared/components/Button";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { fmtVol } from "../shared/lib/format-utils";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import { useToast } from "../shared/hooks/useToast";
import type { DestinationInfo, ScreenId } from "../shared/lib/types";

interface DestinationDetailPanelProps {
  destination: DestinationInfo;
  onNavigate: (screen: ScreenId) => void;
}

export function DestinationDetailPanel({ destination, onNavigate }: DestinationDetailPanelProps): ReactElement {
  const { showToast } = useToast();

  const handleExclude = async (): Promise<void> => {
    try {
      await vitrailApi.addExclusion(destination.domain, "domaine");
      showToast(`${destination.domain} ajouté aux exclusions`);
    } catch (error) {
      logger.error({ error }, "Échec d'ajout d'exclusion");
    }
  };

  return (
    <div className="card" style={{ marginTop: 20 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <div>
          <div style={{ fontWeight: 600, fontSize: "1rem" }}>{destination.domain}</div>
          <div className="mono" style={{ fontSize: ".75rem", color: "var(--t3)" }}>{destination.ip}</div>
        </div>
        <VisibilityBadge visibility={destination.visibility} />
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12, marginBottom: 16 }}>
        <div className="insp-field"><div className="insp-field-label">Volume total</div><div className="insp-field-val">{fmtVol(destination.volumeMb)}</div></div>
        <div className="insp-field"><div className="insp-field-label">Processus</div><div className="insp-field-val">{destination.processCount}</div></div>
        <div className="insp-field"><div className="insp-field-label">TLS</div><div className="insp-field-val">{destination.tls ? "Oui" : "Non"}</div></div>
        <div className="insp-field"><div className="insp-field-label">Pinning</div><div className="insp-field-val">{destination.pinning ? "Oui" : "Non"}</div></div>
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <Button size="sm" onClick={() => onNavigate("timeline")}>
          <Activity /> Voir dans la Timeline
        </Button>
        <Button size="sm" onClick={() => void handleExclude()}>
          <Ban /> Exclure
        </Button>
        <Button size="sm" onClick={() => showToast("Fonctionnalité disponible dans la version complète")}>
          <Tag /> Taguer
        </Button>
      </div>
    </div>
  );
}
