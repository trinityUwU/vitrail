import type { FlowVisibility, SearchCriteria } from "../shared/lib/types";
import type { SearchQuery } from "./useSearch";

function orNull(value: string): string | null {
  return value.trim() === "" ? null : value.trim();
}

export function queryToCriteria(query: SearchQuery): SearchCriteria {
  return {
    process: orNull(query.process),
    destination: orNull(query.destination),
    port: orNull(query.port),
    // Cast sûr : query.visibility n'est peuplé que via VISIBILITY_OPTIONS (shared/lib/visibility.ts),
    // qui énumère exactement les clés de FlowVisibility — jamais une valeur arbitraire.
    visibility: query.visibility === "" ? null : (query.visibility as FlowVisibility),
    from: null,
    to: null,
    text: orNull(query.text),
  };
}

export function criteriaToQuery(criteria: SearchCriteria): SearchQuery {
  return {
    process: criteria.process ?? "",
    destination: criteria.destination ?? "",
    port: criteria.port ?? "",
    visibility: criteria.visibility ?? "",
    text: criteria.text ?? "",
  };
}
