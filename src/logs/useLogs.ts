import { useCallback, useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { LogEntry } from "../shared/lib/types";

interface UseLogsResult {
  entries: LogEntry[];
  purge: () => Promise<void>;
}

export function useLogs(): UseLogsResult {
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

  const purge = useCallback(async () => {
    try {
      await vitrailApi.purgeLogs();
      setEntries([]);
    } catch (error) {
      logger.error({ error }, "Échec de purge du journal système");
    }
  }, []);

  return { entries, purge };
}
