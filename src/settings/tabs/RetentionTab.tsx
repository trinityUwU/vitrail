import type { ChangeEvent, ReactElement } from "react";
import { useRef, useState } from "react";
import { Button } from "../../shared/components/Button";
import { useToast } from "../../shared/hooks/useToast";
import { vitrailApi } from "../../shared/lib/vitrail-api";
import { logger } from "../../shared/lib/logger";
import { downloadConfigJson, readConfigFile } from "../config-actions";
import type { Settings } from "../../shared/lib/types";

interface RetentionTabProps {
  settings: Settings;
  onSettingsChanged: () => void;
}

export function RetentionTab({ settings, onSettingsChanged }: RetentionTabProps): ReactElement {
  const { showToast } = useToast();
  const [purgeDate, setPurgeDate] = useState("");
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleExport = async (): Promise<void> => {
    try {
      const json = await vitrailApi.exportConfig();
      downloadConfigJson(json);
      showToast("Export lancé");
    } catch (error) {
      logger.error({ error }, "Échec d'export de la configuration");
    }
  };

  const handleImport = async (event: ChangeEvent<HTMLInputElement>): Promise<void> => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    try {
      const payload = await readConfigFile(file);
      await vitrailApi.importConfig(payload);
      onSettingsChanged();
      showToast("Configuration importée");
    } catch (error) {
      logger.error({ error }, "Échec d'import de la configuration");
    }
  };

  const handlePurge = async (before: string | null): Promise<void> => {
    try {
      const result = await vitrailApi.purgeData(before);
      showToast(`${result.deletedFlows} flux supprimés, ${result.freedMb.toFixed(1)} Mo libérés`);
      onSettingsChanged();
    } catch (error) {
      logger.error({ error, before }, "Échec de purge des données");
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
          <div className="setting-desc">Supprimer les données stockées avant une date, ou tout</div>
        </div>
        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input type="date" className="input" style={{ width: 150 }} value={purgeDate}
            onChange={(e) => setPurgeDate(e.target.value)} />
          <Button size="sm" onClick={() => void handlePurge(purgeDate || null)}>Purger par date</Button>
          <Button variant="danger" size="sm" onClick={() => void handlePurge(null)}>Purge totale</Button>
        </div>
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Exporter la configuration</div>
          <div className="setting-desc">Sauvegarder la config (pas les données)</div>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <Button size="sm" onClick={() => void handleExport()}>Exporter</Button>
          <Button size="sm" onClick={() => fileInputRef.current?.click()}>Importer</Button>
          <input ref={fileInputRef} type="file" accept="application/json" style={{ display: "none" }}
            onChange={(e) => void handleImport(e)} />
        </div>
      </div>
    </div>
  );
}
