import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Flow } from "../shared/lib/types";

export function useFlowInspector(flowId: string | null): { flow: Flow | null; loading: boolean } {
  const [flow, setFlow] = useState<Flow | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!flowId) {
      setFlow(null);
      return;
    }
    let cancelled = false;
    setLoading(true);
    vitrailApi
      .getFlowDetail(flowId)
      .then((next) => {
        if (!cancelled) setFlow(next);
      })
      .catch((error) => logger.error({ error, flowId }, "Échec de chargement du flux"))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [flowId]);

  return { flow, loading };
}
