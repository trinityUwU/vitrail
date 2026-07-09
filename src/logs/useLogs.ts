import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { LogEntry } from "../shared/lib/types";

export function useLogs(): { entries: LogEntry[] } {
  const [entries, setEntries] = useState<LogEntry[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .getLogEntries()
      .then((next) => {
        if (!cancelled) setEntries(next);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement du journal système"));
    return () => {
      cancelled = true;
    };
  }, []);

  return { entries };
}
