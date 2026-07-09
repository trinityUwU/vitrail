import type { ReactElement } from "react";
import { ArrowRight } from "lucide-react";
import { Button } from "../shared/components/Button";
import { VisibilityBadge } from "../shared/components/VisibilityBadge";
import { fmtVol } from "../shared/lib/format-utils";
import type { DestinationInfo, ProcessInfo, ScreenId } from "../shared/lib/types";

interface TopListsProps {
  processes: ProcessInfo[];
  destinations: DestinationInfo[];
  onNavigate: (screen: ScreenId) => void;
}

export function DashboardTopLists({ processes, destinations, onNavigate }: TopListsProps): ReactElement {
  const maxProcVol = processes[0]?.volumeMb || 1;
  const maxDestVol = destinations[0]?.volumeMb || 1;

  return (
    <div className="dash-top-row">
      <div className="card">
        <div className="section-title">Top processus par volume</div>
        <ul className="top-list">
          {processes.map((p, i) => (
            <li key={p.name}>
              <span className="top-list-rank">{i + 1}</span>
              <span className="top-list-name">{p.name}</span>
              <VisibilityBadge visibility={p.visibility} />
              <span className="top-list-vol">{fmtVol(p.volumeMb)}</span>
              <div className="top-list-bar">
                <div className="top-list-bar-fill" style={{ width: `${(p.volumeMb / maxProcVol) * 100}%` }} />
              </div>
            </li>
          ))}
        </ul>
        <div style={{ marginTop: 12 }}>
          <Button variant="ghost" size="sm" onClick={() => onNavigate("processes")}>
            Voir tout <ArrowRight />
          </Button>
        </div>
      </div>
      <div className="card">
        <div className="section-title">Top destinations par volume</div>
        <ul className="top-list">
          {destinations.map((d, i) => (
            <li key={d.domain}>
              <span className="top-list-rank">{i + 1}</span>
              <span className="top-list-name mono">{d.domain}</span>
              <VisibilityBadge visibility={d.visibility} />
              <span className="top-list-vol">{fmtVol(d.volumeMb)}</span>
              <div className="top-list-bar">
                <div className="top-list-bar-fill" style={{ width: `${(d.volumeMb / maxDestVol) * 100}%` }} />
              </div>
            </li>
          ))}
        </ul>
        <div style={{ marginTop: 12 }}>
          <Button variant="ghost" size="sm" onClick={() => onNavigate("destinations")}>
            Voir tout <ArrowRight />
          </Button>
        </div>
      </div>
    </div>
  );
}
