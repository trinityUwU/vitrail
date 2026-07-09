import type { ReactElement } from "react";
import { useState } from "react";
import { Toggle } from "../../shared/components/Toggle";
import type { Settings } from "../../shared/lib/types";

export function NotificationsTab({ settings }: { settings: Settings }): ReactElement {
  const [enabled, setEnabled] = useState(settings.notificationsEnabled);
  const [sound, setSound] = useState(settings.notificationSound);

  return (
    <div className="settings-section active">
      <div className="setting-row">
        <div>
          <div className="setting-label">Notifications desktop</div>
          <div className="setting-desc">Afficher des notifications système lors du déclenchement d'alertes</div>
        </div>
        <Toggle on={enabled} onToggle={() => setEnabled((v) => !v)} label="Toggle notifications" />
      </div>
      <div className="setting-row">
        <div>
          <div className="setting-label">Son de notification</div>
          <div className="setting-desc">Jouer un son lors d'une alerte</div>
        </div>
        <Toggle on={sound} onToggle={() => setSound((v) => !v)} label="Toggle son" />
      </div>
    </div>
  );
}
