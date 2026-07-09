import { createContext, useCallback, useContext, useState } from "react";
import { vitrailApi } from "../lib/vitrail-api";
import { logger } from "../lib/logger";
import type { Exclusion } from "../lib/types";

// EPIC 4.5/9 exposeront une commande list_exclusions() pour charger l'état persistant —
// en attendant, la liste vit côté frontend et se synchronise via add/remove_exclusion.
const SEED_EXCLUSIONS: Exclusion[] = [
  { name: "Docker Desktop", type: "processus" },
  { name: "registry-1.docker.io", type: "domaine" },
  { name: "curl", type: "processus" },
];

interface ExclusionsContextValue {
  exclusions: Exclusion[];
  addExclusion: (name: string, type: string) => Promise<boolean>;
  removeExclusion: (name: string) => Promise<void>;
}

export const ExclusionsContext = createContext<ExclusionsContextValue | null>(null);

export function useExclusionsProviderState(): ExclusionsContextValue {
  const [exclusions, setExclusions] = useState<Exclusion[]>(SEED_EXCLUSIONS);

  const addExclusion = useCallback(async (name: string, type: string): Promise<boolean> => {
    try {
      const created = await vitrailApi.addExclusion(name, type);
      setExclusions((prev) => [...prev, created]);
      return true;
    } catch (error) {
      logger.error({ error, name }, "Échec d'ajout d'exclusion");
      return false;
    }
  }, []);

  const removeExclusion = useCallback(async (name: string) => {
    try {
      await vitrailApi.removeExclusion(name);
      setExclusions((prev) => prev.filter((e) => e.name !== name));
    } catch (error) {
      logger.error({ error, name }, "Échec de suppression d'exclusion");
    }
  }, []);

  return { exclusions, addExclusion, removeExclusion };
}

export function useExclusionsContext(): ExclusionsContextValue {
  const ctx = useContext(ExclusionsContext);
  if (!ctx) throw new Error("useExclusionsContext doit être utilisé sous ExclusionsProvider");
  return ctx;
}
