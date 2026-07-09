import type { ReactElement } from "react";
import { useState } from "react";
import { useSettings } from "./useSettings";
import { CaTab } from "./tabs/CaTab";
import { NetworkTab } from "./tabs/NetworkTab";
import { ExclusionsTab } from "./tabs/ExclusionsTab";
import { RetentionTab } from "./tabs/RetentionTab";
import { KeylogTab } from "./tabs/KeylogTab";
import { NotificationsTab } from "./tabs/NotificationsTab";
import { AboutTab } from "./tabs/AboutTab";
import type { ScreenId } from "../shared/lib/types";
import "./Settings.css";

const TABS = [
  { id: "ca", label: "CA & TLS" },
  { id: "network", label: "Réseau" },
  { id: "exclusions", label: "Exclusions" },
  { id: "retention", label: "Rétention" },
  { id: "keylog", label: "Keylog SSL" },
  { id: "notif", label: "Notifications" },
  { id: "about", label: "À propos" },
] as const;

type TabId = (typeof TABS)[number]["id"];

export function Settings({ onNavigate }: { onNavigate: (screen: ScreenId) => void }): ReactElement {
  const { settings, refresh } = useSettings();
  const [tab, setTab] = useState<TabId>("ca");

  return (
    <div>
      <div className="screen-title">Paramètres</div>
      <div className="screen-subtitle">Configuration de Vitrail</div>
      <div className="settings-tabs">
        {TABS.map((t) => (
          <button key={t.id} className={`settings-tab${tab === t.id ? " active" : ""}`} onClick={() => setTab(t.id)}>
            {t.label}
          </button>
        ))}
      </div>
      <div className="card">
        {!settings ? null : (
          <>
            {tab === "ca" && <CaTab settings={settings} onRotated={refresh} />}
            {tab === "network" && <NetworkTab settings={settings} />}
            {tab === "exclusions" && <ExclusionsTab />}
            {tab === "retention" && <RetentionTab settings={settings} />}
            {tab === "keylog" && <KeylogTab />}
            {tab === "notif" && <NotificationsTab settings={settings} />}
            {tab === "about" && <AboutTab onNavigate={onNavigate} />}
          </>
        )}
      </div>
    </div>
  );
}
