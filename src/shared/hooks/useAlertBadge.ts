import { useEffect, useState } from "react";
import { vitrailApi } from "../lib/vitrail-api";
import { logger } from "../lib/logger";

export function useAlertBadge(): number {
  const [count, setCount] = useState(0);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listAlertRules()
      .then((rules) => {
        if (cancelled) return;
        const total = rules
          .filter((r) => r.active)
          .reduce((sum, r) => sum + r.triggerCount, 0);
        setCount(total);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des règles d'alerte"));
    return () => {
      cancelled = true;
    };
  }, []);

  return count;
}
