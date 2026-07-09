import type { ReactElement } from "react";
import { Toggle } from "../../shared/components/Toggle";
import { logger } from "../../shared/lib/logger";
import { vitrailApi } from "../../shared/lib/vitrail-api";
import type { Settings } from "../../shared/lib/types";

interface NotificationsTabProps {
  settings: Settings;
  onSettingsChanged: () => void;
}

export function NotificationsTab({ settings, onSettingsChanged }: NotificationsTabProps): ReactElement {
  const handleUpdate = async (patch: Partial<Settings>): Promise<void> => {
    try {
      await vitrailApi.updateSettings({ ...settings, ...patch });
      onSettingsChanged();
    } catch (error) {
      logger.error({ error, patch }, "Échec de mise à jour des paramètres de notification");
    }
  };

  return (
    <div className="settings-section active">
      <div className="setting-row">
        <div>
          <div className="setting-label">Notifications desktop</div>
          <div className="setting-desc">Afficher des notifications système lors du déclenchement d'alertes</div>
        </div>
        <Toggle
          on={settings.notificationsEnabled}
          onToggle={() => void handleUpdate({ notificationsEnabled: !settings.notificationsEnabled })}
          label="Toggle notifications"
        />
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Son de notification</div>
          <div className="setting-desc">Jouer un son lors d'une alerte</div>
        </div>
        <Toggle
          on={settings.notificationSound}
          onToggle={() => void handleUpdate({ notificationSound: !settings.notificationSound })}
          label="Toggle son"
        />
      </div>
    </div>
  );
}
