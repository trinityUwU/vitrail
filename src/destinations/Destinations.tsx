import type { ReactElement } from "react";
import { useState } from "react";
import { TableWrap } from "../shared/components/Table";
import { Badge } from "../shared/components/Badge";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { Button } from "../shared/components/Button";
import { fmtVol } from "../shared/lib/format-utils";
import { useDestinations } from "./useDestinations";
import { DestinationDetailPanel } from "./DestinationDetailPanel";
import type { ScreenId } from "../shared/lib/types";

export function Destinations({ onNavigate }: { onNavigate: (screen: ScreenId) => void }): ReactElement {
  const { destinations } = useDestinations();
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);

  const filtered = destinations.filter(
    (d) => d.domain.toLowerCase().includes(filter.toLowerCase()) || d.ip.includes(filter),
  );
  const selectedDestination = destinations.find((d) => d.domain === selected) ?? null;

  return (
    <div>
      <div className="screen-title">Destinations</div>
      <div className="screen-subtitle">Domaines et adresses IP contactés</div>
      <input
        className="input"
        style={{ width: 280, marginBottom: 20 }}
        placeholder="Filtrer par domaine ou IP..."
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      <TableWrap>
        <table>
          <thead>
            <tr>
              <th>Domaine / IP</th><th>Volume</th><th>Processus</th><th>Visibilité</th><th>TLS</th>
              <th>Pinning</th><th>Première</th><th>Dernière</th><th></th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((d) => (
              <tr key={d.domain}>
                <td>
                  <span className="mono" style={{ fontWeight: 500, color: "var(--t1)" }}>{d.domain}</span>
                  <br />
                  <span className="mono" style={{ fontSize: ".7rem", color: "var(--t3)" }}>{d.ip}</span>
                </td>
                <td style={{ fontWeight: 500 }}>{fmtVol(d.volumeMb)}</td>
                <td>{d.processCount}</td>
                <td><VisibilityBadge visibility={d.visibility} /></td>
                <td>{d.tls ? <Badge variant="ok">Oui</Badge> : <Badge variant="attrib">Non</Badge>}</td>
                <td>{d.pinning ? <Badge variant="meta">Oui</Badge> : <Badge variant="ok">Non</Badge>}</td>
                <td className="mono" style={{ fontSize: ".75rem" }}>{d.firstSeen}</td>
                <td className="mono" style={{ fontSize: ".75rem" }}>{d.lastSeen}</td>
                <td><Button variant="ghost" size="sm" onClick={() => setSelected(d.domain)}>Détail</Button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </TableWrap>
      {selectedDestination && <DestinationDetailPanel destination={selectedDestination} onNavigate={onNavigate} />}
    </div>
  );
}
