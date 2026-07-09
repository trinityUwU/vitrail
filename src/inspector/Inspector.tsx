import type { ReactElement } from "react";
import { ArrowLeft, Clipboard, Download, ShieldCheck } from "lucide-react";
import { Button } from "../shared/components/Button";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { fmtDur, fmtSize } from "../shared/lib/format-utils";
import { logger } from "../shared/lib/logger";
import { useToast } from "../shared/hooks/useToast";
import { useFlowInspector } from "./useFlowInspector";
import { InspectorContent } from "./InspectorContent";
import { InspectorSources } from "./InspectorSources";
import { copyFlowHeaders, downloadFlowJson } from "./inspector-actions";
import type { ScreenId } from "../shared/lib/types";
import "./Inspector.css";

interface InspectorProps {
  flowId: string | null;
  onBack: () => void;
  onSelectProcess: (name: string) => void;
  onSelectDestination: (domain: string) => void;
  onNavigate: (screen: ScreenId) => void;
}

export function Inspector(props: InspectorProps): ReactElement {
  const { flowId, onBack, onSelectProcess, onSelectDestination, onNavigate } = props;
  const { flow } = useFlowInspector(flowId);
  const { showToast } = useToast();

  if (!flow) {
    return (
      <div className="empty-state">
        <p>Aucun flux sélectionné</p>
      </div>
    );
  }

  const handleCopyHeaders = async (): Promise<void> => {
    try {
      await copyFlowHeaders(flow);
      showToast("Copié dans le presse-papiers");
    } catch (error) {
      logger.error({ error }, "Échec de copie des headers");
    }
  };

  const handleExportJson = (): void => {
    try {
      downloadFlowJson(flow);
      showToast("Export lancé");
    } catch (error) {
      logger.error({ error }, "Échec d'export du flux");
    }
  };

  return (
    <div>
      <div style={{ marginBottom: 16 }}>
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft /> Retour
        </Button>
      </div>
      <div className="insp-header">
        <div>
          <div className="screen-title">Inspecteur de flux</div>
          <div className="screen-subtitle" style={{ marginBottom: 0 }}>{flow.process} → {flow.destination}</div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <Button size="sm" onClick={() => void handleCopyHeaders()}>
            <Clipboard /> Copier headers
          </Button>
          <Button size="sm" onClick={handleExportJson}>
            <Download /> Exporter JSON
          </Button>
        </div>
      </div>

      <div className="insp-tuple">
        <div className="insp-field"><div className="insp-field-label">IP source</div><div className="insp-field-val mono">{flow.sourceIp}</div></div>
        <div className="insp-field"><div className="insp-field-label">Port source</div><div className="insp-field-val mono">{flow.sourcePort}</div></div>
        <div className="insp-field"><div className="insp-field-label">IP destination</div><div className="insp-field-val mono">{flow.ip}</div></div>
        <div className="insp-field"><div className="insp-field-label">Port destination</div><div className="insp-field-val mono">{flow.port}</div></div>
        <div className="insp-field"><div className="insp-field-label">Protocole</div><div className="insp-field-val">{flow.protocol}</div></div>
      </div>

      <div className="card" style={{ marginBottom: 16 }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
          <div><span className="insp-field-label">Début</span><div className="mono" style={{ fontSize: ".85rem", marginTop: 2 }}>{flow.timestamp}</div></div>
          <div><span className="insp-field-label">Durée</span><div className="mono" style={{ fontSize: ".85rem", marginTop: 2 }}>{fmtDur(flow.durationMs)}</div></div>
          <div><span className="insp-field-label">Processus</span><div style={{ marginTop: 2 }}>
            <a href="#" style={{ color: "var(--accent)", textDecoration: "none", fontWeight: 500 }} onClick={(e) => { e.preventDefault(); onSelectProcess(flow.process); onNavigate("processes"); }}>{flow.process}</a>
          </div></div>
          <div><span className="insp-field-label">Destination</span><div style={{ marginTop: 2 }}>
            <a href="#" style={{ color: "var(--accent)", textDecoration: "none", fontWeight: 500 }} onClick={(e) => { e.preventDefault(); onSelectDestination(flow.destination); onNavigate("destinations"); }}>{flow.destination}</a>
          </div></div>
          <div><span className="insp-field-label">Taille</span><div className="mono" style={{ fontSize: ".85rem", marginTop: 2 }}>{fmtSize(flow.sizeBytes)}</div></div>
          <div><span className="insp-field-label">Visibilité</span><div style={{ marginTop: 2 }}><VisibilityBadge visibility={flow.visibility} /></div></div>
        </div>
      </div>

      <InspectorContent flow={flow} />

      {flow.certificate && (
        <div className="card" style={{ marginTop: 16 }}>
          <div className="section-title" style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <ShieldCheck style={{ width: 16, height: 16, color: "var(--ok)" }} /> Certificat vu
          </div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 16 }}>
            <div><span className="insp-field-label">Émetteur</span><div style={{ fontSize: ".85rem", marginTop: 2 }}>{flow.certificate.issuer}</div></div>
            <div><span className="insp-field-label">Sujet</span><div className="mono" style={{ fontSize: ".85rem", marginTop: 2 }}>{flow.certificate.subject}</div></div>
            <div><span className="insp-field-label">Valide du</span><div className="mono" style={{ fontSize: ".85rem", marginTop: 2 }}>{flow.certificate.validFrom}</div></div>
            <div><span className="insp-field-label">Valide au</span><div className="mono" style={{ fontSize: ".85rem", marginTop: 2 }}>{flow.certificate.validTo}</div></div>
            <div style={{ gridColumn: "1 / -1" }}><span className="insp-field-label">Empreinte SHA-256</span><div className="mono" style={{ fontSize: ".8rem", marginTop: 2 }}>{flow.certificate.fingerprintSha256}</div></div>
          </div>
        </div>
      )}

      <div style={{ marginTop: 16 }}>
        <InspectorSources flow={flow} />
      </div>
    </div>
  );
}
