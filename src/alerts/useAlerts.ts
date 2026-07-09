import { useCallback, useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { AlertRule } from "../shared/lib/types";

export function useAlerts(): { rules: AlertRule[]; toggleRule: (id: string) => Promise<void> } {
  const [rules, setRules] = useState<AlertRule[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listAlertRules()
      .then((next) => {
        if (!cancelled) setRules(next);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des règles d'alerte"));
    return () => {
      cancelled = true;
    };
  }, []);

  const toggleRule = useCallback(async (id: string) => {
    try {
      await vitrailApi.toggleAlertRule(id);
      setRules((prev) => prev.map((r) => (r.id === id ? { ...r, active: !r.active } : r)));
    } catch (error) {
      logger.error({ error, id }, "Échec du basculement d'une règle d'alerte");
    }
  }, []);

  return { rules, toggleRule };
}
