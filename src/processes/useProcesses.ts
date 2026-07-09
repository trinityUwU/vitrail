import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { ProcessInfo } from "../shared/lib/types";

export function useProcesses(): { processes: ProcessInfo[]; loading: boolean } {
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listProcesses()
      .then((next) => {
        if (!cancelled) setProcesses([...next].sort((a, b) => b.volumeMb - a.volumeMb));
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des processus"))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { processes, loading };
}
