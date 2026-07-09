import type { ReactElement } from "react";
import { Download, Pause, Play } from "lucide-react";
import { Button } from "../shared/components/Button";
import { VISIBILITY_OPTIONS } from "../shared/lib/visibility";
import type { FlowVisibility } from "../shared/lib/types";

export interface TimelineFilterState {
  search: string;
  visibility: FlowVisibility | "";
  process: string;
  port: string;
}

interface TimelineFiltersProps {
  filters: TimelineFilterState;
  onChange: (filters: TimelineFilterState) => void;
  processNames: string[];
  paused: boolean;
  onTogglePause: () => void;
  onExport: (format: "json" | "csv") => void;
}

export function TimelineFilters(props: TimelineFiltersProps): ReactElement {
  const { filters, onChange, processNames, paused, onTogglePause, onExport } = props;

  const update = (patch: Partial<TimelineFilterState>): void => onChange({ ...filters, ...patch });

  return (
    <div className="tl-filters">
      <input
        className="input"
        style={{ width: 200 }}
        placeholder="Rechercher..."
        value={filters.search}
        onChange={(e) => update({ search: e.target.value })}
      />
      <select
        className="input select"
        style={{ width: 150 }}
        value={filters.visibility}
        onChange={(e) => update({ visibility: e.target.value as FlowVisibility | "" })}
      >
        <option value="">Tous niveaux</option>
        {VISIBILITY_OPTIONS.map((v) => (
          <option key={v.key} value={v.key}>
            {v.label}
          </option>
        ))}
      </select>
      <select
        className="input select"
        style={{ width: 160 }}
        value={filters.process}
        onChange={(e) => update({ process: e.target.value })}
      >
        <option value="">Tous processus</option>
        {processNames.map((name) => (
          <option key={name}>{name}</option>
        ))}
      </select>
      <input
        className="input"
        style={{ width: 130 }}
        placeholder="Port..."
        value={filters.port}
        onChange={(e) => update({ port: e.target.value })}
      />
      <div className="tl-pause">
        <Button size="sm" onClick={onTogglePause}>
          {paused ? <Play /> : <Pause />} {paused ? "Reprendre" : "Pause"}
        </Button>
      </div>
      <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
        <Button size="sm" onClick={() => onExport("json")}>
          <Download /> JSON
        </Button>
        <Button size="sm" onClick={() => onExport("csv")}>
          <Download /> CSV
        </Button>
      </div>
    </div>
  );
}
