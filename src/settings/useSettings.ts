import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Settings } from "../shared/lib/types";

export function useSettings(): { settings: Settings | null; refresh: () => void } {
  const [settings, setSettings] = useState<Settings | null>(null);

  const load = (): void => {
    vitrailApi
      .getSettings()
      .then(setSettings)
      .catch((error) => logger.error({ error }, "Échec de chargement des paramètres"));
  };

  useEffect(load, []);

  return { settings, refresh: load };
}
