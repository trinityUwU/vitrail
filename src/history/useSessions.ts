import { useCallback, useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Session } from "../shared/lib/types";

interface UseSessionsResult {
  sessions: Session[];
  deleteSession: (id: string) => Promise<void>;
}

export function useSessions(): UseSessionsResult {
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

  const deleteSession = useCallback(async (id: string) => {
    try {
      await vitrailApi.deleteSession(id);
      setSessions((prev) => prev.filter((s) => s.id !== id));
    } catch (error) {
      logger.error({ error, id }, "Échec de suppression de la session");
    }
  }, []);

  return { sessions, deleteSession };
}
