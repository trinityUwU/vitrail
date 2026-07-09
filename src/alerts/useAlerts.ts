import { useCallback, useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { AlertRule } from "../shared/lib/types";

interface UseAlertsResult {
  rules: AlertRule[];
  toggleRule: (id: string) => Promise<void>;
  createRule: (name: string, description: string, criteria: string) => Promise<void>;
  updateRule: (id: string, name: string, description: string, criteria: string) => Promise<void>;
  deleteRule: (id: string) => Promise<void>;
}

export function useAlerts(): UseAlertsResult {
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

  const createRule = useCallback(async (name: string, description: string, criteria: string) => {
    try {
      const created = await vitrailApi.createAlertRule(name, description, criteria);
      setRules((prev) => [...prev, created]);
    } catch (error) {
      logger.error({ error, name }, "Échec de création d'une règle d'alerte");
    }
  }, []);

  const updateRule = useCallback(
    async (id: string, name: string, description: string, criteria: string) => {
      try {
        const updated = await vitrailApi.updateAlertRule(id, name, description, criteria);
        setRules((prev) => prev.map((r) => (r.id === id ? updated : r)));
      } catch (error) {
        logger.error({ error, id }, "Échec de mise à jour d'une règle d'alerte");
      }
    },
    [],
  );

  const deleteRule = useCallback(async (id: string) => {
    try {
      await vitrailApi.deleteAlertRule(id);
      setRules((prev) => prev.filter((r) => r.id !== id));
    } catch (error) {
      logger.error({ error, id }, "Échec de suppression d'une règle d'alerte");
    }
  }, []);

  return { rules, toggleRule, createRule, updateRule, deleteRule };
}
