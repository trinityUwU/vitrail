import type { ReactElement } from "react";
import { Badge } from "../../shared/components/Badge";
import type { Settings } from "../../shared/lib/types";

export function NetworkTab({ settings }: { settings: Settings }): ReactElement {
  return (
    <div className="settings-section active">
      <div className="setting-row">
        <div>
          <div className="setting-label">Chaîne nftables</div>
          <div className="setting-desc">Nom fixe de la chaîne utilisée pour la redirection</div>
        </div>
        <span className="mono" style={{ fontSize: ".83rem", color: "var(--t2)", background: "var(--bg2)", padding: "4px 10px", borderRadius: "var(--r-s)" }}>
          {settings.nftablesChain}
        </span>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Interfaces surveillées</div>
          <div className="setting-desc">Interfaces réseau sur lesquelles le trafic est intercepté</div>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          {settings.monitoredInterfaces.map((iface) => (
            <Badge key={iface} variant="ok">{iface}</Badge>
          ))}
        </div>
      </div>
    </div>
  );
}
