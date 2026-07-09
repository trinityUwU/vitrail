import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { SessionDetail } from "../shared/lib/types";

export function useSessionDetail(sessionId: string | null): { detail: SessionDetail | null } {
  const [detail, setDetail] = useState<SessionDetail | null>(null);

  useEffect(() => {
    if (!sessionId) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    vitrailApi
      .getSessionDetail(sessionId)
      .then((next) => {
        if (!cancelled) setDetail(next);
      })
      .catch((error) => logger.error({ error, sessionId }, "Échec de chargement du détail de session"));
    return () => {
      cancelled = true;
    };
  }, [sessionId]);

  return { detail };
}
