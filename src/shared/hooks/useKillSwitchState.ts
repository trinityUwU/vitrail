import { createContext, useCallback, useContext, useState } from "react";
import { vitrailApi } from "../lib/vitrail-api";
import { logger } from "../lib/logger";
import type { SystemStatus } from "../lib/types";

export type KillSwitchPhase = "inactive" | "transitioning" | "active" | "degraded";

interface KillSwitchContextValue {
  phase: KillSwitchPhase;
  status: SystemStatus | null;
  activate: () => Promise<void>;
  deactivate: () => Promise<void>;
  emergencyStop: () => Promise<void>;
  refresh: () => Promise<void>;
}

export const KillSwitchContext = createContext<KillSwitchContextValue | null>(null);

// EPIC 7 remplacera la transition simulée par les événements Tauri réels du séquenceur
// d'activation/désactivation (chaque étape émise en direct, cf. EPIC 8.4).
const SIMULATED_TRANSITION_MS = 1200;

export function useKillSwitchProviderState(): KillSwitchContextValue {
  const [phase, setPhase] = useState<KillSwitchPhase>("inactive");
  const [status, setStatus] = useState<SystemStatus | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await vitrailApi.getSystemStatus();
      setStatus(next);
    } catch (error) {
      logger.error({ error }, "Échec de rafraîchissement du statut système");
    }
  }, []);

  const activate = useCallback(async () => {
    setPhase("transitioning");
    await new Promise((resolve) => setTimeout(resolve, SIMULATED_TRANSITION_MS));
    try {
      const next = await vitrailApi.activateVitrail();
      setStatus(next);
      setPhase("active");
    } catch (error) {
      logger.error({ error }, "Échec de l'activation de Vitrail");
      setPhase("degraded");
    }
  }, []);

  const deactivate = useCallback(async () => {
    try {
      const next = await vitrailApi.deactivateVitrail();
      setStatus(next);
      setPhase("inactive");
    } catch (error) {
      logger.error({ error }, "Échec de la désactivation de Vitrail");
    }
  }, []);

  const emergencyStop = useCallback(async () => {
    try {
      const next = await vitrailApi.emergencyStop();
      setStatus(next);
      setPhase("inactive");
    } catch (error) {
      logger.error({ error }, "Échec de l'arrêt d'urgence");
    }
  }, []);

  return { phase, status, activate, deactivate, emergencyStop, refresh };
}

export function useKillSwitch(): KillSwitchContextValue {
  const ctx = useContext(KillSwitchContext);
  if (!ctx) throw new Error("useKillSwitch doit être utilisé sous KillSwitchProvider");
  return ctx;
}
