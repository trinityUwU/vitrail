import type { ReactElement } from "react";
import { TableWrap } from "../shared/components/Table";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { fmtDur, fmtSize } from "../shared/lib/format-utils";
import type { Flow } from "../shared/lib/types";
import type { TimelineFilterState } from "./TimelineFilters";

function matchesFilters(flow: Flow, filters: TimelineFilterState): boolean {
  const haystack = `${flow.process} ${flow.destination}`.toLowerCase();
  if (filters.search && !haystack.includes(filters.search.toLowerCase())) return false;
  if (filters.visibility && flow.visibility !== filters.visibility) return false;
  if (filters.process && flow.process !== filters.process) return false;
  if (filters.port && !String(flow.port).includes(filters.port)) return false;
  return true;
}

interface TimelineTableProps {
  flows: Flow[];
  filters: TimelineFilterState;
  onSelectFlow: (id: string) => void;
}

export function TimelineTable({ flows, filters, onSelectFlow }: TimelineTableProps): ReactElement {
  const filtered = flows.filter((f) => matchesFilters(f, filters));

  return (
    <TableWrap>
      <table>
        <thead>
          <tr>
            <th>Heure</th>
            <th>Processus</th>
            <th>Destination</th>
            <th>Port</th>
            <th>Protocole</th>
            <th>Taille</th>
            <th>Visibilité</th>
            <th>Durée</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((f) => (
            <tr key={f.id} style={{ cursor: "pointer" }} onClick={() => onSelectFlow(f.id)}>
              <td className="mono" style={{ whiteSpace: "nowrap" }}>{f.timestamp}</td>
              <td style={{ fontWeight: 500, color: "var(--t1)" }}>{f.process}</td>
              <td className="mono">{f.destination}</td>
              <td className="mono">{f.port}</td>
              <td style={{ fontSize: ".75rem" }}>{f.protocol}</td>
              <td className="mono">{fmtSize(f.sizeBytes)}</td>
              <td><VisibilityBadge visibility={f.visibility} /></td>
              <td className="mono" style={{ fontSize: ".75rem" }}>{fmtDur(f.durationMs)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </TableWrap>
  );
}
