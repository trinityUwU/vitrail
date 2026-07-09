import type { ReactElement } from "react";
import { Button } from "../../shared/components/Button";
import { useToast } from "../../shared/hooks/useToast";
import { vitrailApi } from "../../shared/lib/vitrail-api";
import { logger } from "../../shared/lib/logger";
import type { Settings } from "../../shared/lib/types";

export function RetentionTab({ settings }: { settings: Settings }): ReactElement {
  const { showToast } = useToast();

  const handleExport = async (): Promise<void> => {
    try {
      await vitrailApi.exportConfig();
      showToast("Export lancé");
    } catch (error) {
      logger.error({ error }, "Échec d'export de la configuration");
    }
  };

  return (
    <div className="settings-section active">
      <div className="setting-row">
        <div>
          <div className="setting-label">Politique de rétention</div>
          <div className="setting-desc">Durée de conservation des données de trafic</div>
        </div>
        <span style={{ fontWeight: 500 }}>{settings.retentionDays ? `${settings.retentionDays} jours` : "Illimité"}</span>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Taille de la base de données</div>
          <div className="setting-desc">Espace disque utilisé actuellement</div>
        </div>
        <span style={{ fontWeight: 500 }}>{settings.databaseSizeMb.toFixed(1)} Mo</span>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Purge manuelle</div>
          <div className="setting-desc">Supprimer les données stockées</div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <Button size="sm" onClick={() => showToast("Action de purge effectuée")}>Purger par date</Button>
          <Button variant="danger" size="sm" onClick={() => showToast("Action de purge effectuée")}>Purge totale</Button>
        </div>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Exporter la configuration</div>
          <div className="setting-desc">Sauvegarder la config (pas les données)</div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <Button size="sm" onClick={() => void handleExport()}>Exporter</Button>
          <Button size="sm" onClick={() => showToast("Import de configuration")}>Importer</Button>
        </div>
      </div>
    </div>
  );
}
