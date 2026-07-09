import type { ReactElement } from "react";
import { AlertTriangle, Loader } from "lucide-react";
import { useKillSwitch } from "../hooks/useKillSwitchState";

export function DegradationBanner(): ReactElement | null {
  const { phase } = useKillSwitch();

  if (phase === "degraded") {
    return (
      <div id="degradation-banner">
        <AlertTriangle />
        <span>
          PolarProxy ne répond pas — le trafic est attribué mais non déchiffré. Vérifiez le
          panneau Kill Switch.
        </span>
      </div>
    );
  }

  if (phase === "transitioning") {
    return (
      <div id="degradation-banner">
        <Loader />
        <span>Activation en cours — séquence d'initialisation des sous-systèmes...</span>
      </div>
    );
  }

  return null;
}
