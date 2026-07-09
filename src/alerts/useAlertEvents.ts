import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { AlertEvent } from "../shared/lib/types";

export function useAlertEvents(ruleId: string): { events: AlertEvent[] } {
  const [events, setEvents] = useState<AlertEvent[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listAlertEvents(ruleId)
      .then((next) => {
        if (!cancelled) setEvents(next);
      })
      .catch((error) => logger.error({ error, ruleId }, "Échec de chargement des déclenchements"));
    return () => {
      cancelled = true;
    };
  }, [ruleId]);

  return { events };
}
