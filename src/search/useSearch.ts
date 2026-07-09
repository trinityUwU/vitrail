import { useCallback, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Flow } from "../shared/lib/types";

export interface SearchQuery {
  process: string;
  destination: string;
  port: string;
  visibility: string;
  text: string;
}

export const EMPTY_QUERY: SearchQuery = { process: "", destination: "", port: "", visibility: "", text: "" };

function hasAnyCriteria(query: SearchQuery): boolean {
  return Object.values(query).some((v) => v.trim().length > 0);
}

export function useSearch(): {
  results: Flow[] | null;
  run: (query: SearchQuery) => Promise<void>;
} {
  const [results, setResults] = useState<Flow[] | null>(null);

  const run = useCallback(async (query: SearchQuery) => {
    if (!hasAnyCriteria(query)) {
      setResults(null);
      return;
    }
    try {
      const combined = [query.process, query.destination, query.text].filter(Boolean).join(" ");
      const next = await vitrailApi.searchFlows(combined);
      setResults(next);
    } catch (error) {
      logger.error({ error, query }, "Échec de la recherche avancée");
    }
  }, []);

  return { results, run };
}
