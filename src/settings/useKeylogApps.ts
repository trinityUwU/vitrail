import { useCallback, useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";

interface UseKeylogAppsResult {
  apps: string[];
  addApp: (path: string) => Promise<void>;
  removeApp: (path: string) => Promise<void>;
}

export function useKeylogApps(): UseKeylogAppsResult {
  const [apps, setApps] = useState<string[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listKeylogApps()
      .then((next) => {
        if (!cancelled) setApps(next);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des applications keylog"));
    return () => {
      cancelled = true;
    };
  }, []);

  const addApp = useCallback(async (path: string) => {
    try {
      const next = await vitrailApi.addKeylogApp(path);
      setApps(next);
    } catch (error) {
      logger.error({ error, path }, "Échec d'ajout d'une application keylog");
    }
  }, []);

  const removeApp = useCallback(async (path: string) => {
    try {
      const next = await vitrailApi.removeKeylogApp(path);
      setApps(next);
    } catch (error) {
      logger.error({ error, path }, "Échec de suppression d'une application keylog");
    }
  }, []);

  return { apps, addApp, removeApp };
}
