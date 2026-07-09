import type { ReactElement } from "react";
import { EyeOff, Info } from "lucide-react";
import { fmtDur, fmtSize } from "../shared/lib/format-utils";
import type { Flow } from "../shared/lib/types";

function statusClass(status: number): string {
  if (status >= 500) return "http-status-5";
  if (status >= 400) return "http-status-4";
  if (status >= 300) return "http-status-3";
  return "http-status-2";
}

function statusLabel(status: number): string {
  if (status === 200) return "OK";
  if (status === 206) return "Partial Content";
  return "Moved";
}

export function InspectorContent({ flow }: { flow: Flow }): ReactElement {
  if (flow.visibility === "fully" && flow.method) {
    const status = flow.status ?? 200;
    return (
      <div style={{ marginBottom: 16 }}>
        <div className="section-title">Contenu déchiffré — Requête / Réponse HTTP</div>
        <div className="http-block">
          <span className="http-method">{flow.method}</span> <span className="http-path">{flow.path}</span> HTTP/1.1{"\n"}
          Host: {flow.destination}{"\n"}
          Accept: application/json, text/plain, */*{"\n"}
          Connection: keep-alive{"\n\n"}
          HTTP/1.1 <span className={`http-status ${statusClass(status)}`}>{status} {statusLabel(status)}</span>{"\n"}
          Content-Type: application/json; charset=utf-8{"\n"}
          Content-Length: {flow.sizeBytes}{"\n"}
          X-Request-Id: {flow.id}-req{"\n\n"}
          {`{"timestamp":"${flow.timestamp}","data":[...]}`}
        </div>
      </div>
    );
  }

  if (flow.visibility === "meta") {
    return (
      <div className="card" style={{ borderColor: "rgba(192,123,42,.25)", background: "var(--meta-l)" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
          <EyeOff style={{ color: "var(--meta)" }} />
          <strong style={{ color: "var(--meta)" }}>Pinning détecté — contenu non visible</strong>
        </div>
        <p style={{ fontSize: ".82rem", color: "var(--t2)", lineHeight: 1.6 }}>
          Le serveur distant utilise un certificat épinglé (certificate pinning). Le MITM local ne peut
          pas déchiffrer ce flux. Seules les métadonnées (SNI, taille, durée) sont disponibles.
        </p>
        <div style={{ marginTop: 12, display: "flex", gap: 16, fontSize: ".8rem" }}>
          <span><strong>SNI :</strong> <span className="mono">{flow.destination}</span></span>
          <span><strong>Taille :</strong> <span className="mono">{fmtSize(flow.sizeBytes)}</span></span>
          <span><strong>Durée :</strong> <span className="mono">{fmtDur(flow.durationMs)}</span></span>
        </div>
      </div>
    );
  }

  return (
    <div className="card" style={{ background: "var(--bg2)" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <Info style={{ color: "var(--t3)" }} />
        <strong style={{ color: "var(--t2)" }}>Flux attribué sans TLS</strong>
      </div>
      <p style={{ fontSize: ".82rem", color: "var(--t3)" }}>
        Ce flux n'utilise pas TLS. Seules les métadonnées de couche réseau sont disponibles.
      </p>
    </div>
  );
}
