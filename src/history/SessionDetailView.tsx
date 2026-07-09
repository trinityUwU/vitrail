import type { ReactElement } from "react";
import { TimelineTable } from "../timeline/TimelineTable";
import type { TimelineFilterState } from "../timeline/TimelineFilters";
import type { SessionDetail } from "../shared/lib/types";

const EMPTY_FILTERS: TimelineFilterState = { search: "", visibility: "", process: "", port: "" };

export function SessionDetailView({ detail }: { detail: SessionDetail }): ReactElement {
  return (
    <div className="card" style={{ marginTop: 8 }}>
      <div className="section-title">Flux de la session {detail.session.id}</div>
      <TimelineTable flows={detail.flows} filters={EMPTY_FILTERS} onSelectFlow={() => undefined} />
    </div>
  );
}
