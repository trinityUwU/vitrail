import type { ReactElement } from "react";
import { useMemo, useState } from "react";
import { Power } from "lucide-react";
import { useKillSwitch } from "../shared/hooks/useKillSwitchState";
import { useTimelineFlows } from "./useTimelineFlows";
import { TimelineFilters, type TimelineFilterState } from "./TimelineFilters";
import { TimelineTable } from "./TimelineTable";
import { EmptyState } from "../shared/components/EmptyState";
import { useToast } from "../shared/hooks/useToast";
import type { ScreenId } from "../shared/lib/types";
import "./Timeline.css";

const EMPTY_FILTERS: TimelineFilterState = { search: "", visibility: "", process: "", port: "" };

interface TimelineProps {
  onNavigate: (screen: ScreenId) => void;
  onSelectFlow: (id: string) => void;
}

export function Timeline({ onSelectFlow }: TimelineProps): ReactElement {
  const { phase } = useKillSwitch();
  const active = phase === "active" || phase === "degraded";
  const { flows, paused, togglePause } = useTimelineFlows(active);
  const [filters, setFilters] = useState<TimelineFilterState>(EMPTY_FILTERS);
  const { showToast } = useToast();

  const processNames = useMemo(() => Array.from(new Set(flows.map((f) => f.process))), [flows]);

  if (!active) {
    return (
      <div>
        <div className="screen-title">Timeline</div>
        <div className="screen-subtitle">Flux réseau en temps réel</div>
        <EmptyState icon={Power} message="Activez Vitrail pour voir du trafic" />
      </div>
    );
  }

  return (
    <div>
      <div className="screen-title">Timeline</div>
      <div className="screen-subtitle">Flux réseau en temps réel</div>
      <TimelineFilters
        filters={filters}
        onChange={setFilters}
        processNames={processNames}
        paused={paused}
        onTogglePause={togglePause}
        onExport={(format) => showToast(`Export ${format.toUpperCase()} lancé`)}
      />
      <div style={{ maxHeight: "calc(100vh - 240px)", overflowY: "auto" }}>
        <TimelineTable flows={flows} filters={filters} onSelectFlow={onSelectFlow} />
      </div>
    </div>
  );
}
