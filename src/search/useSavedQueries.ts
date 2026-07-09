import { useCallback, useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { SavedQuery, SearchCriteria } from "../shared/lib/types";

interface UseSavedQueriesResult {
  savedQueries: SavedQuery[];
  save: (name: string, criteria: SearchCriteria) => Promise<SavedQuery | null>;
  remove: (id: string) => Promise<void>;
}

export function useSavedQueries(): UseSavedQueriesResult {
  const [savedQueries, setSavedQueries] = useState<SavedQuery[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listSavedQueries()
      .then((next) => {
        if (!cancelled) setSavedQueries(next);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des requêtes sauvegardées"));
    return () => {
      cancelled = true;
    };
  }, []);

  const save = useCallback(async (name: string, criteria: SearchCriteria) => {
    try {
      const created = await vitrailApi.saveSearchQuery(name, criteria);
      setSavedQueries((prev) => [...prev, created]);
      return created;
    } catch (error) {
      logger.error({ error, name }, "Échec de sauvegarde de la requête");
      return null;
    }
  }, []);

  const remove = useCallback(async (id: string) => {
    try {
      await vitrailApi.deleteSavedQuery(id);
      setSavedQueries((prev) => prev.filter((q) => q.id !== id));
    } catch (error) {
      logger.error({ error, id }, "Échec de suppression de la requête sauvegardée");
    }
  }, []);

  return { savedQueries, save, remove };
}
