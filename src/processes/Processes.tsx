import type { ReactElement } from "react";
import { useState } from "react";
import { TableWrap } from "../shared/components/Table";
import { Badge } from "../shared/components/Badge";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { Button } from "../shared/components/Button";
import { fmtVol } from "../shared/lib/format-utils";
import { useProcesses } from "./useProcesses";
import { ProcessDetailPanel } from "./ProcessDetailPanel";
import type { ScreenId } from "../shared/lib/types";

export function Processes({ onNavigate }: { onNavigate: (screen: ScreenId) => void }): ReactElement {
  const { processes } = useProcesses();
  const [filter, setFilter] = useState("");
  const [selected, setSelected] = useState<string | null>(null);

  const filtered = processes.filter((p) => p.name.toLowerCase().includes(filter.toLowerCase()));
  const selectedProcess = processes.find((p) => p.name === selected) ?? null;

  return (
    <div>
      <div className="screen-title">Processus</div>
      <div className="screen-subtitle">Applications ayant généré du trafic réseau</div>
      <input
        className="input"
        style={{ width: 280, marginBottom: 20 }}
        placeholder="Filtrer par nom de processus..."
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />
      <TableWrap>
        <table>
          <thead>
            <tr>
              <th>Processus</th><th>Chemin</th><th>PIDs</th><th>Volume</th><th>Destinations</th>
              <th>Visibilité</th><th>Keylog</th><th></th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((p) => (
              <tr key={p.name}>
                <td style={{ fontWeight: 500, color: "var(--t1)" }}>{p.name}</td>
                <td className="mono" style={{ fontSize: ".73rem", maxWidth: 260, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p.path}>{p.path}</td>
                <td className="mono" style={{ fontSize: ".75rem" }}>{p.pids.join(", ")}</td>
                <td style={{ fontWeight: 500 }}>{fmtVol(p.volumeMb)}</td>
                <td>{p.destinations}</td>
                <td><VisibilityBadge visibility={p.visibility} /></td>
                <td>{p.keylogCovered ? <Badge variant="ok">Actif</Badge> : <Badge variant="attrib">Non</Badge>}</td>
                <td><Button variant="ghost" size="sm" onClick={() => setSelected(p.name)}>Détail</Button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </TableWrap>
      {selectedProcess && <ProcessDetailPanel process={selectedProcess} onNavigate={onNavigate} />}
    </div>
  );
}
