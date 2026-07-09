import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import type { Flow } from "../shared/lib/types";

interface TimelineState {
  flows: Flow[];
  paused: boolean;
  togglePause: () => void;
}

// Écoute l'événement "vitrail://flow" — émetteur factice temporaire côté Rust (EPIC 8.4),
// à remplacer par le vrai flux correlation::stream_flows() (EPIC 5.4).
export function useTimelineFlows(active: boolean): TimelineState {
  const [flows, setFlows] = useState<Flow[]>([]);
  const [paused, setPaused] = useState(false);
  const pausedRef = useRef(paused);
  pausedRef.current = paused;

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    vitrailApi
      .listFlows()
      .then((initial) => {
        if (!cancelled) setFlows(initial);
      })
      .catch((error) => logger.error({ error }, "Échec de chargement de la timeline"));

    const unlistenPromise = listen<Flow>("vitrail://flow", (event) => {
      if (pausedRef.current) return;
      setFlows((prev) => [event.payload, ...prev].slice(0, 200));
    });

    return () => {
      cancelled = true;
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, [active]);

  return { flows, paused, togglePause: () => setPaused((p) => !p) };
}
