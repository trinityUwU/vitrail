import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { DashboardSummary } from "../shared/lib/types";

interface DashboardState {
  summary: DashboardSummary | null;
  loading: boolean;
}

export function useDashboard(killSwitchActive: boolean): DashboardState {
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!killSwitchActive) {
      setSummary(null);
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    vitrailApi
      .getDashboardSummary()
      .then((next) => {
        if (!cancelled) setSummary(next);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement du dashboard"))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [killSwitchActive]);

  return { summary, loading };
}
