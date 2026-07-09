import { useCallback, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Exclusion } from "../shared/lib/types";

// EPIC 4.5/9 exposeront une commande list_exclusions() pour charger l'état persistant —
// en attendant, la liste vit côté frontend et se synchronise via add/remove_exclusion.
const SEED_EXCLUSIONS: Exclusion[] = [
  { name: "Docker Desktop", type: "processus" },
  { name: "registry-1.docker.io", type: "domaine" },
  { name: "curl", type: "processus" },
];

export function useExclusions(): {
  exclusions: Exclusion[];
  addExclusion: (name: string, type: string) => Promise<void>;
  removeExclusion: (name: string) => Promise<void>;
} {
  const [exclusions, setExclusions] = useState<Exclusion[]>(SEED_EXCLUSIONS);

  const addExclusion = useCallback(async (name: string, type: string) => {
    try {
      const created = await vitrailApi.addExclusion(name, type);
      setExclusions((prev) => [...prev, created]);
    } catch (error) {
      logger.error({ error, name }, "Échec d'ajout d'exclusion");
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
