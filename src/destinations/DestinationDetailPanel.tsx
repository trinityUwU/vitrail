import type { ReactElement } from "react";
import { useState } from "react";
import { Activity, Ban, Check, Tag } from "lucide-react";
import { Button } from "../shared/components/Button";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { fmtVol } from "../shared/lib/format-utils";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import { useToast } from "../shared/hooks/useToast";
import { useExclusionsContext } from "../shared/hooks/useExclusionsState";
import type { DestinationInfo, ScreenId } from "../shared/lib/types";

interface DestinationDetailPanelProps {
  destination: DestinationInfo;
  onNavigate: (screen: ScreenId) => void;
}

export function DestinationDetailPanel({ destination, onNavigate }: DestinationDetailPanelProps): ReactElement {
  const { showToast } = useToast();
  const { addExclusion } = useExclusionsContext();
  const [tag, setTag] = useState(destination.tag);
  const [tagInput, setTagInput] = useState("");
  const [tagging, setTagging] = useState(false);

  const handleExclude = async (): Promise<void> => {
    const ok = await addExclusion(destination.domain, "domaine");
    if (ok) showToast(`${destination.domain} ajouté aux exclusions`);
  };

  const handleTag = async (): Promise<void> => {
    if (!tagInput.trim()) return;
    try {
      const updated = await vitrailApi.tagDestination(destination.domain, tagInput.trim());
      setTag(updated.tag);
      setTagInput("");
      setTagging(false);
      showToast(`${destination.domain} taguée "${updated.tag ?? tagInput}"`);
    } catch (error) {
      logger.error({ error }, "Échec de tag de destination");
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
      {tag && !tagging && (
        <div style={{ marginBottom: 12, fontSize: ".8rem", color: "var(--t2)" }}>
          <strong>Tag :</strong> {tag}
        </div>
      )}
      {tagging ? (
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <input
            className="input"
            style={{ width: 220 }}
            placeholder="Nom du tag..."
            value={tagInput}
            onChange={(e) => setTagInput(e.target.value)}
            autoFocus
          />
          <Button variant="primary" size="sm" onClick={() => void handleTag()}>
            <Check /> Confirmer
          </Button>
        </div>
      ) : null}
      <div style={{ display: "flex", gap: 8 }}>
        <Button size="sm" onClick={() => onNavigate("timeline")}>
          <Activity /> Voir dans la Timeline
        </Button>
        <Button size="sm" onClick={() => void handleExclude()}>
          <Ban /> Exclure
        </Button>
        <Button size="sm" onClick={() => setTagging((v) => !v)}>
          <Tag /> {tag ? "Modifier le tag" : "Taguer"}
        </Button>
      </div>
    </div>
  );
}
