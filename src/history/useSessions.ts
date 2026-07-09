import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Session } from "../shared/lib/types";

export function useSessions(): { sessions: Session[] } {
  const [sessions, setSessions] = useState<Session[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listSessions()
      .then((next) => {
        if (!cancelled) setSessions(next);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des sessions"));
    return () => {
      cancelled = true;
    };
  }, []);

  return { sessions };
}
