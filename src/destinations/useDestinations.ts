import { useEffect, useState } from "react";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { DestinationInfo } from "../shared/lib/types";

export function useDestinations(): { destinations: DestinationInfo[] } {
  const [destinations, setDestinations] = useState<DestinationInfo[]>([]);

  useEffect(() => {
    let cancelled = false;
    vitrailApi
      .listDestinations()
      .then((next) => {
        if (!cancelled) setDestinations([...next].sort((a, b) => b.volumeMb - a.volumeMb));
      })
      .catch((error) => logger.error({ error }, "Échec de chargement des destinations"));
    return () => {
      cancelled = true;
    };
  }, []);

  return { destinations };
}
