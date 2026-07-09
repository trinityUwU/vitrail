import type { ReactElement } from "react";
import { useKillSwitch } from "../shared/hooks/useKillSwitchState";
import { Toggle } from "../shared/components/Toggle";
import { useToast } from "../shared/hooks/useToast";
import { vitrailApi } from "../shared/lib/vitrail-api";
import { logger } from "../shared/lib/logger";
import { SUBSYSTEM_ICONS } from "./subsystem-icons";
import { EmergencyStop } from "./EmergencyStop";
import { KillSwitchLog } from "./KillSwitchLog";
import "./KillSwitch.css";

export function KillSwitch(): ReactElement {
  const { phase, status, activate, deactivate, emergencyStop } = useKillSwitch();
  const { showToast } = useToast();
  const isActive = phase === "active" || phase === "degraded";

  const handleVerify = async (): Promise<void> => {
    try {
      const report = await vitrailApi.verifyTeardown();
      showToast(report.clean ? "Vérification conforme — aucune divergence" : "Divergences détectées");
    } catch (error) {
      logger.error({ error }, "Échec de la vérification post-désactivation");
    }
  };

  return (
    <div>
      <div className="screen-title">Kill Switch</div>
      <div className="screen-subtitle">Garantie centrale d'activation et de désactivation complète</div>

      <div className={`ks-main-toggle ${isActive ? "active" : "inactive"}`}>
        <div>
          <div className="ks-toggle-label">{isActive ? "Vitrail est actif" : "Vitrail est inactif"}</div>
          <div className="ks-toggle-sub">
            {isActive ? "Sous-systèmes en cours d'exécution" : "Aucun sous-système n'est en cours d'exécution"}
          </div>
        </div>
        <Toggle
          on={isActive}
          size="lg"
          label="Toggle kill switch"
          onToggle={() => (isActive ? void deactivate() : void activate())}
        />
      </div>

      <div className="section-title" style={{ marginBottom: 8 }}>État des sous-systèmes</div>
      {(status?.subsystems ?? []).map((sub) => {
        const Icon = SUBSYSTEM_ICONS[sub.id] ?? SUBSYSTEM_ICONS.nftables;
        return (
          <div className="ks-subsystem" key={sub.id}>
            <div className="ks-sub-icon"><Icon /></div>
            <div className="ks-sub-info">
              <div className="ks-sub-name">{sub.name}</div>
              <div className="ks-sub-detail">{sub.detail}</div>
            </div>
            <span className={`ks-sub-status ${sub.status}`}>
              {sub.status === "ok" ? "Actif" : sub.status === "err" ? "Erreur" : sub.status === "wait" ? "En cours" : "Inactif"}
            </span>
          </div>
        );
      })}

      <EmergencyStop onConfirm={() => void emergencyStop()} />
      <KillSwitchLog onVerify={() => void handleVerify()} />
    </div>
  );
}
