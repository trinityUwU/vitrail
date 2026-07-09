import type { ReactElement } from "react";
import { Activity, Ban } from "lucide-react";
import { Button } from "../shared/components/Button";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { fmtVol } from "../shared/lib/format-utils";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import { useToast } from "../shared/hooks/useToast";
import type { ProcessInfo, ScreenId } from "../shared/lib/types";

interface ProcessDetailPanelProps {
  process: ProcessInfo;
  onNavigate: (screen: ScreenId) => void;
}

export function ProcessDetailPanel({ process, onNavigate }: ProcessDetailPanelProps): ReactElement {
  const { showToast } = useToast();

  const handleExclude = async (): Promise<void> => {
    try {
      await vitrailApi.addExclusion(process.name, "processus");
      showToast(`${process.name} ajouté aux exclusions`);
    } catch (error) {
      logger.error({ error }, "Échec d'ajout d'exclusion");
    }
  };

  return (
    <div className="card" style={{ marginTop: 20 }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <div>
          <div style={{ fontWeight: 600, fontSize: "1rem" }}>{process.name}</div>
          <div className="mono" style={{ fontSize: ".75rem", color: "var(--t3)" }}>{process.path}</div>
        </div>
        <VisibilityBadge visibility={process.visibility} />
      </div>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12, marginBottom: 16 }}>
        <div className="insp-field"><div className="insp-field-label">Volume total</div><div className="insp-field-val">{fmtVol(process.volumeMb)}</div></div>
        <div className="insp-field"><div className="insp-field-label">Destinations</div><div className="insp-field-val">{process.destinations}</div></div>
        <div className="insp-field"><div className="insp-field-label">PIDs actifs</div><div className="insp-field-val mono">{process.pids.join(", ")}</div></div>
        <div className="insp-field"><div className="insp-field-label">Keylog</div><div className="insp-field-val">{process.keylogCovered ? "Oui" : "Non"}</div></div>
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <Button size="sm" onClick={() => onNavigate("timeline")}>
          <Activity /> Voir dans la Timeline
        </Button>
        <Button size="sm" onClick={() => void handleExclude()}>
          <Ban /> Exclure du MITM
        </Button>
      </div>
    </div>
  );
}
